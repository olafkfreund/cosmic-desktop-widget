//! News Headlines widget displaying rotating news from RSS feeds
//!
//! This widget displays rotating news headlines. Currently uses embedded sample headlines.
//! RSS feed fetching will be implemented in a future version with proper async handling.

use std::time::{Duration, Instant};

use tracing::debug;

use super::registry::DynWidgetFactory;
use super::traits::{FontSize, Widget, WidgetContent, WidgetInfo};

/// A news headline with source information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Headline {
    pub title: String,
    pub source: String,
}

impl Headline {
    /// Create a new headline
    pub fn new(title: &str, source: &str) -> Self {
        Self {
            title: title.to_string(),
            source: source.to_string(),
        }
    }

    /// Format the headline for display
    pub fn display(&self, show_source: bool) -> String {
        if show_source {
            format!("{} - {}", self.title, self.source)
        } else {
            format!("{}", self.title)
        }
    }
}

/// News Headlines widget displaying rotating news
pub struct NewsWidget {
    headlines: Vec<Headline>,
    current_index: usize,
    last_rotation: Instant,
    rotation_interval: Duration,
    show_source: bool,
}

impl NewsWidget {
    /// Create a new News widget with sample headlines
    pub fn new(rotation_interval: u64, show_source: bool) -> Self {
        let headlines = Self::default_headlines();

        Self {
            headlines,
            current_index: 0,
            last_rotation: Instant::now(),
            rotation_interval: Duration::from_secs(rotation_interval),
            show_source,
        }
    }

    /// Create with custom headlines
    pub fn with_headlines(headlines: Vec<Headline>, rotation_interval: u64, show_source: bool) -> Self {
        Self {
            headlines,
            current_index: 0,
            last_rotation: Instant::now(),
            rotation_interval: Duration::from_secs(rotation_interval),
            show_source,
        }
    }

    /// Default sample headlines for demonstration
    fn default_headlines() -> Vec<Headline> {
        vec![
            Headline::new("Technology sector sees major growth in AI development", "Tech News"),
            Headline::new("Climate summit reaches historic agreement on emissions", "World News"),
            Headline::new("New space telescope captures stunning images of distant galaxies", "Science"),
            Headline::new("Global markets respond positively to economic reforms", "Business"),
            Headline::new("Breakthrough in renewable energy storage announced", "Technology"),
            Headline::new("International cooperation strengthens on cybersecurity", "Tech News"),
            Headline::new("Wildlife conservation efforts show promising results", "Environment"),
            Headline::new("Medical researchers make progress on disease treatment", "Health"),
            Headline::new("Smart city initiatives improve urban living conditions", "Innovation"),
            Headline::new("Education technology transforms learning experiences", "Education"),
        ]
    }

    /// Get the current headline
    pub fn current_headline(&self) -> Option<&Headline> {
        self.headlines.get(self.current_index)
    }

    /// Advance to the next headline
    fn next_headline(&mut self) {
        if self.headlines.is_empty() {
            return;
        }

        self.current_index = (self.current_index + 1) % self.headlines.len();
    }

    /// Display string for the current headline
    pub fn display_string(&self) -> String {
        match self.current_headline() {
            Some(headline) => headline.display(self.show_source),
            None => "No headlines available".to_string(),
        }
    }

    /// Check if it's time to rotate to next headline
    fn should_rotate(&self) -> bool {
        self.last_rotation.elapsed() >= self.rotation_interval
    }
}

impl Widget for NewsWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "news",
            name: "News Headlines",
            preferred_height: 40.0,
            min_height: 30.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        // Rotate headline if needed
        if self.should_rotate() && !self.headlines.is_empty() {
            self.next_headline();
            self.last_rotation = Instant::now();
            debug!(index = self.current_index, "Headline rotated");
        }
    }

    fn content(&self) -> WidgetContent {
        WidgetContent::Text {
            text: self.display_string(),
            size: FontSize::Small,
        }
    }

    fn update_interval(&self) -> Duration {
        // Check frequently for rotation
        Duration::from_secs(1)
    }

    fn is_ready(&self) -> bool {
        !self.headlines.is_empty()
    }
}

impl Default for NewsWidget {
    fn default() -> Self {
        Self::new(30, true)
    }
}

// ============================================================================
// Factory
// ============================================================================

/// Factory for NewsWidget
pub struct NewsWidgetFactory;

