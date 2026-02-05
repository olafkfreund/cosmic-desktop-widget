//! Configuration GUI for COSMIC Desktop Widget
//!
//! This application provides a graphical interface for configuring the desktop widget
//! using libcosmic for native COSMIC Desktop integration.

use cosmic::{
    app::{Core, Task},
    cosmic_config, cosmic_theme,
    iced::{
        alignment::Horizontal,
        Alignment, Length,
    },
    widget::{
        self, button, container, dropdown, horizontal_space, settings, slider, text,
        text_input, toggler,
    },
    Application, Apply, Element,
    theme,
};
use cosmic_desktop_widget::{Config, Position};

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
    Widgets,
}

impl Tab {
    fn title(&self) -> &'static str {
        match self {
            Tab::General => "General",
            Tab::Appearance => "Appearance",
            Tab::Widgets => "Widgets",
        }
    }
}

/// Messages for the configuration application
#[derive(Debug, Clone)]
enum Message {
    // Tab navigation
    TabSelected(usize),

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

    // Widget settings
    WidgetToggled(usize, bool),
    WidgetMoveUp(usize),
    WidgetMoveDown(usize),

    // Actions
    Save,
    Cancel,
    ConfigSaved(Result<(), String>),
}

/// Configuration application state
struct ConfigApp {
    core: Core,
    current_tab: Tab,

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

    // Available themes
    available_themes: Vec<String>,

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

