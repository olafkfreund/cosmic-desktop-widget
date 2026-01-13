// Widget implementations - Clock and Weather

use chrono::Local;
use serde::{Deserialize, Serialize};

/// Clock widget displaying current time
pub struct ClockWidget {
    current_time: String,
    last_update: std::time::Instant,
}

impl ClockWidget {
    pub fn new() -> Self {
        Self {
            current_time: Self::format_time(),
            last_update: std::time::Instant::now(),
        }
    }

    pub fn update(&mut self) {
        // Update every second
        if self.last_update.elapsed().as_secs() >= 1 {
            self.current_time = Self::format_time();
            self.last_update = std::time::Instant::now();
        }
    }

    pub fn time_string(&self) -> String {
        self.current_time.clone()
    }

    fn format_time() -> String {
        let now = Local::now();
        now.format("%H:%M:%S").to_string()
    }

    pub fn date_string() -> String {
        let now = Local::now();
        now.format("%A, %B %d, %Y").to_string()
    }
}

impl Default for ClockWidget {
    fn default() -> Self {
        Self::new()
    }
}

/// Weather widget displaying current weather conditions
pub struct WeatherWidget {
    city: String,
    api_key: String,
    data: Option<WeatherData>,
    last_update: std::time::Instant,
    update_interval: std::time::Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherData {
    pub temperature: f32,
    pub condition: String,
    pub humidity: u32,
    pub wind_speed: f32,
}

impl WeatherWidget {
    pub fn new(city: &str, api_key: &str) -> Self {
        Self {
            city: city.to_string(),
            api_key: api_key.to_string(),
            data: None,
            last_update: std::time::Instant::now(),
            update_interval: std::time::Duration::from_secs(600), // 10 minutes
        }
    }

    pub fn update(&mut self) {
        // Check if we need to fetch new data
        if self.data.is_none() || self.last_update.elapsed() >= self.update_interval {
            // Spawn async task to fetch weather
            // In a real implementation, use tokio or async-std
            // For now, just use placeholder data
            self.data = Some(WeatherData {
                temperature: 22.0,
                condition: "Sunny".to_string(),
                humidity: 65,
                wind_speed: 5.2,
            });
            self.last_update = std::time::Instant::now();
        }
    }

    pub fn display_string(&self) -> Option<String> {
        self.data.as_ref().map(|data| {
            format!(
                "{}°C {} | {}% humidity",
                data.temperature.round(),
                data.condition,
                data.humidity
            )
        })
    }

    pub async fn fetch_weather(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units=metric",
            self.city, self.api_key
        );

        let response = reqwest::get(&url).await?;
        let json: serde_json::Value = response.json().await?;

        self.data = Some(WeatherData {
            temperature: json["main"]["temp"].as_f64().unwrap_or(0.0) as f32,
            condition: json["weather"][0]["main"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string(),
            humidity: json["main"]["humidity"].as_u64().unwrap_or(0) as u32,
            wind_speed: json["wind"]["speed"].as_f64().unwrap_or(0.0) as f32,
        });

        self.last_update = std::time::Instant::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_widget() {
        let clock = ClockWidget::new();
        assert!(!clock.time_string().is_empty());
    }

    #[test]
    fn test_weather_widget() {
        let weather = WeatherWidget::new("London", "test_key");
        assert_eq!(weather.city, "London");
    }
}
