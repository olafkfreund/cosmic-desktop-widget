//! Configuration GUI for COSMIC Desktop Widget
//!
//! This application provides a graphical interface for configuring the desktop widget
//! using libcosmic for native COSMIC Desktop integration.
//!
//! Follows COSMIC Desktop UI design patterns:
//! - Uses cosmic::widget::settings module for settings pages
//! - Uses theme spacing variables for consistent layout
//! - Sections have titles and descriptions
//! - Proper navigation using segmented buttons

use cosmic::{
    app::{Core, Task},
    cosmic_config, cosmic_theme,
    iced::{
        alignment::{Horizontal, Vertical},
        Alignment, Length,
    },
    widget::{
        self, button, column, container, dropdown, horizontal_space, icon, row,
        segmented_button, settings, slider, text, text_input, toggler, vertical_space,
    },
    Application, Apply, Element,
    theme,
};
use cosmic_desktop_widget::{Config, GradientConfig, Position, SoundsConfig, ThemeColors, ThemeConfig, ThemeStyle};

const APP_ID: &str = "com.github.olafkfreund.cosmic-desktop-widget-config";

fn main() -> cosmic::iced::Result {
    tracing_subscriber::fmt::init();

    let settings = cosmic::app::Settings::default();

    cosmic::app::run::<ConfigApp>(settings, ())
}

/// Available tabs in the configuration UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    General,
    Appearance,
    Theme,
    Sounds,
    Widgets,
}

impl Tab {
    fn title(&self) -> &'static str {
        match self {
            Tab::General => "General",
            Tab::Appearance => "Appearance",
            Tab::Theme => "Theme",
            Tab::Sounds => "Sounds",
            Tab::Widgets => "Widgets",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Tab::General => "Panel size, position, and layout settings",
            Tab::Appearance => "Theme selection and transparency options",
            Tab::Theme => "Custom color and style configuration",
            Tab::Sounds => "Notification and alarm sounds",
            Tab::Widgets => "Enable, disable, and reorder widgets",
        }
    }

    fn icon_name(&self) -> &'static str {
        match self {
            Tab::General => "preferences-system-symbolic",
            Tab::Appearance => "preferences-desktop-appearance-symbolic",
            Tab::Theme => "preferences-color-symbolic",
            Tab::Sounds => "audio-volume-high-symbolic",
            Tab::Widgets => "view-app-grid-symbolic",
        }
    }
}

/// Messages for the configuration application
#[derive(Debug, Clone)]
enum Message {
    // Tab navigation
    TabSelected(segmented_button::Entity),

    // General settings
    WidthChanged(String),
    HeightChanged(String),
    PositionChanged(Position),
    MarginTopChanged(String),
    MarginRightChanged(String),
    MarginBottomChanged(String),
    MarginLeftChanged(String),
    PaddingChanged(f32),
    SpacingChanged(f32),

    // Appearance settings
    ThemeSelected(String),
    OpacityChanged(f32),

    // Theme editor settings
    ThemeBackgroundChanged(String),
    ThemeTextPrimaryChanged(String),
    ThemeTextSecondaryChanged(String),
    ThemeAccentChanged(String),
    ThemeBorderChanged(String),
    ThemeCornerRadiusChanged(f32),
    ThemeBorderWidthChanged(f32),
    ThemeBlurToggled(bool),
    GradientEnabledToggled(bool),
    GradientStartChanged(String),
    GradientEndChanged(String),
    GradientAngleChanged(f32),

    // Sound settings
    SoundsEnabledToggled(bool),
    SoundsMasterVolumeChanged(f32),
    AlarmSoundSelected(String),
    AlarmVolumeChanged(f32),
    AlarmRepeatChanged(String),
    NotificationSoundSelected(String),
    NotificationVolumeChanged(f32),
    PreviewSound(String),

    // Widget settings
    WidgetToggled(usize, bool),
    WidgetMoveUp(usize),
    WidgetMoveDown(usize),
    WidgetExpanded(usize),
    WidgetRemove(usize),
    WidgetAdd(String),

    // Per-widget configuration
    WidgetPositionChanged(usize, String),
    WidgetWidthChanged(usize, String),
    WidgetHeightChanged(usize, String),
    WidgetMarginTopChanged(usize, String),
    WidgetMarginRightChanged(usize, String),
    WidgetMarginBottomChanged(usize, String),
    WidgetMarginLeftChanged(usize, String),
    WidgetOpacityChanged(usize, f32),

    // Actions
    Save,
    Cancel,
    ConfigSaved(Result<(), String>),
}

/// Configuration application state
struct ConfigApp {
    core: Core,
    current_tab: Tab,
    /// Segmented button model for tab navigation
    tab_model: segmented_button::SingleSelectModel,

    // Configuration state (working copy)
    config: Config,
    original_config: Config,

    // UI state
    width_input: String,
    height_input: String,
    margin_top_input: String,
    margin_right_input: String,
    margin_bottom_input: String,
    margin_left_input: String,

    // Theme editor state
    theme_config: ThemeConfig,
    theme_background_input: String,
    theme_text_primary_input: String,
    theme_text_secondary_input: String,
    theme_accent_input: String,
    theme_border_input: String,
    gradient_start_input: String,
    gradient_end_input: String,

    // Sound settings state
    alarm_repeat_input: String,

    // Available themes
    available_themes: Vec<String>,

    // Available sounds
    available_sounds: Vec<String>,

    // Available widget types (from registry)
    available_widget_types: Vec<String>,

    // Widget configuration state
    expanded_widget: Option<usize>,
    widget_width_inputs: Vec<String>,
    widget_height_inputs: Vec<String>,
    widget_margin_inputs: Vec<(String, String, String, String)>, // top, right, bottom, left

    // Save status
    save_error: Option<String>,
}

impl Application for ConfigApp {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        // Load configuration
        let config = Config::load().unwrap_or_default();
        let original_config = config.clone();

