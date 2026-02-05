//! Crypto widget displaying cryptocurrency prices
//!
//! This widget shows cryptocurrency prices from CoinGecko API (free, no API key required).
//! Supports multiple cryptocurrencies with configurable update intervals.

use std::time::{Duration, Instant};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::registry::DynWidgetFactory;
use super::traits::{FontSize, Widget, WidgetContent, WidgetInfo};

/// CoinGecko API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoinGeckoResponse {
    #[serde(flatten)]
    coins: std::collections::HashMap<String, CoinData>,
}

/// Data for a single cryptocurrency
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoinData {
    #[serde(default)]
    usd: Option<f64>,
    #[serde(default)]
    eur: Option<f64>,
    #[serde(default)]
    usd_24h_change: Option<f64>,
    #[serde(default)]
    eur_24h_change: Option<f64>,
}

/// Cryptocurrency price data
#[derive(Debug, Clone)]
pub struct CryptoPrice {
    pub symbol: String,
    pub price: f64,
    pub change_24h: Option<f64>,
}

impl CryptoPrice {
    /// Format price for display
    pub fn display(&self, show_change: bool) -> String {
        let price_str = if self.price >= 1000.0 {
            format!("${:.0}", self.price)
        } else if self.price >= 1.0 {
            format!("${:.2}", self.price)
        } else {
            format!("${:.4}", self.price)
        };

        if show_change {
            match self.change_24h {
                Some(change) if change >= 0.0 => {
                    format!("{}: {} (+{:.2}%)", self.symbol, price_str, change)
                }
                Some(change) => {
                    format!("{}: {} ({:.2}%)", self.symbol, price_str, change)
                }
                None => format!("{}: {}", self.symbol, price_str),
            }
        } else {
            format!("{}: {}", self.symbol, price_str)
        }
    }
}

/// Crypto widget showing cryptocurrency prices
pub struct CryptoWidget {
    coins: Vec<String>,
    currency: String,
    show_change: bool,
    data: Option<Vec<CryptoPrice>>,
    last_update: Instant,
    update_interval: Duration,
    error_message: Option<String>,
}

impl CryptoWidget {
    /// Create a new Crypto widget
    pub fn new(
        coins: Vec<String>,
        currency: &str,
        show_change: bool,
        update_interval: u64,
    ) -> Self {
        Self {
            coins,
            currency: currency.to_string(),
            show_change,
            data: None,
            last_update: Instant::now(),
            update_interval: Duration::from_secs(update_interval),
            error_message: None,
        }
    }

    /// Set crypto data from successful API fetch
    pub fn set_data(&mut self, data: Vec<CryptoPrice>) {
        debug!(
            count = data.len(),
            currency = %self.currency,
            "Crypto data updated"
        );
        self.data = Some(data);
        self.last_update = Instant::now();
        self.error_message = None;
    }

    /// Set error message from failed API fetch
    pub fn set_error(&mut self, error: String) {
        warn!(error = %error, "Crypto fetch error");
        self.error_message = Some(error);
    }

    /// Get display string for all cryptocurrencies
    pub fn display_string(&self) -> Option<String> {
        // If there's an error and no data, show error
        if self.data.is_none() && self.error_message.is_some() {
            return self.error_message.as_ref().map(|e| format!("Error: {}", e));
        }

        self.data.as_ref().map(|prices| {
            // Check if data is stale (older than 2x update interval)
            let stale_threshold = self.update_interval * 2;
            let is_stale = self.last_update.elapsed() > stale_threshold;

            let mut lines: Vec<String> = prices
                .iter()
                .map(|price| price.display(self.show_change))
                .collect();

            // Add indicators
            if is_stale {
                lines.push("(stale)".to_string());
            }
            if self.error_message.is_some() {
                lines.push("⚠".to_string());
            }

            lines.join(" | ")
        })
    }

    /// Fetch cryptocurrency prices from CoinGecko API
    pub async fn fetch_prices(&mut self) -> anyhow::Result<()> {
        if self.coins.is_empty() {
            return Err(anyhow::anyhow!("No coins configured"));
        }

        info!(
            coins = ?self.coins,
            currency = %self.currency,
            "Fetching crypto prices from CoinGecko API"
        );

        let coins_param = self.coins.join(",");
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies={}&include_24hr_change=true",
            coins_param, self.currency
        );

        let response = reqwest::get(&url).await.map_err(|e| {
            warn!(error = %e, coins = ?self.coins, "Failed to fetch crypto prices from API");
            e
        })?;