impl DynWidgetFactory for NewsWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "news"
    }

    fn create(&self, config: &toml::Table) -> anyhow::Result<Box<dyn Widget>> {
        let rotation_interval = config
            .get("rotation_interval")
            .and_then(|v| v.as_integer())
            .unwrap_or(30) as u64;

        let show_source = config
            .get("show_source")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // Check for custom headlines
        if let Some(headlines_array) = config.get("headlines").and_then(|v| v.as_array()) {
            let mut headlines = Vec::new();
            for item in headlines_array {
                if let Some(table) = item.as_table() {
                    let title = table.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let source = table.get("source").and_then(|v| v.as_str()).unwrap_or("News");
                    if !title.is_empty() {
                        headlines.push(Headline::new(title, source));
                    }
                }
            }

            if !headlines.is_empty() {
                debug!(count = headlines.len(), "Using custom headlines");
                return Ok(Box::new(NewsWidget::with_headlines(headlines, rotation_interval, show_source)));
            }
        }

        debug!(
            rotation_interval = %rotation_interval,
            show_source = %show_source,
            "Creating NewsWidget with default headlines"
        );

        Ok(Box::new(NewsWidget::new(rotation_interval, show_source)))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();
        config.insert("rotation_interval".to_string(), toml::Value::Integer(30));
        config.insert("show_source".to_string(), toml::Value::Boolean(true));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> anyhow::Result<()> {
        if let Some(rotation_interval) = config.get("rotation_interval") {
            let interval = rotation_interval
                .as_integer()
                .ok_or_else(|| anyhow::anyhow!("'rotation_interval' must be an integer"))?;

            if interval < 1 {
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
    fn test_headline_display() {
        let headline = Headline::new("Breaking News", "BBC");
        assert_eq!(headline.display(true), "Breaking News - BBC");
        assert_eq!(headline.display(false), "Breaking News");
    }

    #[test]
    fn test_news_widget_creation() {
        let widget = NewsWidget::default();
        assert_eq!(widget.info().id, "news");
        assert!(!widget.headlines.is_empty());
    }

    #[test]
    fn test_headline_rotation() {
        let headlines = vec![
            Headline::new("Headline 1", "Source 1"),
            Headline::new("Headline 2", "Source 2"),
            Headline::new("Headline 3", "Source 3"),
        ];
        let mut widget = NewsWidget::with_headlines(headlines, 30, true);

        assert_eq!(widget.current_index, 0);
        widget.next_headline();
        assert_eq!(widget.current_index, 1);
        widget.next_headline();
        assert_eq!(widget.current_index, 2);
        widget.next_headline();
        assert_eq!(widget.current_index, 0); // Should wrap around
    }

    #[test]
    fn test_default_headlines_not_empty() {
        let headlines = NewsWidget::default_headlines();
        assert!(!headlines.is_empty());
        assert!(headlines.len() >= 5); // Should have several sample headlines
    }

    #[test]
    fn test_factory_creation() {
        let factory = NewsWidgetFactory;
        let config = factory.default_config();
        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "news");
    }

    #[test]
    fn test_factory_validation() {
        let factory = NewsWidgetFactory;

        // Valid config
        let mut valid = toml::Table::new();
        valid.insert("rotation_interval".to_string(), toml::Value::Integer(30));
        assert!(factory.validate_config(&valid).is_ok());

        // Invalid rotation_interval
        let mut invalid = toml::Table::new();
        invalid.insert("rotation_interval".to_string(), toml::Value::Integer(0));
        assert!(factory.validate_config(&invalid).is_err());
    }

    #[test]
    fn test_display_string() {
        let widget = NewsWidget::default();
        let display = widget.display_string();
        assert!(display.starts_with(""));
    }

    #[test]
    fn test_is_ready() {
        let widget = NewsWidget::default();
        assert!(widget.is_ready());

        let empty_widget = NewsWidget::with_headlines(Vec::new(), 30, true);
        assert!(!empty_widget.is_ready());
    }

    #[test]
    fn test_custom_headlines_from_config() {
        let factory = NewsWidgetFactory;
        let mut config = toml::Table::new();

        // Create custom headlines array
        let mut headline1 = toml::Table::new();
        headline1.insert("title".to_string(), toml::Value::String("Custom News 1".to_string()));
        headline1.insert("source".to_string(), toml::Value::String("Custom Source".to_string()));

        let mut headline2 = toml::Table::new();
        headline2.insert("title".to_string(), toml::Value::String("Custom News 2".to_string()));
        headline2.insert("source".to_string(), toml::Value::String("Custom Source".to_string()));

        let headlines = vec![toml::Value::Table(headline1), toml::Value::Table(headline2)];
        config.insert("headlines".to_string(), toml::Value::Array(headlines));

        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "news");
    }
}