        let available_themes = vec![
            "cosmic_dark".to_string(),
            "light".to_string(),
            "transparent_dark".to_string(),
            "transparent_light".to_string(),
            "glass".to_string(),
            "custom".to_string(),
        ];

        let available_sounds = vec![
            "alarm".to_string(),
            "chime".to_string(),
            "notification".to_string(),
            "beep".to_string(),
        ];

        // All available widget types (12 total)
        let available_widget_types = vec![
            "battery".to_string(),
            "calendar".to_string(),
            "clock".to_string(),
            "countdown".to_string(),
            "crypto".to_string(),
            "mpris".to_string(),
            "news".to_string(),
            "pomodoro".to_string(),
            "quotes".to_string(),
            "stocks".to_string(),
            "system_monitor".to_string(),
            "weather".to_string(),
        ];

        // Initialize theme config from existing or default
        let theme_config = config.theme_config.clone().unwrap_or_default();

        // Initialize widget configuration inputs
        let widget_width_inputs: Vec<String> = config.widgets
            .iter()
            .map(|w| w.width.map(|v| v.to_string()).unwrap_or_else(|| "250".to_string()))
            .collect();

        let widget_height_inputs: Vec<String> = config.widgets
            .iter()
            .map(|w| w.height.map(|v| v.to_string()).unwrap_or_else(|| "90".to_string()))
            .collect();

        let widget_margin_inputs: Vec<(String, String, String, String)> = config.widgets
            .iter()
            .map(|w| {
                (
                    w.margin_top.map(|v| v.to_string()).unwrap_or_else(|| "10".to_string()),
                    w.margin_right.map(|v| v.to_string()).unwrap_or_else(|| "20".to_string()),
                    w.margin_bottom.map(|v| v.to_string()).unwrap_or_else(|| "0".to_string()),
                    w.margin_left.map(|v| v.to_string()).unwrap_or_else(|| "0".to_string()),
                )
            })
            .collect();

        // Create tab navigation model using segmented buttons
        let mut tab_model = segmented_button::SingleSelectModel::builder()
            .insert(|b| b.text(Tab::General.title()).data(Tab::General).activate())
            .insert(|b| b.text(Tab::Appearance.title()).data(Tab::Appearance))
            .insert(|b| b.text(Tab::Theme.title()).data(Tab::Theme))
            .insert(|b| b.text(Tab::Sounds.title()).data(Tab::Sounds))
            .insert(|b| b.text(Tab::Widgets.title()).data(Tab::Widgets))
            .build();

        let app = ConfigApp {
            core,
            current_tab: Tab::General,
            tab_model,
            width_input: config.panel.width.to_string(),
            height_input: config.panel.height.to_string(),
            margin_top_input: config.panel.margin.top.to_string(),
            margin_right_input: config.panel.margin.right.to_string(),
            margin_bottom_input: config.panel.margin.bottom.to_string(),
            margin_left_input: config.panel.margin.left.to_string(),
            theme_background_input: theme_config.colors.background.clone(),
            theme_text_primary_input: theme_config.colors.text_primary.clone(),
            theme_text_secondary_input: theme_config.colors.text_secondary.clone(),
            theme_accent_input: theme_config.colors.accent.clone(),
            theme_border_input: theme_config.colors.border.clone(),
            gradient_start_input: theme_config.gradient.as_ref().map(|g| g.start_color.clone()).unwrap_or_else(|| "#1e1e2e".to_string()),
            gradient_end_input: theme_config.gradient.as_ref().map(|g| g.end_color.clone()).unwrap_or_else(|| "#313244".to_string()),
            theme_config,
            alarm_repeat_input: config.sounds.alarm.repeat.to_string(),
            config,
            original_config,
            available_themes,
            available_sounds,
            available_widget_types,
            expanded_widget: None,
            widget_width_inputs,
            widget_height_inputs,
            widget_margin_inputs,
            save_error: None,
        };

