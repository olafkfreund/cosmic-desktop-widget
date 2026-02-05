//! Stocks widget displaying real-time stock prices
//!
//! This widget shows stock prices, changes, and percentage changes using
//! the Yahoo Finance API (free, no API key required).

use std::time::{Duration, Instant};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use super::registry::DynWidgetFactory;
use super::traits::{FontSize, Widget, WidgetContent, WidgetInfo};

/// Stock data from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockData {
    pub symbol: String,
    pub price: f64,
    pub change: f64,
    pub percent_change: f64,
}

impl StockData {
    /// Format the stock data for display
    pub fn display(&self, show_change: bool, show_percent: bool) -> String {
        let mut parts = vec![format!("{}: ${:.2}", self.symbol, self.price)];

        if show_change || show_percent {
            let mut change_parts = Vec::new();

            if show_change {
                let sign = if self.change >= 0.0 { "+" } else { "" };
                change_parts.push(format!("{}{:.2}", sign, self.change));
            }

            if show_percent {
                let sign = if self.percent_change >= 0.0 { "+" } else { "" };
                change_parts.push(format!("({}{}%)", sign, self.percent_change));
            }

            parts.push(change_parts.join(" "));
        }

        parts.join(" ")
    }
}

/// Stocks widget showing real-time stock prices
pub struct StocksWidget {
    symbols: Vec<String>,
    stocks_data: Vec<StockData>,
    last_update: Instant,
    update_interval: Duration,
    show_change: bool,
    show_percent: bool,
    error_message: Option<String>,
}

impl StocksWidget {
    /// Create a new Stocks widget
    pub fn new(
        symbols: Vec<String>,
        show_change: bool,
        show_percent: bool,
        update_interval: u64,
    ) -> Self {
        Self {
            symbols,
            stocks_data: Vec::new(),
            last_update: Instant::now(),
            update_interval: Duration::from_secs(update_interval),
            show_change,
            show_percent,
            error_message: None,
        }
    }

    /// Fetch stock data from Yahoo Finance API
    pub async fn fetch_stocks(&mut self) -> anyhow::Result<()> {
        if self.symbols.is_empty() {
            return Err(anyhow::anyhow!("No stock symbols configured"));
        }

        debug!(symbols = ?self.symbols, "Fetching stock data from Yahoo Finance API");

        let mut new_stocks_data = Vec::new();

        for symbol in &self.symbols {
            match Self::fetch_single_stock(symbol).await {
                Ok(data) => new_stocks_data.push(data),
                Err(e) => {
                    warn!(symbol = %symbol, error = %e, "Failed to fetch stock data");
                    // Continue with other symbols even if one fails
                }
            }
        }

        if new_stocks_data.is_empty() && !self.symbols.is_empty() {
            return Err(anyhow::anyhow!("Failed to fetch any stock data"));
        }

        self.stocks_data = new_stocks_data;
        self.last_update = Instant::now();
        self.error_message = None;

        debug!(
            count = self.stocks_data.len(),
            "Stock data updated successfully"
        );
        Ok(())
    }

    /// Fetch data for a single stock symbol
    async fn fetch_single_stock(symbol: &str) -> anyhow::Result<StockData> {
        // Use Yahoo Finance query API v7 (free, no API key)
        let url = format!(
            "https://query1.finance.yahoo.com/v7/finance/quote?symbols={}",
            symbol
        );

        let response = reqwest::get(&url)
            .await
            .with_context(|| format!("Failed to fetch data for {}", symbol))?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Yahoo Finance API returned error status: {}",
                response.status()
            );
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Yahoo Finance response")?;

        // Parse the response
        let result = json["quoteResponse"]["result"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| anyhow::anyhow!("No data returned for symbol: {}", symbol))?;

