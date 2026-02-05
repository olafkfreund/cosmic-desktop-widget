//! Quotes widget displaying inspirational quotes
//!
//! This widget shows quotes from a configurable source (embedded, file, or JSON).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Context;
use tracing::{debug, warn};

use super::registry::DynWidgetFactory;
use super::traits::{
    FontSize, MouseButton, ScrollDirection, Widget, WidgetAction, WidgetContent, WidgetInfo,
};

/// A quote with optional author attribution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Quote {
    pub text: String,
    #[serde(default)]
    pub author: Option<String>,
}

impl Quote {
    /// Create a new quote
    pub fn new(text: &str, author: Option<&str>) -> Self {
        Self {
            text: text.to_string(),
            author: author.map(|s| s.to_string()),
        }
    }

    /// Format the quote for display
    pub fn display(&self) -> String {
        match &self.author {
            Some(author) => format!("\"{}\" — {}", self.text, author),
            None => format!("\"{}\"", self.text),
        }
    }
}

/// Quotes widget showing inspirational quotes
pub struct QuotesWidget {
    quotes: Vec<Quote>,
    current_index: usize,
    last_update: Instant,
    rotation_interval: Duration,
    random: bool,
}

impl QuotesWidget {
    /// Create a new Quotes widget with embedded quotes
    pub fn new(rotation_interval: u64, random: bool) -> Self {
        let quotes = Self::default_quotes();
        let current_index = if random {
            rand::random::<usize>() % quotes.len()
        } else {
            0
        };

        Self {
            quotes,
            current_index,
            last_update: Instant::now(),
            rotation_interval: Duration::from_secs(rotation_interval),
            random,
        }
    }

    /// Create from a custom list of quotes
    pub fn with_quotes(quotes: Vec<Quote>, rotation_interval: u64, random: bool) -> Self {
        let current_index = if random && !quotes.is_empty() {
            rand::random::<usize>() % quotes.len()
        } else {
            0
        };

        Self {
            quotes,
            current_index,
            last_update: Instant::now(),
            rotation_interval: Duration::from_secs(rotation_interval),
            random,
        }
    }

    /// Load quotes from a JSON file
    pub fn from_file(path: &PathBuf, rotation_interval: u64, random: bool) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read quotes file: {}", path.display()))?;

        let quotes: Vec<Quote> = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse quotes file: {}", path.display()))?;

        if quotes.is_empty() {
            anyhow::bail!("Quotes file is empty");
        }

        debug!(count = quotes.len(), path = %path.display(), "Loaded quotes from file");
        Ok(Self::with_quotes(quotes, rotation_interval, random))
    }

    /// Get the current quote
    pub fn current_quote(&self) -> Option<&Quote> {
        self.quotes.get(self.current_index)
    }

    /// Advance to the next quote
    fn next_quote(&mut self) {
        if self.quotes.is_empty() {
            return;
        }

        if self.random {
            // Pick a random different quote if possible
            if self.quotes.len() > 1 {
                let mut new_index = self.current_index;
                while new_index == self.current_index {
                    new_index = rand::random::<usize>() % self.quotes.len();
                }
                self.current_index = new_index;
            }
        } else {
            self.current_index = (self.current_index + 1) % self.quotes.len();
        }
    }

    /// Default embedded quotes
    fn default_quotes() -> Vec<Quote> {
        vec![
            Quote::new("The only way to do great work is to love what you do.", Some("Steve Jobs")),
            Quote::new("Innovation distinguishes between a leader and a follower.", Some("Steve Jobs")),
            Quote::new("Stay hungry, stay foolish.", Some("Steve Jobs")),
            Quote::new("The future belongs to those who believe in the beauty of their dreams.", Some("Eleanor Roosevelt")),
            Quote::new("It is during our darkest moments that we must focus to see the light.", Some("Aristotle")),
            Quote::new("The only impossible journey is the one you never begin.", Some("Tony Robbins")),
            Quote::new("Success is not final, failure is not fatal: it is the courage to continue that counts.", Some("Winston Churchill")),
            Quote::new("Believe you can and you're halfway there.", Some("Theodore Roosevelt")),
            Quote::new("The best time to plant a tree was 20 years ago. The second best time is now.", Some("Chinese Proverb")),
            Quote::new("Your time is limited, don't waste it living someone else's life.", Some("Steve Jobs")),
            Quote::new("The only limit to our realization of tomorrow is our doubts of today.", Some("Franklin D. Roosevelt")),
            Quote::new("Do what you can, with what you have, where you are.", Some("Theodore Roosevelt")),
            Quote::new("Everything you've ever wanted is on the other side of fear.", Some("George Addair")),
            Quote::new("The mind is everything. What you think you become.", Some("Buddha")),
            Quote::new("Simplicity is the ultimate sophistication.", Some("Leonardo da Vinci")),
            Quote::new("Code is like humor. When you have to explain it, it's bad.", Some("Cory House")),
            Quote::new("First, solve the problem. Then, write the code.", Some("John Johnson")),
            Quote::new("Make it work, make it right, make it fast.", Some("Kent Beck")),
            Quote::new("Any fool can write code that a computer can understand. Good programmers write code that humans can understand.", Some("Martin Fowler")),
            Quote::new("Programs must be written for people to read, and only incidentally for machines to execute.", Some("Harold Abelson")),
        ]
    }

    /// Display string for the current quote
    pub fn display_string(&self) -> String {
        match self.current_quote() {
            Some(quote) => quote.display(),
            None => "No quotes available".to_string(),
        }
    }
}