        (app, Task::none())
    }

    fn header_start(&self) -> Vec<Element<Self::Message>> {
        vec![text("COSMIC Desktop Widget Configuration")
            .size(20)
            .into()]
    }

    fn view(&self) -> Element<Self::Message> {
        let spacing = theme::active().cosmic().spacing;

        // Tab content based on current selection
        let content = match self.current_tab {
            Tab::General => self.view_general(),
            Tab::Appearance => self.view_appearance(),
            Tab::Theme => self.view_theme(),
            Tab::Sounds => self.view_sounds(),
            Tab::Widgets => self.view_widgets(),
        };

        // COSMIC-style segmented button navigation
        let tabs = segmented_button::horizontal(&self.tab_model)
            .on_activate(Message::TabSelected)
            .button_padding([spacing.space_xxs, spacing.space_xs, spacing.space_xxs, spacing.space_xs])
            .spacing(spacing.space_xxs);

        // Tab description text
        let tab_description = text::body(self.current_tab.description())
            .apply(container)
            .padding([0, spacing.space_s]);

        // Action buttons following COSMIC patterns
        let buttons = row::with_capacity(3)
            .push(
                button::standard("Cancel")
                    .on_press(Message::Cancel)
            )
            .push(horizontal_space())
            .push(
                button::suggested("Save")
                    .on_press(Message::Save)
            )
            .spacing(spacing.space_s)
            .padding([spacing.space_s, spacing.space_m]);

        // Main content layout
        let mut main_content = column::with_capacity(5)
            .push(
                container(tabs)
                    .padding([spacing.space_s, spacing.space_m])
                    .width(Length::Fill)
            )
            .push(tab_description)
            .push(content)
            .push(vertical_space().height(Length::Fixed(spacing.space_s as f32)))
            .push(buttons)
            .spacing(0);

        // Error message if present
        if let Some(error) = &self.save_error {
            main_content = main_content.push(
                container(
                    text::body(error)
                )
                .padding(spacing.space_s)
                .width(Length::Fill)
            );
        }

        container(main_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::TabSelected(entity) => {
                self.tab_model.activate(entity);
                if let Some(tab) = self.tab_model.data::<Tab>(entity) {
                    self.current_tab = *tab;
                }
            }

            // General settings updates
            Message::WidthChanged(value) => {
                self.width_input = value.clone();
                if let Ok(width) = value.parse::<u32>() {
                    if width > 0 && width <= 10000 {
                        self.config.panel.width = width;
                    }
                }
            }
            Message::HeightChanged(value) => {
                self.height_input = value.clone();
                if let Ok(height) = value.parse::<u32>() {
                    if height > 0 && height <= 10000 {
                        self.config.panel.height = height;
                    }
                }
            }
            Message::PositionChanged(position) => {
                self.config.panel.position = position;
            }
            Message::MarginTopChanged(value) => {
                self.margin_top_input = value.clone();
                if let Ok(margin) = value.parse::<i32>() {
                    self.config.panel.margin.top = margin;
                }
            }
            Message::MarginRightChanged(value) => {
                self.margin_right_input = value.clone();
                if let Ok(margin) = value.parse::<i32>() {
                    self.config.panel.margin.right = margin;
                }
            }
            Message::MarginBottomChanged(value) => {
                self.margin_bottom_input = value.clone();
                if let Ok(margin) = value.parse::<i32>() {
                    self.config.panel.margin.bottom = margin;
                }
            }
            Message::MarginLeftChanged(value) => {
                self.margin_left_input = value.clone();
                if let Ok(margin) = value.parse::<i32>() {
                    self.config.panel.margin.left = margin;
                }
            }
            Message::PaddingChanged(value) => {
                self.config.panel.padding = value;
            }
            Message::SpacingChanged(value) => {
                self.config.panel.spacing = value;
            }

            // Appearance settings
            Message::ThemeSelected(theme) => {
                self.config.panel.theme = theme;
            }
            Message::OpacityChanged(opacity) => {
                self.config.panel.background_opacity = Some(opacity);
            }

            // Theme editor settings
            Message::ThemeBackgroundChanged(value) => {
                self.theme_background_input = value.clone();
                self.theme_config.colors.background = value;
                self.config.theme_config = Some(self.theme_config.clone());
            }
            Message::ThemeTextPrimaryChanged(value) => {
                self.theme_text_primary_input = value.clone();
                self.theme_config.colors.text_primary = value;
                self.config.theme_config = Some(self.theme_config.clone());
            }
            Message::ThemeTextSecondaryChanged(value) => {
                self.theme_text_secondary_input = value.clone();
                self.theme_config.colors.text_secondary = value;
                self.config.theme_config = Some(self.theme_config.clone());
            }
            Message::ThemeAccentChanged(value) => {
                self.theme_accent_input = value.clone();
                self.theme_config.colors.accent = value;
                self.config.theme_config = Some(self.theme_config.clone());
            }
            Message::ThemeBorderChanged(value) => {
                self.theme_border_input = value.clone();
                self.theme_config.colors.border = value;
                self.config.theme_config = Some(self.theme_config.clone());
            }
            Message::ThemeCornerRadiusChanged(value) => {
                self.theme_config.style.corner_radius = value;
                self.config.theme_config = Some(self.theme_config.clone());
            }
            Message::ThemeBorderWidthChanged(value) => {
                self.theme_config.style.border_width = value;
                self.config.theme_config = Some(self.theme_config.clone());
            }
            Message::ThemeBlurToggled(enabled) => {
                self.theme_config.style.blur_enabled = enabled;
                self.config.theme_config = Some(self.theme_config.clone());
            }
            Message::GradientEnabledToggled(enabled) => {
                if self.theme_config.gradient.is_none() {
                    self.theme_config.gradient = Some(GradientConfig::default());
                }
                if let Some(ref mut gradient) = self.theme_config.gradient {
                    gradient.enabled = enabled;
                }
                self.config.theme_config = Some(self.theme_config.clone());
            }
            Message::GradientStartChanged(value) => {
                self.gradient_start_input = value.clone();
                if self.theme_config.gradient.is_none() {
                    self.theme_config.gradient = Some(GradientConfig::default());
                }
                if let Some(ref mut gradient) = self.theme_config.gradient {
                    gradient.start_color = value;
                }
                self.config.theme_config = Some(self.theme_config.clone());
            }
            Message::GradientEndChanged(value) => {
                self.gradient_end_input = value.clone();
                if self.theme_config.gradient.is_none() {
                    self.theme_config.gradient = Some(GradientConfig::default());
                }
                if let Some(ref mut gradient) = self.theme_config.gradient {
                    gradient.end_color = value;
                }
                self.config.theme_config = Some(self.theme_config.clone());
            }
            Message::GradientAngleChanged(value) => {
                if self.theme_config.gradient.is_none() {
                    self.theme_config.gradient = Some(GradientConfig::default());
                }
                if let Some(ref mut gradient) = self.theme_config.gradient {
                    gradient.angle = value;
                }
                self.config.theme_config = Some(self.theme_config.clone());
            }

            // Sound settings
            Message::SoundsEnabledToggled(enabled) => {
                self.config.sounds.enabled = enabled;
            }
            Message::SoundsMasterVolumeChanged(volume) => {
                self.config.sounds.volume = volume;
            }
            Message::AlarmSoundSelected(sound) => {
                self.config.sounds.alarm.effect = sound;
            }
            Message::AlarmVolumeChanged(volume) => {
                self.config.sounds.alarm.volume = volume;
            }
            Message::AlarmRepeatChanged(value) => {
                self.alarm_repeat_input = value.clone();
                if let Ok(repeat) = value.parse::<u32>() {
                    self.config.sounds.alarm.repeat = repeat.clamp(1, 10);
                }
            }
            Message::NotificationSoundSelected(sound) => {
                self.config.sounds.notification.effect = sound;
            }
            Message::NotificationVolumeChanged(volume) => {
                self.config.sounds.notification.volume = volume;
            }
            Message::PreviewSound(_sound_type) => {
                // Audio preview would be implemented here if audio feature is enabled
                // For now, this is a no-op
            }

            // Widget settings
            Message::WidgetToggled(index, enabled) => {
                if let Some(widget) = self.config.widgets.get_mut(index) {
                    widget.enabled = enabled;
                }
            }
            Message::WidgetMoveUp(index) => {
                if index > 0 && index < self.config.widgets.len() {
                    self.config.widgets.swap(index - 1, index);
                }
            }
            Message::WidgetMoveDown(index) => {
                if index < self.config.widgets.len() - 1 {
                    self.config.widgets.swap(index, index + 1);
                }
            }
            Message::WidgetExpanded(index) => {
                // Toggle expanded state
                if self.expanded_widget == Some(index) {
                    self.expanded_widget = None;
                } else {
                    self.expanded_widget = Some(index);
                }
            }
            Message::WidgetRemove(index) => {
                if index < self.config.widgets.len() {
                    self.config.widgets.remove(index);
                    // Also remove from input state vectors
                    if index < self.widget_width_inputs.len() {
                        self.widget_width_inputs.remove(index);
                    }
                    if index < self.widget_height_inputs.len() {
                        self.widget_height_inputs.remove(index);
                    }
                    if index < self.widget_margin_inputs.len() {
                        self.widget_margin_inputs.remove(index);
                    }
                    // Reset expanded state
                    self.expanded_widget = None;
                }
            }
            Message::WidgetAdd(widget_type) => {
                use cosmic_desktop_widget::WidgetInstance;
                let new_widget = WidgetInstance::new(&widget_type);
                self.config.widgets.push(new_widget);
                // Add input state for new widget
                self.widget_width_inputs.push("250".to_string());
                self.widget_height_inputs.push("90".to_string());
                self.widget_margin_inputs.push(("10".to_string(), "20".to_string(), "0".to_string(), "0".to_string()));
            }

            // Per-widget configuration
            Message::WidgetPositionChanged(index, position) => {
                if let Some(widget) = self.config.widgets.get_mut(index) {
                    widget.position = Some(position);
                }
            }
            Message::WidgetWidthChanged(index, value) => {
                if index < self.widget_width_inputs.len() {
                    self.widget_width_inputs[index] = value.clone();
                }
                if let Ok(width) = value.parse::<u32>() {
                    if width > 0 && width <= 10000 {
                        if let Some(widget) = self.config.widgets.get_mut(index) {
                            widget.width = Some(width);
                        }
                    }
                }
            }
            Message::WidgetHeightChanged(index, value) => {
                if index < self.widget_height_inputs.len() {
                    self.widget_height_inputs[index] = value.clone();
                }
                if let Ok(height) = value.parse::<u32>() {
                    if height > 0 && height <= 10000 {
                        if let Some(widget) = self.config.widgets.get_mut(index) {
                            widget.height = Some(height);
                        }
                    }
                }
            }
            Message::WidgetMarginTopChanged(index, value) => {
                if index < self.widget_margin_inputs.len() {
                    self.widget_margin_inputs[index].0 = value.clone();
                }
                if let Ok(margin) = value.parse::<i32>() {
                    if let Some(widget) = self.config.widgets.get_mut(index) {
                        widget.margin_top = Some(margin);
                    }
                }
            }
            Message::WidgetMarginRightChanged(index, value) => {
                if index < self.widget_margin_inputs.len() {
                    self.widget_margin_inputs[index].1 = value.clone();
                }
                if let Ok(margin) = value.parse::<i32>() {
                    if let Some(widget) = self.config.widgets.get_mut(index) {
                        widget.margin_right = Some(margin);
                    }
                }
            }
            Message::WidgetMarginBottomChanged(index, value) => {
                if index < self.widget_margin_inputs.len() {
                    self.widget_margin_inputs[index].2 = value.clone();
                }
                if let Ok(margin) = value.parse::<i32>() {
                    if let Some(widget) = self.config.widgets.get_mut(index) {
                        widget.margin_bottom = Some(margin);
                    }
                }
            }
            Message::WidgetMarginLeftChanged(index, value) => {
                if index < self.widget_margin_inputs.len() {
                    self.widget_margin_inputs[index].3 = value.clone();
                }
                if let Ok(margin) = value.parse::<i32>() {
                    if let Some(widget) = self.config.widgets.get_mut(index) {
                        widget.margin_left = Some(margin);
                    }
                }
            }
            Message::WidgetOpacityChanged(index, opacity) => {
                if let Some(widget) = self.config.widgets.get_mut(index) {
                    widget.opacity = Some(opacity);
                }
            }

            // Actions
            Message::Save => {
                match self.config.save() {
                    Ok(_) => {
                        self.original_config = self.config.clone();
                        self.save_error = None;
                    }
                    Err(e) => {
                        self.save_error = Some(format!("Failed to save: {}", e));
                    }
                }
            }
            Message::Cancel => {
                self.config = self.original_config.clone();
                self.width_input = self.config.panel.width.to_string();
                self.height_input = self.config.panel.height.to_string();
                self.margin_top_input = self.config.panel.margin.top.to_string();
                self.margin_right_input = self.config.panel.margin.right.to_string();
                self.margin_bottom_input = self.config.panel.margin.bottom.to_string();
                self.margin_left_input = self.config.panel.margin.left.to_string();
                self.save_error = None;
            }

            Message::ConfigSaved(_) => {
                // Informational message
            }
        }

        Task::none()
    }
}