        let app = ConfigApp {
            core,
            current_tab: Tab::General,
            width_input: config.panel.width.to_string(),
            height_input: config.panel.height.to_string(),
            margin_top_input: config.panel.margin.top.to_string(),
            margin_right_input: config.panel.margin.right.to_string(),
            margin_bottom_input: config.panel.margin.bottom.to_string(),
            margin_left_input: config.panel.margin.left.to_string(),
            config,
            original_config,
            available_themes,
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
        let content = match self.current_tab {
            Tab::General => self.view_general(),
            Tab::Appearance => self.view_appearance(),
            Tab::Widgets => self.view_widgets(),
        };

        let tabs = widget::row()
            .push(if self.current_tab == Tab::General {
                button::suggested(Tab::General.title())
                    .on_press(Message::TabSelected(0))
            } else {
                button::standard(Tab::General.title())
                    .on_press(Message::TabSelected(0))
            })
            .push(if self.current_tab == Tab::Appearance {
                button::suggested(Tab::Appearance.title())
                    .on_press(Message::TabSelected(1))
            } else {
                button::standard(Tab::Appearance.title())
                    .on_press(Message::TabSelected(1))
            })
            .push(if self.current_tab == Tab::Widgets {
                button::suggested(Tab::Widgets.title())
                    .on_press(Message::TabSelected(2))
            } else {
                button::standard(Tab::Widgets.title())
                    .on_press(Message::TabSelected(2))
            })
            .spacing(4)
            .padding(8);

        let buttons = widget::row()
            .push(button::standard("Cancel").on_press(Message::Cancel))
            .push(horizontal_space())
            .push(button::suggested("Save").on_press(Message::Save))
            .spacing(8)
            .padding(16);

        let mut main_content = widget::column()
            .push(tabs)
            .push(content)
            .push(buttons)
            .spacing(0);

        if let Some(error) = &self.save_error {
            main_content = main_content.push(
                container(text(error))
                    .padding(8)
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
            Message::TabSelected(index) => {
                self.current_tab = match index {
                    0 => Tab::General,
                    1 => Tab::Appearance,
                    2 => Tab::Widgets,
                    _ => Tab::General,
                };
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
    /// View for General settings tab
    fn view_general(&self) -> Element<Message> {
        let size_section = settings::section()
            .title("Panel Size")
            .add(
                settings::item(
                    "Width",
                    text_input("450", &self.width_input)
                        .on_input(Message::WidthChanged)
                        .width(Length::Fixed(100.0)),
                ),
            )
            .add(
                settings::item(
                    "Height",
                    text_input("180", &self.height_input)
                        .on_input(Message::HeightChanged)
                        .width(Length::Fixed(100.0)),
                ),
            );

        let position_section = settings::section()
            .title("Position")
            .add(settings::item(
                "Screen Position",
                self.view_position_grid(),
            ));

        let margin_section = settings::section()
            .title("Margins")
            .add(
                settings::item(
                    "Top",
                    text_input("10", &self.margin_top_input)
                        .on_input(Message::MarginTopChanged)
                        .width(Length::Fixed(100.0)),
                ),
            )
            .add(
                settings::item(
                    "Right",
                    text_input("20", &self.margin_right_input)
                        .on_input(Message::MarginRightChanged)
                        .width(Length::Fixed(100.0)),
                ),
            )
            .add(
                settings::item(
                    "Bottom",
                    text_input("0", &self.margin_bottom_input)
                        .on_input(Message::MarginBottomChanged)
                        .width(Length::Fixed(100.0)),
                ),
            )
            .add(
                settings::item(
                    "Left",
                    text_input("0", &self.margin_left_input)
                        .on_input(Message::MarginLeftChanged)
                        .width(Length::Fixed(100.0)),
                ),
            );

        let layout_section = settings::section()
            .title("Layout")
            .add(settings::item(
                "Padding",
                widget::row()
                    .push(
                        slider(0.0..=50.0, self.config.panel.padding, Message::PaddingChanged)
                            .width(Length::Fill)
                            .step(1.0)
                    )
                    .push(text(format!("{:.0}px", self.config.panel.padding)).width(Length::Fixed(60.0)))
                    .spacing(8)
                    .align_y(Alignment::Center),
            ))
            .add(settings::item(
                "Spacing",
                widget::row()
                    .push(
                        slider(0.0..=50.0, self.config.panel.spacing, Message::SpacingChanged)
                            .width(Length::Fill)
                            .step(1.0)
                    )
                    .push(text(format!("{:.0}px", self.config.panel.spacing)).width(Length::Fixed(60.0)))
                    .spacing(8)
                    .align_y(Alignment::Center),
            ));

        let content = widget::column()
            .push(size_section)
            .push(position_section)
            .push(margin_section)
            .push(layout_section)
            .spacing(16)
            .padding(16);

        container(widget::scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// View for Appearance settings tab
    fn view_appearance(&self) -> Element<Message> {
        let current_theme_idx = self.available_themes
            .iter()
            .position(|t| t == &self.config.panel.theme);

        let theme_section = settings::section()
            .title("Theme")
            .add(settings::item(
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
            ));

        let opacity = self.config.panel.background_opacity.unwrap_or(0.9);
        let opacity_section = settings::section()
            .title("Transparency")
            .add(settings::item(
                "Background Opacity",
                widget::row()
                    .push(
                        slider(0.0..=1.0, opacity, Message::OpacityChanged)
                            .width(Length::Fill)
                            .step(0.05)
                    )
                    .push(text(format!("{:.0}%", opacity * 100.0)).width(Length::Fixed(60.0)))
                    .spacing(8)
                    .align_y(Alignment::Center),
            ));

        let content = widget::column()
            .push(theme_section)
            .push(opacity_section)
            .spacing(16)
            .padding(16);

        container(widget::scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// View for Widgets settings tab
    fn view_widgets(&self) -> Element<Message> {
        let mut widget_list = widget::column().spacing(8);

        for (index, widget_instance) in self.config.widgets.iter().enumerate() {
            let widget_row = container(
                widget::row()
                    .push(
                        toggler(widget_instance.enabled)
                            .on_toggle(move |enabled| Message::WidgetToggled(index, enabled))
                            .label(&widget_instance.widget_type)
                    )
                    .push(horizontal_space())
                    .push(
                        button::icon(widget::icon::from_name("go-up-symbolic"))
                            .on_press_maybe(if index > 0 {
                                Some(Message::WidgetMoveUp(index))
                            } else {
                                None
                            })
                            .padding(4)
                    )
                    .push(
                        button::icon(widget::icon::from_name("go-down-symbolic"))
                            .on_press_maybe(
                                if index < self.config.widgets.len() - 1 {
                                    Some(Message::WidgetMoveDown(index))
                                } else {
                                    None
                                },
                            )
                            .padding(4)
                    )
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .padding(8),
            )
            .padding(8)
            .width(Length::Fill);

            widget_list = widget_list.push(widget_row);
        }

        let widgets_section = settings::section()
            .title("Active Widgets")
            .add(widget_list);

        let content = widget::column()
            .push(widgets_section)
            .push(
                text("Enable/disable widgets and reorder them by using the arrow buttons.")
                    .size(12)
            )
            .spacing(16)
            .padding(16);

        container(widget::scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Create a 3x3 grid for position selection
    fn view_position_grid(&self) -> Element<Message> {
        let positions = [
            [Position::TopLeft, Position::TopCenter, Position::TopRight],
            [Position::CenterLeft, Position::Center, Position::CenterRight],
            [Position::BottomLeft, Position::BottomCenter, Position::BottomRight],
        ];

        let mut grid_column = widget::column().spacing(4);

        for row_positions in positions {
            let mut grid_row = widget::row().spacing(4);

            for position in row_positions {
                let is_selected = self.config.panel.position == position;

                let btn = if is_selected {
                    button::suggested(position.as_str())
                        .on_press(Message::PositionChanged(position))
                        .width(Length::Fixed(120.0))
                } else {
                    button::standard(position.as_str())
                        .on_press(Message::PositionChanged(position))
                        .width(Length::Fixed(120.0))
                };

                grid_row = grid_row.push(btn);
            }

            grid_column = grid_column.push(grid_row);
        }

        grid_column.into()
    }
}