        if !response.status().is_success() {
            let status = response.status();
            warn!(status = %status, "CoinGecko API returned error status");
            return Err(anyhow::anyhow!("API returned status: {}", status));
        }

        let response_data: CoinGeckoResponse = response.json().await.map_err(|e| {
            warn!(error = %e, "Failed to parse CoinGecko API response");
            e
        })?;

        let mut prices = Vec::new();

        for coin_id in &self.coins {
            if let Some(coin_data) = response_data.coins.get(coin_id) {
                let (price, change) = match self.currency.as_str() {
                    "eur" => (coin_data.eur, coin_data.eur_24h_change),
                    _ => (coin_data.usd, coin_data.usd_24h_change),
                };

                if let Some(price_value) = price {
                    prices.push(CryptoPrice {
                        symbol: coin_id.to_uppercase(),
                        price: price_value,
                        change_24h: change,
                    });
                } else {
                    warn!(coin = %coin_id, "No price data available for coin");
                }
            } else {
                warn!(coin = %coin_id, "Coin not found in API response");
            }
        }

        if prices.is_empty() {
            return Err(anyhow::anyhow!("No valid price data received"));
        }

        self.data = Some(prices.clone());
        self.last_update = Instant::now();

        info!(count = prices.len(), "Crypto API fetch successful");

        Ok(())
    }

    /// Get the list of configured coins
    pub fn coins(&self) -> &[String] {
        &self.coins
    }

    /// Get the configured currency
    pub fn currency(&self) -> &str {
        &self.currency
    }
}

impl Widget for CryptoWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "crypto",
            name: "Crypto",
            preferred_height: 40.0,
            min_height: 30.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        // Update is handled by background thread
        // This method is a no-op for async widgets
    }

    fn content(&self) -> WidgetContent {
        match self.display_string() {
            Some(text) => WidgetContent::Text {
                text,
                size: FontSize::Medium,
            },
            None => WidgetContent::Empty,
        }
    }

    fn update_interval(&self) -> Duration {
        self.update_interval
    }

    fn is_ready(&self) -> bool {
        self.data.is_some() || self.error_message.is_some()
    }

    fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

impl Default for CryptoWidget {
    fn default() -> Self {
        Self::new(
            vec!["bitcoin".to_string(), "ethereum".to_string()],
            "usd",
            true,
            120, // 2 minutes
        )
    }
}

// ============================================================================
// Factory
// ============================================================================

/// Factory for CryptoWidget
pub struct CryptoWidgetFactory;