        let price = result["regularMarketPrice"]
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("Missing price data for {}", symbol))?;

        let change = result["regularMarketChange"].as_f64().unwrap_or(0.0);

        let percent_change = result["regularMarketChangePercent"].as_f64().unwrap_or(0.0);

        Ok(StockData {
            symbol: symbol.to_uppercase(),
            price,
            change,
            percent_change,
        })
    }

    /// Set stock data from successful API fetch
    pub fn set_data(&mut self, data: Vec<StockData>) {
        debug!(count = data.len(), "Stock data updated");
        self.stocks_data = data;
        self.last_update = Instant::now();
        self.error_message = None;
    }

    /// Set error message from failed API fetch
    pub fn set_error(&mut self, error: String) {
        warn!(error = %error, "Stock fetch error");
        self.error_message = Some(error);
        // Keep old data if available
    }

    /// Generate display string
    pub fn display_string(&self) -> String {
        // If there's an error and no data, show error
        if self.stocks_data.is_empty() && self.error_message.is_some() {
            return format!("Error: {}", self.error_message.as_ref().unwrap());
        }

        if self.stocks_data.is_empty() {
            return "No stock data".to_string();
        }

        let stock_strings: Vec<String> = self
            .stocks_data
            .iter()
            .map(|stock| stock.display(self.show_change, self.show_percent))
            .collect();

        let result = stock_strings.join(" | ");

        // Add stale indicator if data is old
        let stale_threshold = self.update_interval * 2;
        if self.last_update.elapsed() > stale_threshold {
            format!("{} (stale)", result)
        } else if self.error_message.is_some() {
            // Show warning if there's an error but we have old data
            format!("{} ⚠", result)
        } else {
            result
        }
    }
}

impl Widget for StocksWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "stocks",
            name: "Stocks",
            preferred_height: 40.0,
            min_height: 30.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        // Updates happen in the background thread, not here
        // This method is a no-op similar to WeatherWidget
    }

    fn content(&self) -> WidgetContent {
        WidgetContent::Text {
            text: self.display_string(),
            size: FontSize::Medium,
        }
    }

    fn update_interval(&self) -> Duration {
        self.update_interval
    }

    fn is_ready(&self) -> bool {
        !self.stocks_data.is_empty() || self.error_message.is_some()
    }

    fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

impl Default for StocksWidget {
    fn default() -> Self {
        Self::new(
            vec!["AAPL".to_string()],
            true,
            true,
            300, // 5 minutes
        )
    }
}

// ============================================================================
// Factory
// ============================================================================

/// Factory for StocksWidget
pub struct StocksWidgetFactory;