impl ConfigApp {
    /// View for General settings tab - follows COSMIC settings patterns
    fn view_general(&self) -> Element<Message> {
        let spacing = theme::active().cosmic().spacing;

        // Size section with description
        let size_section = settings::section()
            .title("Panel Size")
            .add(
                settings::item(
                    "Width",
                    text_input("450", &self.width_input)
                        .on_input(Message::WidthChanged)
                        .width(Length::Fixed(80.0)),
                )

            )
            .add(
                settings::item(
                    "Height",
                    text_input("180", &self.height_input)
                        .on_input(Message::HeightChanged)
                        .width(Length::Fixed(80.0)),
                )

            );

        // Position section with visual grid
        let position_section = settings::section()
            .title("Default Position")
            .add(
                settings::item(
                    "Screen Position",
                    self.view_position_grid(),
                )

            );

        // Margins section with descriptions
        let margin_section = settings::section()
            .title("Screen Margins")
            .add(
                settings::item(
                    "Top",
                    text_input("10", &self.margin_top_input)
                        .on_input(Message::MarginTopChanged)
                        .width(Length::Fixed(80.0)),
                )

            )
            .add(
                settings::item(
                    "Right",
                    text_input("20", &self.margin_right_input)
                        .on_input(Message::MarginRightChanged)
                        .width(Length::Fixed(80.0)),
                )

            )
            .add(
                settings::item(
                    "Bottom",
                    text_input("0", &self.margin_bottom_input)
                        .on_input(Message::MarginBottomChanged)
                        .width(Length::Fixed(80.0)),
                )

            )
            .add(
                settings::item(
                    "Left",
                    text_input("0", &self.margin_left_input)
                        .on_input(Message::MarginLeftChanged)
                        .width(Length::Fixed(80.0)),
                )

            );

        // Layout section with sliders
        let layout_section = settings::section()
            .title("Layout")
            .add(
                settings::item(
                    "Padding",
                    row::with_capacity(2)
                        .push(
                            slider(0.0..=50.0, self.config.panel.padding, Message::PaddingChanged)
                                .width(Length::Fixed(200.0))
                                .step(1.0)
                        )
                        .push(text::body(format!("{:.0}px", self.config.panel.padding)))
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center),
                )

            )
            .add(
                settings::item(
                    "Spacing",
                    row::with_capacity(2)
                        .push(
                            slider(0.0..=50.0, self.config.panel.spacing, Message::SpacingChanged)
                                .width(Length::Fixed(200.0))
                                .step(1.0)
                        )
                        .push(text::body(format!("{:.0}px", self.config.panel.spacing)))
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center),
                )

            );

        // Use settings::view_column for proper COSMIC layout
        let content = settings::view_column(vec![
            size_section.into(),
            position_section.into(),
            margin_section.into(),
            layout_section.into(),
        ])
        .padding(spacing.space_m);

        container(widget::scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// View for Appearance settings tab - follows COSMIC patterns
    fn view_appearance(&self) -> Element<Message> {
        let spacing = theme::active().cosmic().spacing;

        let current_theme_idx = self.available_themes
            .iter()
            .position(|t| t == &self.config.panel.theme);

        let theme_section = settings::section()
            .title("Theme")
            .add(
                settings::item(
                    "Color Scheme",
                    dropdown(
                        &self.available_themes,
                        current_theme_idx,
                        move |idx| {
                            let themes = vec![
                                "cosmic_dark".to_string(),
                                "light".to_string(),
                                "transparent_dark".to_string(),
                                "transparent_light".to_string(),
                                "glass".to_string(),
                                "custom".to_string(),
                            ];
                            Message::ThemeSelected(themes.get(idx).cloned().unwrap_or_default())
                        },
                    )
                    .width(Length::Fixed(200.0)),
                )

            );

        let opacity = self.config.panel.background_opacity.unwrap_or(0.9);
        let opacity_section = settings::section()
            .title("Transparency")
            .add(
                settings::item(
                    "Background Opacity",
                    row::with_capacity(2)
                        .push(
                            slider(0.0..=1.0, opacity, Message::OpacityChanged)
                                .width(Length::Fixed(200.0))
                                .step(0.05)
                        )
                        .push(text::body(format!("{:.0}%", opacity * 100.0)))
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center),
                )

            );

        let hint_section = settings::section()
            .title("Advanced")
            .add(
                settings::item_row(vec![
                    text::body("For custom colors and gradients, use the Theme tab.").into(),
                ]),
            );

        let content = settings::view_column(vec![
            theme_section.into(),
            opacity_section.into(),
            hint_section.into(),
        ])
        .padding(spacing.space_m);

        container(widget::scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// View for Theme editor tab - follows COSMIC patterns
    fn view_theme(&self) -> Element<Message> {
        let spacing = theme::active().cosmic().spacing;

        // Colors section with hex input fields
        let colors_section = settings::section()
            .title("Custom Colors")
            .add(
                settings::item(
                    "Background",
                    text_input("#1e1e2e", &self.theme_background_input)
                        .on_input(Message::ThemeBackgroundChanged)
                        .width(Length::Fixed(100.0)),
                )

            )
            .add(
                settings::item(
                    "Primary Text",
                    text_input("#cdd6f4", &self.theme_text_primary_input)
                        .on_input(Message::ThemeTextPrimaryChanged)
                        .width(Length::Fixed(100.0)),
                )

            )
            .add(
                settings::item(
                    "Secondary Text",
                    text_input("#a6adc8", &self.theme_text_secondary_input)
                        .on_input(Message::ThemeTextSecondaryChanged)
                        .width(Length::Fixed(100.0)),
                )

            )
            .add(
                settings::item(
                    "Accent",
                    text_input("#89b4fa", &self.theme_accent_input)
                        .on_input(Message::ThemeAccentChanged)
                        .width(Length::Fixed(100.0)),
                )

            )
            .add(
                settings::item(
                    "Border",
                    text_input("#45475a", &self.theme_border_input)
                        .on_input(Message::ThemeBorderChanged)
                        .width(Length::Fixed(100.0)),
                )

            );

        // Style section with sliders and toggles
        let style_section = settings::section()
            .title("Style Options")
            .add(
                settings::item(
                    "Corner Radius",
                    row::with_capacity(2)
                        .push(
                            slider(0.0..=30.0, self.theme_config.style.corner_radius, Message::ThemeCornerRadiusChanged)
                                .width(Length::Fixed(200.0))
                                .step(1.0)
                        )
                        .push(text::body(format!("{:.0}px", self.theme_config.style.corner_radius)))
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center),
                )

            )
            .add(
                settings::item(
                    "Border Width",
                    row::with_capacity(2)
                        .push(
                            slider(0.0..=5.0, self.theme_config.style.border_width, Message::ThemeBorderWidthChanged)
                                .width(Length::Fixed(200.0))
                                .step(0.5)
                        )
                        .push(text::body(format!("{:.1}px", self.theme_config.style.border_width)))
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center),
                )

            )
            .add(
                settings::item(
                    "Enable Blur",
                    toggler(self.theme_config.style.blur_enabled)
                        .on_toggle(Message::ThemeBlurToggled),
                )

            );

        // Gradient section
        let gradient_enabled = self.theme_config.gradient.as_ref().map(|g| g.enabled).unwrap_or(false);
        let gradient_angle = self.theme_config.gradient.as_ref().map(|g| g.angle).unwrap_or(135.0);

        let gradient_section = settings::section()
            .title("Gradient Background")
            .add(
                settings::item(
                    "Enable Gradient",
                    toggler(gradient_enabled)
                        .on_toggle(Message::GradientEnabledToggled),
                )

            )
            .add(
                settings::item(
                    "Start Color",
                    text_input("#1e1e2e", &self.gradient_start_input)
                        .on_input(Message::GradientStartChanged)
                        .width(Length::Fixed(100.0)),
                )

            )
            .add(
                settings::item(
                    "End Color",
                    text_input("#313244", &self.gradient_end_input)
                        .on_input(Message::GradientEndChanged)
                        .width(Length::Fixed(100.0)),
                )

            )
            .add(
                settings::item(
                    "Angle",
                    row::with_capacity(2)
                        .push(
                            slider(0.0..=360.0, gradient_angle, Message::GradientAngleChanged)
                                .width(Length::Fixed(200.0))
                                .step(15.0)
                        )
                        .push(text::body(format!("{:.0}°", gradient_angle)))
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center),
                )

            );

        // Info section
        let info_section = settings::section()
            .title("Usage")
            .add(
                settings::item_row(vec![
                    text::body("Set theme to 'custom' in Appearance tab to use these colors.").into(),
                ]),
            );

        let content = settings::view_column(vec![
            info_section.into(),
            colors_section.into(),
            style_section.into(),
            gradient_section.into(),
        ])
        .padding(spacing.space_m);

        container(widget::scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// View for Sounds settings tab - follows COSMIC patterns
    fn view_sounds(&self) -> Element<Message> {
        let spacing = theme::active().cosmic().spacing;

        // General sound settings
        let general_section = settings::section()
            .title("Sound Settings")
            .add(
                settings::item(
                    "Enable Sounds",
                    toggler(self.config.sounds.enabled)
                        .on_toggle(Message::SoundsEnabledToggled),
                )

            )
            .add(
                settings::item(
                    "Master Volume",
                    row::with_capacity(2)
                        .push(
                            slider(0.0..=1.0, self.config.sounds.volume, Message::SoundsMasterVolumeChanged)
                                .width(Length::Fixed(200.0))
                                .step(0.05)
                        )
                        .push(text::body(format!("{:.0}%", self.config.sounds.volume * 100.0)))
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center),
                )

            );

        // Alarm sound settings
        let alarm_sound_idx = self.available_sounds
            .iter()
            .position(|s| s == &self.config.sounds.alarm.effect);

        let alarm_section = settings::section()
            .title("Alarm Sound")
            .add(
                settings::item(
                    "Sound Effect",
                    row::with_capacity(2)
                        .push(
                            dropdown(
                                &self.available_sounds,
                                alarm_sound_idx,
                                move |idx| {
                                    let sounds = vec![
                                        "alarm".to_string(),
                                        "chime".to_string(),
                                        "notification".to_string(),
                                        "beep".to_string(),
                                    ];
                                    Message::AlarmSoundSelected(sounds.get(idx).cloned().unwrap_or_default())
                                },
                            )
                            .width(Length::Fixed(150.0))
                        )
                        .push(
                            button::standard("Preview")
                                .on_press(Message::PreviewSound("alarm".to_string()))
                        )
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center),
                )

            )
            .add(
                settings::item(
                    "Volume",
                    row::with_capacity(2)
                        .push(
                            slider(0.0..=1.0, self.config.sounds.alarm.volume, Message::AlarmVolumeChanged)
                                .width(Length::Fixed(200.0))
                                .step(0.05)
                        )
                        .push(text::body(format!("{:.0}%", self.config.sounds.alarm.volume * 100.0)))
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center),
                )

            )
            .add(
                settings::item(
                    "Repeat Count",
                    text_input("3", &self.alarm_repeat_input)
                        .on_input(Message::AlarmRepeatChanged)
                        .width(Length::Fixed(60.0)),
                )

            );

        // Notification sound settings
        let notification_sound_idx = self.available_sounds
            .iter()
            .position(|s| s == &self.config.sounds.notification.effect);

        let notification_section = settings::section()
            .title("Notification Sound")
            .add(
                settings::item(
                    "Sound Effect",
                    row::with_capacity(2)
                        .push(
                            dropdown(
                                &self.available_sounds,
                                notification_sound_idx,
                                move |idx| {
                                    let sounds = vec![
                                        "alarm".to_string(),
                                        "chime".to_string(),
                                        "notification".to_string(),
                                        "beep".to_string(),
                                    ];
                                    Message::NotificationSoundSelected(sounds.get(idx).cloned().unwrap_or_default())
                                },
                            )
                            .width(Length::Fixed(150.0))
                        )
                        .push(
                            button::standard("Preview")
                                .on_press(Message::PreviewSound("notification".to_string()))
                        )
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center),
                )

            )
            .add(
                settings::item(
                    "Volume",
                    row::with_capacity(2)
                        .push(
                            slider(0.0..=1.0, self.config.sounds.notification.volume, Message::NotificationVolumeChanged)
                                .width(Length::Fixed(200.0))
                                .step(0.05)
                        )
                        .push(text::body(format!("{:.0}%", self.config.sounds.notification.volume * 100.0)))
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center),
                )

            );

        // Info section about audio feature
        let info_section = settings::section()
            .title("Note")
            .add(
                settings::item_row(vec![
                    text::body("Sound playback requires the 'audio' feature enabled at build time.").into(),
                ]),
            );

        let content = settings::view_column(vec![
            general_section.into(),
            alarm_section.into(),
            notification_section.into(),
            info_section.into(),
        ])
        .padding(spacing.space_m);

        container(widget::scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// View for Widgets settings tab - follows COSMIC patterns
    fn view_widgets(&self) -> Element<Message> {
        let spacing = theme::active().cosmic().spacing;

        // Get list of widget types not yet added
        let existing_types: Vec<&str> = self.config.widgets.iter()
            .map(|w| w.widget_type.as_str())
            .collect();
        let available_to_add: Vec<&String> = self.available_widget_types.iter()
            .filter(|t| !existing_types.contains(&t.as_str()))
            .collect();

        // Add widget section (only show if there are widgets left to add)
        let add_widget_section = if !available_to_add.is_empty() {
            let add_buttons: Vec<Element<'_, Message>> = available_to_add.iter().map(|widget_type| {
                let display_name = widget_type
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>() + &widget_type[1..])
                    .unwrap_or_else(|| (*widget_type).clone())
                    .replace('_', " ");

                let wt = (*widget_type).clone();
                button::standard(format!("+ {}", display_name))
                    .on_press(Message::WidgetAdd(wt))
                    .into()
            }).collect();

            // Create a flex row that wraps
            let mut button_row = row::with_capacity(add_buttons.len())
                .spacing(spacing.space_xs);
            for btn in add_buttons {
                button_row = button_row.push(btn);
            }

            settings::section()
                .title("Add Widget")
                .add(
                    settings::item_row(vec![
                        text::body("Click to add a new widget type:").into(),
                    ])
                )
                .add(
                    settings::item_row(vec![
                        container(
                            button_row.padding([spacing.space_xxs, 0])
                        ).width(Length::Fill).into(),
                    ])
                )
        } else {
            settings::section()
                .title("Add Widget")
                .add(
                    settings::item_row(vec![
                        text::body("All widget types have been added.").into(),
                    ])
                )
        };

        // Build widget list using settings items
        let mut widgets_section = settings::section()
            .title("Active Widgets");

        if self.config.widgets.is_empty() {
            widgets_section = widgets_section.add(
                settings::item_row(vec![
                    text::body("No widgets configured. Add a widget above to get started.").into(),
                ])
            );
        }

        for (index, widget_instance) in self.config.widgets.iter().enumerate() {
            // Capitalize widget type for display
            let display_name = widget_instance.widget_type
                .chars()
                .next()
                .map(|c| c.to_uppercase().collect::<String>() + &widget_instance.widget_type[1..])
                .unwrap_or_else(|| widget_instance.widget_type.clone())
                .replace('_', " ");

            let is_expanded = self.expanded_widget == Some(index);
            let expand_icon = if is_expanded {
                "go-down-symbolic"
            } else {
                "go-next-symbolic"
            };

            // Widget header row with expand/collapse button
            let header_row = row::with_capacity(7)
                .push(
                    button::icon(icon::from_name(expand_icon))
                        .on_press(Message::WidgetExpanded(index))
                        .padding([spacing.space_xxs, spacing.space_xs])
                )
                .push(text::body(display_name))
                .push(horizontal_space())
                .push(
                    toggler(widget_instance.enabled)
                        .on_toggle(move |enabled| Message::WidgetToggled(index, enabled))
                )
                .push(
                    button::icon(icon::from_name("go-up-symbolic"))
                        .on_press_maybe(if index > 0 {
                            Some(Message::WidgetMoveUp(index))
                        } else {
                            None
                        })
                        .padding([spacing.space_xxs, spacing.space_xs])
                )
                .push(
                    button::icon(icon::from_name("go-down-symbolic"))
                        .on_press_maybe(
                            if index < self.config.widgets.len() - 1 {
                                Some(Message::WidgetMoveDown(index))
                            } else {
                                None
                            },
                        )
                        .padding([spacing.space_xxs, spacing.space_xs])
                )
                .push(
                    button::icon(icon::from_name("user-trash-symbolic"))
                        .on_press(Message::WidgetRemove(index))
                        .padding([spacing.space_xxs, spacing.space_xs])
                )
                .spacing(spacing.space_xs)
                .align_y(Alignment::Center);

            widgets_section = widgets_section.add(
                settings::item_row(vec![header_row.into()]),
            );

            // Add expanded configuration section if this widget is expanded
            if is_expanded {
                widgets_section = widgets_section.add(
                    settings::item_row(vec![
                        self.view_widget_config(index, widget_instance)
                    ])
                );
            }
        }

        // Help section
        let help_section = settings::section()
            .title("Help")
            .add(
                settings::item_row(vec![
                    text::body("Click the arrow to expand widget configuration.").into(),
                ]),
            )
            .add(
                settings::item_row(vec![
                    text::body("Use the trash icon to remove a widget.").into(),
                ]),
            )
            .add(
                settings::item_row(vec![
                    text::body("Widget order determines display order on screen.").into(),
                ]),
            );

        let content = settings::view_column(vec![
            add_widget_section.into(),
            widgets_section.into(),
            help_section.into(),
        ])
        .padding(spacing.space_m);

        container(widget::scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// View for per-widget configuration - follows COSMIC patterns
    fn view_widget_config(&self, index: usize, widget: &cosmic_desktop_widget::WidgetInstance) -> Element<'_, Message> {
        let spacing = theme::active().cosmic().spacing;

        // Available positions (static list)
        const POSITIONS: &[&str] = &[
            "top-left", "top-center", "top-right",
            "center-left", "center", "center-right",
            "bottom-left", "bottom-center", "bottom-right",
        ];

        let current_position = widget.position.as_ref().map(|s| s.as_str()).unwrap_or("top-right");
        let position_idx = POSITIONS.iter().position(|p| *p == current_position);

        // Position dropdown
        let position_row = row::with_capacity(2)
            .push(text::body("Position:").width(Length::Fixed(100.0)))
            .push(
                dropdown(
                    POSITIONS,
                    position_idx,
                    move |idx| {
                        let pos = POSITIONS.get(idx).map(|s| s.to_string()).unwrap_or_default();
                        Message::WidgetPositionChanged(index, pos)
                    },
                )
                .width(Length::Fixed(150.0))
            )
            .spacing(spacing.space_s)
            .align_y(Alignment::Center);

        // Size inputs
        let width_input = if let Some(input) = self.widget_width_inputs.get(index) {
            input.as_str()
        } else {
            "250"
        };

        let height_input = if let Some(input) = self.widget_height_inputs.get(index) {
            input.as_str()
        } else {
            "90"
        };

        let size_row = row::with_capacity(5)
            .push(text::body("Size:").width(Length::Fixed(100.0)))
            .push(text::body("Width"))
            .push(
                text_input("250", width_input)
                    .on_input(move |value| Message::WidgetWidthChanged(index, value))
                    .width(Length::Fixed(70.0))
            )
            .push(text::body("Height"))
            .push(
                text_input("90", height_input)
                    .on_input(move |value| Message::WidgetHeightChanged(index, value))
                    .width(Length::Fixed(70.0))
            )
            .spacing(spacing.space_s)
            .align_y(Alignment::Center);

        // Margin inputs
        let (margin_top, margin_right, margin_bottom, margin_left) =
            if let Some(inputs) = self.widget_margin_inputs.get(index) {
                (inputs.0.as_str(), inputs.1.as_str(), inputs.2.as_str(), inputs.3.as_str())
            } else {
                ("10", "20", "0", "0")
            };

        let margin_row = row::with_capacity(9)
            .push(text::body("Margins:").width(Length::Fixed(100.0)))
            .push(text::body("T"))
            .push(
                text_input("10", margin_top)
                    .on_input(move |value| Message::WidgetMarginTopChanged(index, value))
                    .width(Length::Fixed(50.0))
            )
            .push(text::body("R"))
            .push(
                text_input("20", margin_right)
                    .on_input(move |value| Message::WidgetMarginRightChanged(index, value))
                    .width(Length::Fixed(50.0))
            )
            .push(text::body("B"))
            .push(
                text_input("0", margin_bottom)
                    .on_input(move |value| Message::WidgetMarginBottomChanged(index, value))
                    .width(Length::Fixed(50.0))
            )
            .push(text::body("L"))
            .push(
                text_input("0", margin_left)
                    .on_input(move |value| Message::WidgetMarginLeftChanged(index, value))
                    .width(Length::Fixed(50.0))
            )
            .spacing(spacing.space_s)
            .align_y(Alignment::Center);

        // Opacity slider
        let opacity_value = widget.opacity.unwrap_or(0.9);
        let opacity_row = row::with_capacity(3)
            .push(text::body("Opacity:").width(Length::Fixed(100.0)))
            .push(
                slider(0.0..=1.0, opacity_value, move |value| Message::WidgetOpacityChanged(index, value))
                    .width(Length::Fixed(200.0))
                    .step(0.05)
            )
            .push(text::body(format!("{:.0}%", opacity_value * 100.0)))
            .spacing(spacing.space_s)
            .align_y(Alignment::Center);

        // Build configuration column
        column::with_capacity(4)
            .push(position_row)
            .push(size_row)
            .push(margin_row)
            .push(opacity_row)
            .spacing(spacing.space_s)
            .padding([spacing.space_s, spacing.space_m])
            .into()
    }

    /// Create a 3x3 grid for position selection - follows COSMIC patterns
    fn view_position_grid(&self) -> Element<'_, Message> {
        let spacing = theme::active().cosmic().spacing;

        let positions = [
            [Position::TopLeft, Position::TopCenter, Position::TopRight],
            [Position::CenterLeft, Position::Center, Position::CenterRight],
            [Position::BottomLeft, Position::BottomCenter, Position::BottomRight],
        ];

        let mut grid_column = column::with_capacity(3).spacing(spacing.space_xxs);

        for row_positions in positions {
            let mut grid_row = row::with_capacity(3).spacing(spacing.space_xxs);

            for position in row_positions {
                let is_selected = self.config.panel.position == position;

                // Use shorter labels for cleaner grid
                let label = match position {
                    Position::TopLeft => "↖ Top Left",
                    Position::TopCenter => "↑ Top",
                    Position::TopRight => "↗ Top Right",
                    Position::CenterLeft => "← Left",
                    Position::Center => "● Center",
                    Position::CenterRight => "→ Right",
                    Position::BottomLeft => "↙ Bottom Left",
                    Position::BottomCenter => "↓ Bottom",
                    Position::BottomRight => "↘ Bottom Right",
                };

                let btn = if is_selected {
                    button::suggested(label)
                        .on_press(Message::PositionChanged(position))
                        .width(Length::Fixed(110.0))
                } else {
                    button::standard(label)
                        .on_press(Message::PositionChanged(position))
                        .width(Length::Fixed(110.0))
                };

                grid_row = grid_row.push(btn);
            }

            grid_column = grid_column.push(grid_row);
        }

        grid_column.into()
    }
}