impl DynWidgetFactory for CryptoWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "crypto"
    }

    fn create(&self, config: &toml::Table) -> anyhow::Result<Box<dyn Widget>> {
        // Parse coins array
        let coins = if let Some(coins_value) = config.get("coins") {
            if let Some(coins_array) = coins_value.as_array() {
                coins_array
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            } else {
                vec!["bitcoin".to_string(), "ethereum".to_string()]
            }
        } else {
            vec!["bitcoin".to_string(), "ethereum".to_string()]
        };

        if coins.is_empty() {
            anyhow::bail!("At least one cryptocurrency must be configured");
        }

        let currency = config
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("usd");

        let show_change = config
            .get("show_change")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let update_interval = config
            .get("update_interval")
            .and_then(|v| v.as_integer())
            .unwrap_or(120) as u64;

        debug!(
            coins = ?coins,
            currency = %currency,
            show_change = %show_change,
            update_interval = %update_interval,
            "Creating CryptoWidget"
        );

        Ok(Box::new(CryptoWidget::new(
            coins,
            currency,
            show_change,
            update_interval,
        )))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();

        // Create coins array
        let coins = vec![
            toml::Value::String("bitcoin".to_string()),
            toml::Value::String("ethereum".to_string()),
        ];
        config.insert("coins".to_string(), toml::Value::Array(coins));
        config.insert(
            "currency".to_string(),
            toml::Value::String("usd".to_string()),
        );
        config.insert("show_change".to_string(), toml::Value::Boolean(true));
        config.insert("update_interval".to_string(), toml::Value::Integer(120));

        config
    }

    fn validate_config(&self, config: &toml::Table) -> anyhow::Result<()> {
        // Validate currency
        if let Some(currency) = config.get("currency") {
            let currency_str = currency.as_str().context("'currency' must be a string")?;

            if currency_str != "usd" && currency_str != "eur" {
                anyhow::bail!("'currency' must be 'usd' or 'eur', got '{}'", currency_str);
            }
        }

        // Validate update interval
        if let Some(interval) = config.get("update_interval") {
            let interval_val = interval
                .as_integer()
                .context("'update_interval' must be an integer")?;

            if interval_val < 60 {
                warn!(
                    "Crypto update interval ({} seconds) is very short, may exceed API rate limits",
                    interval_val
                );
            }
        }

        // Validate coins array
        if let Some(coins) = config.get("coins") {
            let coins_array = coins.as_array().context("'coins' must be an array")?;

            if coins_array.is_empty() {
                anyhow::bail!("'coins' array cannot be empty");
            }

            for coin in coins_array {
                if coin.as_str().is_none() {
                    anyhow::bail!("All items in 'coins' array must be strings");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_widget_creation() {
        let widget = CryptoWidget::default();
        assert_eq!(widget.info().id, "crypto");
        assert_eq!(widget.coins().len(), 2);
        assert_eq!(widget.currency(), "usd");
    }

    #[test]
    fn test_crypto_widget_custom() {
        let coins = vec![
            "bitcoin".to_string(),
            "ethereum".to_string(),
            "solana".to_string(),
        ];
        let widget = CryptoWidget::new(coins.clone(), "eur", false, 300);
        assert_eq!(widget.coins(), coins.as_slice());
        assert_eq!(widget.currency(), "eur");
        assert!(!widget.show_change);
    }

    #[test]
    fn test_crypto_price_display_with_change() {
        let price = CryptoPrice {
            symbol: "BTC".to_string(),
            price: 50000.0,
            change_24h: Some(5.5),
        };
        let display = price.display(true);
        assert!(display.contains("BTC"));
        assert!(display.contains("$50000"));
        assert!(display.contains("+5.50%"));
    }

    #[test]
    fn test_crypto_price_display_negative_change() {
        let price = CryptoPrice {
            symbol: "ETH".to_string(),
            price: 3000.0,
            change_24h: Some(-2.3),
        };
        let display = price.display(true);
        assert!(display.contains("ETH"));
        assert!(display.contains("$3000"));
        assert!(display.contains("-2.30%"));
    }

    #[test]
    fn test_crypto_price_display_without_change() {
        let price = CryptoPrice {
            symbol: "SOL".to_string(),
            price: 100.5,
            change_24h: Some(3.2),
        };
        let display = price.display(false);
        assert!(display.contains("SOL"));
        assert!(display.contains("$100.50"));
        assert!(!display.contains("%"));
    }

    #[test]
    fn test_crypto_price_display_small_price() {
        let price = CryptoPrice {
            symbol: "DOGE".to_string(),
            price: 0.0725,
            change_24h: None,
        };
        let display = price.display(true);
        assert!(display.contains("DOGE"));
        assert!(display.contains("$0.0725"));
    }

    #[test]
    fn test_crypto_widget_set_data() {
        let mut widget = CryptoWidget::default();
        let prices = vec![CryptoPrice {
            symbol: "BTC".to_string(),
            price: 50000.0,
            change_24h: Some(5.0),
        }];
        widget.set_data(prices);
        assert!(widget.data.is_some());
        assert!(widget.error_message.is_none());
    }

    #[test]
    fn test_crypto_widget_set_error() {
        let mut widget = CryptoWidget::default();
        widget.set_error("API Error".to_string());
        assert!(widget.error_message.is_some());
    }

    #[test]
    fn test_crypto_widget_display_string_with_error_no_data() {
        let mut widget = CryptoWidget::default();
        widget.set_error("Connection failed".to_string());
        let display = widget.display_string();
        assert!(display.is_some());
        assert!(display.unwrap().contains("Error"));
    }

    #[test]
    fn test_factory_creation() {
        let factory = CryptoWidgetFactory;
        let config = factory.default_config();
        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "crypto");
    }

    #[test]
    fn test_factory_validation_invalid_currency() {
        let factory = CryptoWidgetFactory;
        let mut config = toml::Table::new();
        config.insert(
            "currency".to_string(),
            toml::Value::String("gbp".to_string()),
        );
        assert!(factory.validate_config(&config).is_err());
    }

    #[test]
    fn test_factory_validation_empty_coins() {
        let factory = CryptoWidgetFactory;
        let mut config = toml::Table::new();
        config.insert("coins".to_string(), toml::Value::Array(vec![]));
        assert!(factory.validate_config(&config).is_err());
    }

    #[test]
    fn test_factory_validation_valid() {
        let factory = CryptoWidgetFactory;
        let config = factory.default_config();
        assert!(factory.validate_config(&config).is_ok());
    }
}