impl DynWidgetFactory for StocksWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "stocks"
    }

    fn create(&self, config: &toml::Table) -> anyhow::Result<Box<dyn Widget>> {
        // Parse symbols array
        let symbols = if let Some(symbols_value) = config.get("symbols") {
            if let Some(arr) = symbols_value.as_array() {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_uppercase()))
                    .collect()
            } else if let Some(s) = symbols_value.as_str() {
                // Single symbol as string
                vec![s.to_uppercase()]
            } else {
                vec!["AAPL".to_string()]
            }
        } else {
            vec!["AAPL".to_string()]
        };

        if symbols.is_empty() {
            anyhow::bail!("At least one stock symbol must be configured");
        }

        let show_change = config
            .get("show_change")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let show_percent = config
            .get("show_percent")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let update_interval = config
            .get("update_interval")
            .and_then(|v| v.as_integer())
            .unwrap_or(300) as u64;

        debug!(
            symbols = ?symbols,
            show_change = %show_change,
            show_percent = %show_percent,
            update_interval = %update_interval,
            "Creating StocksWidget"
        );

        Ok(Box::new(StocksWidget::new(
            symbols,
            show_change,
            show_percent,
            update_interval,
        )))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();
        config.insert(
            "symbols".to_string(),
            toml::Value::Array(vec![toml::Value::String("AAPL".to_string())]),
        );
        config.insert("show_change".to_string(), toml::Value::Boolean(true));
        config.insert("show_percent".to_string(), toml::Value::Boolean(true));
        config.insert("update_interval".to_string(), toml::Value::Integer(300));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> anyhow::Result<()> {
        // Validate symbols
        if let Some(symbols_value) = config.get("symbols") {
            if let Some(arr) = symbols_value.as_array() {
                if arr.is_empty() {
                    anyhow::bail!("'symbols' array cannot be empty");
                }
                for item in arr {
                    if item.as_str().is_none() {
                        anyhow::bail!("All symbols must be strings");
                    }
                }
            } else if symbols_value.as_str().is_none() {
                anyhow::bail!("'symbols' must be a string or array of strings");
            }
        }

        // Validate update interval
        if let Some(interval) = config.get("update_interval") {
            let interval_val = interval
                .as_integer()
                .ok_or_else(|| anyhow::anyhow!("'update_interval' must be an integer"))?;

            if interval_val < 60 {
                warn!(
                    "Stock update interval ({} seconds) is very short, may exceed API rate limits",
                    interval_val
                );
            }

            if interval_val < 1 {
                anyhow::bail!("'update_interval' must be at least 1 second");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stock_data_display() {
        let stock = StockData {
            symbol: "AAPL".to_string(),
            price: 150.25,
            change: 2.50,
            percent_change: 1.69,
        };

        // With both change and percent
        let display = stock.display(true, true);
        assert!(display.contains("AAPL"));
        assert!(display.contains("150.25"));
        assert!(display.contains("+2.50"));
        assert!(display.contains("+1.69%"));

        // Price only
        let display = stock.display(false, false);
        assert_eq!(display, "AAPL: $150.25");

        // Negative change
        let stock_down = StockData {
            symbol: "GOOGL".to_string(),
            price: 2800.00,
            change: -15.00,
            percent_change: -0.53,
        };
        let display = stock_down.display(true, true);
        assert!(display.contains("-15.00"));
        assert!(display.contains("-0.53%"));
    }

    #[test]
    fn test_stocks_widget_creation() {
        let widget = StocksWidget::new(
            vec!["AAPL".to_string(), "GOOGL".to_string()],
            true,
            true,
            300,
        );
        assert_eq!(widget.info().id, "stocks");
        assert_eq!(widget.symbols.len(), 2);
    }

    #[test]
    fn test_stocks_widget_display_no_data() {
        let widget = StocksWidget::default();
        let display = widget.display_string();
        assert_eq!(display, "No stock data");
    }

    #[test]
    fn test_stocks_widget_display_with_data() {
        let mut widget = StocksWidget::new(vec!["AAPL".to_string()], true, true, 300);
        widget.set_data(vec![StockData {
            symbol: "AAPL".to_string(),
            price: 150.25,
            change: 2.50,
            percent_change: 1.69,
        }]);

        let display = widget.display_string();
        assert!(display.contains("AAPL"));
        assert!(display.contains("150.25"));
    }

    #[test]
    fn test_stocks_widget_display_with_error() {
        let mut widget = StocksWidget::default();
        widget.set_error("API Error".to_string());
        let display = widget.display_string();
        assert!(display.contains("Error"));
    }

    #[test]
    fn test_factory_creation() {
        let factory = StocksWidgetFactory;
        let config = factory.default_config();
        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "stocks");
    }

    #[test]
    fn test_factory_custom_config() {
        let factory = StocksWidgetFactory;
        let mut config = toml::Table::new();
        config.insert(
            "symbols".to_string(),
            toml::Value::Array(vec![
                toml::Value::String("AAPL".to_string()),
                toml::Value::String("MSFT".to_string()),
            ]),
        );
        config.insert("show_change".to_string(), toml::Value::Boolean(false));
        config.insert("update_interval".to_string(), toml::Value::Integer(600));

        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "stocks");
    }

    #[test]
    fn test_factory_validation_empty_symbols() {
        let factory = StocksWidgetFactory;
        let mut config = toml::Table::new();
        config.insert("symbols".to_string(), toml::Value::Array(vec![]));

        assert!(factory.validate_config(&config).is_err());
    }

    #[test]
    fn test_factory_validation_invalid_interval() {
        let factory = StocksWidgetFactory;
        let mut config = toml::Table::new();
        config.insert("update_interval".to_string(), toml::Value::Integer(0));

        assert!(factory.validate_config(&config).is_err());
    }
}