impl Widget for QuotesWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "quotes",
            name: "Quotes",
            preferred_height: 50.0,
            min_height: 40.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        if self.last_update.elapsed() >= self.rotation_interval {
            self.next_quote();
            self.last_update = Instant::now();
            debug!(index = self.current_index, "Quote rotated");
        }
    }

    fn content(&self) -> WidgetContent {
        WidgetContent::Text {
            text: self.display_string(),
            size: FontSize::Small,
        }
    }

    fn update_interval(&self) -> Duration {
        // Check more frequently than rotation to ensure timely updates
        Duration::from_secs(1)
    }

    fn is_interactive(&self) -> bool {
        true
    }

    fn on_click(&mut self, button: MouseButton, _x: f32, _y: f32) -> Option<WidgetAction> {
        match button {
            MouseButton::Left => {
                // Advance to next quote on left click
                self.next_quote();
                self.last_update = Instant::now(); // Reset timer
                debug!("Quote advanced by click");
                Some(WidgetAction::NextItem)
            }
            MouseButton::Right => {
                // Could add context menu or other action in future
                None
            }
            _ => None,
        }
    }

    fn on_scroll(&mut self, direction: ScrollDirection, _x: f32, _y: f32) -> Option<WidgetAction> {
        match direction {
            ScrollDirection::Down => {
                // Scroll down = next quote
                self.next_quote();
                self.last_update = Instant::now();
                debug!("Quote advanced by scroll");
                Some(WidgetAction::NextItem)
            }
            ScrollDirection::Up => {
                // Scroll up = also next quote (could be previous in future)
                self.next_quote();
                self.last_update = Instant::now();
                debug!("Quote advanced by scroll");
                Some(WidgetAction::NextItem)
            }
            _ => None,
        }
    }
}

impl Default for QuotesWidget {
    fn default() -> Self {
        Self::new(60, true) // Rotate every minute, randomly
    }
}

// ============================================================================
// Factory
// ============================================================================

/// Factory for QuotesWidget
pub struct QuotesWidgetFactory;

impl DynWidgetFactory for QuotesWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "quotes"
    }

    fn create(&self, config: &toml::Table) -> anyhow::Result<Box<dyn Widget>> {
        let rotation_interval = config
            .get("rotation_interval")
            .and_then(|v| v.as_integer())
            .unwrap_or(60) as u64;

        let random = config
            .get("random")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // Check for custom quotes file
        if let Some(file_path) = config.get("quotes_file").and_then(|v| v.as_str()) {
            let path = PathBuf::from(file_path);

            // Expand ~ to home directory
            let expanded_path = if file_path.starts_with("~/") {
                if let Some(home) = dirs::home_dir() {
                    home.join(&file_path[2..])
                } else {
                    path
                }
            } else {
                path
            };

            match QuotesWidget::from_file(&expanded_path, rotation_interval, random) {
                Ok(widget) => return Ok(Box::new(widget)),
                Err(e) => {
                    warn!(error = %e, path = %expanded_path.display(), "Failed to load custom quotes, using defaults");
                }
            }
        }

        // Check for inline quotes
        if let Some(quotes_array) = config.get("quotes").and_then(|v| v.as_array()) {
            let mut quotes = Vec::new();
            for item in quotes_array {
                if let Some(text) = item.as_str() {
                    quotes.push(Quote::new(text, None));
                } else if let Some(table) = item.as_table() {
                    let text = table.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let author = table.get("author").and_then(|v| v.as_str());
                    if !text.is_empty() {
                        quotes.push(Quote::new(text, author));
                    }
                }
            }

            if !quotes.is_empty() {
                debug!(count = quotes.len(), "Using custom inline quotes");
                return Ok(Box::new(QuotesWidget::with_quotes(
                    quotes,
                    rotation_interval,
                    random,
                )));
            }
        }

        debug!(
            rotation_interval = %rotation_interval,
            random = %random,
            "Creating QuotesWidget with default quotes"
        );

        Ok(Box::new(QuotesWidget::new(rotation_interval, random)))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();
        config.insert("rotation_interval".to_string(), toml::Value::Integer(60));
        config.insert("random".to_string(), toml::Value::Boolean(true));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> anyhow::Result<()> {
        if let Some(interval) = config.get("rotation_interval") {
            let interval_val = interval
                .as_integer()
                .ok_or_else(|| anyhow::anyhow!("'rotation_interval' must be an integer"))?;

            if interval_val < 1 {
                anyhow::bail!("'rotation_interval' must be at least 1 second");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_display() {
        let quote = Quote::new("Test quote", Some("Author"));
        assert_eq!(quote.display(), "\"Test quote\" — Author");

        let quote_no_author = Quote::new("Test quote", None);
        assert_eq!(quote_no_author.display(), "\"Test quote\"");
    }

    #[test]
    fn test_quotes_widget_creation() {
        let widget = QuotesWidget::default();
        assert_eq!(widget.info().id, "quotes");
        assert!(widget.current_quote().is_some());
    }

    #[test]
    fn test_quotes_rotation() {
        let quotes = vec![
            Quote::new("Quote 1", None),
            Quote::new("Quote 2", None),
            Quote::new("Quote 3", None),
        ];
        let mut widget = QuotesWidget::with_quotes(quotes, 0, false);

        let first = widget.current_index;
        widget.next_quote();
        let second = widget.current_index;
        widget.next_quote();
        let third = widget.current_index;

        // Sequential rotation
        assert_eq!(first, 0);
        assert_eq!(second, 1);
        assert_eq!(third, 2);
    }

    #[test]
    fn test_default_quotes_not_empty() {
        let quotes = QuotesWidget::default_quotes();
        assert!(!quotes.is_empty());
    }

    #[test]
    fn test_factory_creation() {
        let factory = QuotesWidgetFactory;
        let config = factory.default_config();
        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "quotes");
    }
}
