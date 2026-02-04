// Weather service with async API integration
//
// Uses a worker thread pattern with calloop channel to keep async I/O
// off the main event loop, preventing blocking.

use calloop::channel::{sync_channel, Channel, SyncSender};
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::error::WeatherError;
use crate::widget::WeatherData;

/// Result type for weather operations
pub type WeatherResult = Result<WeatherData, WeatherError>;

/// Weather service that manages async API fetching in a background thread
pub struct WeatherService {
    sender: SyncSender<WeatherResult>,
    pub channel: Channel<WeatherResult>,
}

impl WeatherService {
    /// Create a new weather service with channel for result communication
    pub fn new() -> Self {
        let (sender, channel) = sync_channel(1);
        Self { sender, channel }
    }

    /// Start fetching weather data in a background thread
    ///
    /// This spawns a dedicated thread that runs a tokio runtime for async HTTP requests.
    /// Results are sent back to the main event loop via the calloop channel.
    pub fn start_fetching(&self, city: String, api_key: String, interval: Duration) {
        let sender = self.sender.clone();

        info!(
            city = %city,
            interval_secs = interval.as_secs(),
            "Starting weather fetching thread"
        );

        thread::spawn(move || {
            // Create tokio runtime in this thread
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(e) => {
                    error!(error = %e, "Failed to create tokio runtime for weather fetching");
                    let _ = sender.send(Err(WeatherError::InvalidResponse(
                        format!("Failed to create async runtime: {}", e)
                    )));
                    return;
                }
            };

            // Fetch immediately on start
            let result = rt.block_on(fetch_weather_data(&city, &api_key));
            if let Err(ref e) = result {
                warn!(error = %e, "Initial weather fetch failed");
            }
            let _ = sender.send(result);

            // Then fetch periodically
            loop {
                thread::sleep(interval);

                debug!(city = %city, "Fetching weather update");
                let result = rt.block_on(fetch_weather_data(&city, &api_key));

                if let Err(ref e) = result {
                    warn!(error = %e, city = %city, "Weather fetch failed");
                }

                // Send result through channel (non-blocking)
                if sender.send(result).is_err() {
                    error!("Weather channel disconnected, stopping fetch thread");
                    break;
                }
            }
        });
    }
}

impl Default for WeatherService {
    fn default() -> Self {
        Self::new()
    }
}

/// Fetch weather data from OpenWeatherMap API with retry logic
async fn fetch_weather_data(city: &str, api_key: &str) -> WeatherResult {
    if api_key.is_empty() {
        warn!("Weather API key not configured");
        return Err(WeatherError::NoApiKey);
    }

    // Retry logic: try up to 3 times with exponential backoff
    let mut attempts = 0;
    let max_attempts = 3;
    let mut backoff = Duration::from_secs(1);

    loop {
        attempts += 1;

        match fetch_weather_attempt(city, api_key).await {
            Ok(data) => {
                info!(
                    city = %city,
                    temp = %data.temperature,
                    condition = %data.condition,
                    "Weather fetch successful"
                );
                return Ok(data);
            }
            Err(e) => {
                if attempts >= max_attempts {
                    error!(
                        error = %e,
                        city = %city,
                        attempts = attempts,
                        "Weather fetch failed after all retries"
                    );
                    return Err(e);
                }

                warn!(
                    error = %e,
                    city = %city,
                    attempt = attempts,
                    retry_in_secs = backoff.as_secs(),
                    "Weather fetch failed, retrying"
                );

                tokio::time::sleep(backoff).await;
                backoff *= 2; // Exponential backoff
            }
        }
    }
}

/// Single attempt to fetch weather data from API
async fn fetch_weather_attempt(city: &str, api_key: &str) -> WeatherResult {
    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units=metric",
        city, api_key
    );

    debug!(city = %city, "Sending weather API request");

    let response = reqwest::get(&url).await.map_err(|e| {
        debug!(error = %e, "HTTP request failed");
        e
    })?;

    if !response.status().is_success() {
        let status = response.status();
        warn!(status = %status, city = %city, "API returned error status");

        // Check if it's a 404 (city not found)
        if status == 404 {
            return Err(WeatherError::CityNotFound(city.to_string()));
        }

        return Err(WeatherError::InvalidResponse(format!(
            "HTTP {}",
            status
        )));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| {
        warn!(error = %e, "Failed to parse JSON response");
        e
    })?;

    // Check for API error response (even with 200 status)
    if let Some(cod) = json.get("cod") {
        if cod != 200 && cod != "200" {
            let msg = json
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            warn!(code = ?cod, message = %msg, "API returned error in body");
            return Err(WeatherError::InvalidResponse(msg.to_string()));
        }
    }

    // Parse weather data with detailed error messages
    let temperature = json["main"]["temp"]
        .as_f64()
        .ok_or_else(|| {
            WeatherError::ParseError("missing or invalid temperature field".to_string())
        })? as f32;

    let condition = json["weather"][0]["main"]
        .as_str()
        .ok_or_else(|| {
            WeatherError::ParseError("missing or invalid condition field".to_string())
        })?
        .to_string();

    let humidity = json["main"]["humidity"]
        .as_u64()
        .ok_or_else(|| {
            WeatherError::ParseError("missing or invalid humidity field".to_string())
        })? as u32;

    let wind_speed = json["wind"]["speed"]
        .as_f64()
        .ok_or_else(|| {
            WeatherError::ParseError("missing or invalid wind_speed field".to_string())
        })? as f32;

    debug!(
        temp = %temperature,
        condition = %condition,
        humidity = %humidity,
        wind = %wind_speed,
        "Weather data parsed successfully"
    );

    Ok(WeatherData {
        temperature,
        condition,
        humidity,
        wind_speed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_service_creation() {
        let service = WeatherService::new();
        // Should create without panicking
        drop(service);
    }

    #[test]
    fn test_empty_api_key() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(fetch_weather_data("London", ""));
        assert!(matches!(result, Err(WeatherError::NoApiKey)));
    }
}
