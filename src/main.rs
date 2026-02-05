// COSMIC Desktop Widget - Wayland Layer Shell Implementation
// A true desktop widget that lives on your desktop background

use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{Shm, ShmHandler},
};
use std::time::Duration;
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_surface},
    Connection, QueueHandle,
};

use cosmic_desktop_widget::{
    config::Config,
    config_watcher::ConfigWatcher,
    metrics::{Timer, WidgetMetrics, TARGET_RENDER_TIME_MS},
    panel::{MarginAdjustments, PanelDetection},
    render::Renderer,
    surface::WidgetSurface,
    update::UpdateScheduler,
    widget::{ClockWidget, WeatherWidget, Widget, WidgetRegistry},
    InputState, Position,
};

/// Main application state
struct DesktopWidget {
    // Wayland states
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: Shm,
    layer_shell: LayerShell,
    seat_state: SeatState,

    // Multiple widget surfaces (one per widget)
    widget_surfaces: Vec<WidgetSurface>,

    // Rendering
    renderer: Renderer,

    // Dynamic widgets (new system)
    widgets: Vec<Box<dyn Widget>>,

    // Widget layout positions for hit-testing (y_offset, height)
    widget_positions: Vec<(f32, f32)>,

    // Legacy widgets (for backward compatibility during transition)
    clock_widget: Option<ClockWidget>,
    weather_widget: Option<WeatherWidget>,

    // Update coordination
    update_scheduler: UpdateScheduler,

    // Configuration
    config: Config,

    // Panel-aware margins
    panel_margins: MarginAdjustments,

    // Performance metrics
    metrics: WidgetMetrics,

    // Input handling
    input_state: InputState,

    // State
    first_frame: bool,
}

impl CompositorHandler for DesktopWidget {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Handle DPI scaling if needed
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // Handle rotation if needed
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        // Find which widget surface this frame callback is for
        if let Some(idx) = self.widget_surfaces.iter().position(|s| &s.wl_surface == surface) {
            self.draw_widget_surface(idx, qh);
        }
    }
}

impl OutputHandler for DesktopWidget {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for DesktopWidget {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        tracing::info!("Layer surface closed");

        // Find and remove the closed surface
        if let Some(idx) = self.widget_surfaces.iter().position(|s| &s.layer == layer) {
            tracing::info!(widget_index = idx, "Removing closed widget surface");
            self.widget_surfaces.remove(idx);
        }
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // Find which surface this configure is for
        let surface_idx = match self.widget_surfaces.iter().position(|s| &s.layer == layer) {
            Some(idx) => idx,
            None => {
                tracing::warn!("Configure event for unknown surface");
                return;
            }
        };

        let surface = &mut self.widget_surfaces[surface_idx];

        // Update size if compositor changed it
        if configure.new_size.0 > 0 && configure.new_size.1 > 0 {
            surface.resize(configure.new_size.0, configure.new_size.1);
        }

        surface.configured = true;

        tracing::info!(
            widget_index = surface.widget_index,
            width = surface.width,
            height = surface.height,
            "Surface configured"
        );

        // Draw this specific surface
        self.draw_widget_surface(surface_idx, qh);
    }
}

impl ShmHandler for DesktopWidget {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl SeatHandler for DesktopWidget {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wayland_client::protocol::wl_seat::WlSeat,
    ) {
        tracing::debug!("New seat available");
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wayland_client::protocol::wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            tracing::info!("Pointer capability available, initializing pointer");
            let _ = self.seat_state.get_pointer(qh, &seat);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wayland_client::protocol::wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            tracing::info!("Pointer capability removed");
            // Pointer cleanup is handled automatically by SeatState
        }
    }

    fn remove_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wayland_client::protocol::wl_seat::WlSeat,
    ) {
        tracing::debug!("Seat removed");
    }
}

impl PointerHandler for DesktopWidget {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wayland_client::protocol::wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            match &event.kind {
                PointerEventKind::Enter { .. } => {
                    self.input_state.pointer_enter();
                }
                PointerEventKind::Leave { .. } => {
                    self.input_state.pointer_leave();
                    self.input_state
                        .update_hover(None, &mut self.widgets);
                }
                PointerEventKind::Motion { time: _ } => {
                    let (x, y) = event.position;
                    self.input_state.update_position(x, y);
                    // Hover state tracking disabled with multi-surface architecture
                }
                PointerEventKind::Press {
                    time: _,
                    button: _,
                    serial: _,
                } => {
                    // Mouse input handling is disabled for desktop widgets
                    // (KeyboardInteractivity::None means no input events)
                }
                PointerEventKind::Release { .. } => {
                    // Currently no action on release
                }
                PointerEventKind::Axis {
                    time: _,
                    horizontal: _,
                    vertical: _,
                    source: _,
                } => {
                    // Scroll input handling is disabled for desktop widgets
                    // (KeyboardInteractivity::None means no input events)
                }
            }
        }
    }
}

impl DesktopWidget {
    fn new(
        registry_state: RegistryState,
        output_state: OutputState,
        compositor_state: CompositorState,
        shm_state: Shm,
        layer_shell: LayerShell,
        seat_state: SeatState,
        config: Config,
    ) -> Self {
        // Get theme from config
        let theme = config.get_theme();

        // Detect COSMIC panels to avoid overlap
        let panel_detection = PanelDetection::detect();
        let panel_margins = panel_detection.margin_adjustments();
        tracing::info!(
            top = panel_margins.top,
            bottom = panel_margins.bottom,
            left = panel_margins.left,
            right = panel_margins.right,
            "Panel margins detected"
        );

        // Create widgets using the new registry system
        let registry = WidgetRegistry::with_builtins();
        let mut widgets: Vec<Box<dyn Widget>> = Vec::new();
        let mut clock_widget: Option<ClockWidget> = None;
        let mut weather_widget: Option<WeatherWidget> = None;

        for instance in config.enabled_widgets() {
            match registry.create(&instance.widget_type, &instance.config) {
                Ok(widget) => {
                    tracing::info!(
                        widget_type = %instance.widget_type,
                        "Created widget from config"
                    );

                    // Keep references to clock/weather for legacy rendering
                    if instance.widget_type == "clock" {
                        // Extract config values for legacy widget
                        let format = instance
                            .config
                            .get("format")
                            .and_then(|v| v.as_str())
                            .unwrap_or("24h");
                        let show_seconds = instance
                            .config
                            .get("show_seconds")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);
                        let show_date = instance
                            .config
                            .get("show_date")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        clock_widget = Some(ClockWidget::new(format, show_seconds, show_date));
                    } else if instance.widget_type == "weather" {
                        let city = instance
                            .config
                            .get("city")
                            .and_then(|v| v.as_str())
                            .unwrap_or("London");
                        let api_key = instance
                            .config
                            .get("api_key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let temp_unit = instance
                            .config
                            .get("temperature_unit")
                            .and_then(|v| v.as_str())
                            .unwrap_or("celsius");
                        let update_interval = instance
                            .config
                            .get("update_interval")
                            .and_then(|v| v.as_integer())
                            .unwrap_or(600) as u64;
                        weather_widget = Some(WeatherWidget::new(
                            city,
                            api_key,
                            temp_unit,
                            update_interval,
                        ));
                    }

                    widgets.push(widget);
                }
                Err(e) => {
                    tracing::error!(
                        widget_type = %instance.widget_type,
                        error = %e,
                        "Failed to create widget"
                    );
                }
            }
        }

        // Calculate minimum update interval from all widgets
        let min_interval = widgets
            .iter()
            .map(|w| w.update_interval())
            .min()
            .unwrap_or(Duration::from_secs(1));

        tracing::info!(
            widget_count = widgets.len(),
            min_update_interval_ms = min_interval.as_millis(),
            "Widgets initialized"
        );

        Self {
            registry_state,
            output_state,
            compositor_state,
            shm_state,
            layer_shell,
            seat_state,
            widget_surfaces: Vec::new(), // Created separately
            renderer: Renderer::with_theme(theme),
            widgets,
            widget_positions: Vec::new(), // Populated during first layout
            clock_widget,
            weather_widget,
            update_scheduler: UpdateScheduler::new(
                Duration::from_secs(1),   // Clock updates every second
                Duration::from_secs(600), // Default weather interval
            ),
            config,
            panel_margins,
            metrics: WidgetMetrics::new(),
            input_state: InputState::new(),
            first_frame: true,
        }
    }

    /// Create Layer Shell surfaces for all enabled widgets
    fn create_widget_surfaces(&mut self, qh: &QueueHandle<Self>) {
        self.widget_surfaces.clear();

        for (widget_index, widget_config) in self.config.widgets.iter().enumerate() {
            if !widget_config.enabled {
                continue;
            }

            // Parse position
            let position = widget_config.position.parse::<Position>().unwrap_or_default();

            // Create Wayland surface
            let wl_surface = self.compositor_state.create_surface(qh);

            // Create Layer Shell surface
            let layer = self.layer_shell.create_layer_surface(
                qh,
                wl_surface.clone(),
                Layer::Bottom, // Below windows, above wallpaper
                Some(format!("cosmic-widget-{}", widget_index)),
                None, // All outputs
            );

            // Configure position using position enum
            let anchor = position.to_anchor();
            layer.set_anchor(anchor);
            layer.set_size(widget_config.width, widget_config.height);

            // Combine config margins with auto-detected panel margins
            let margin = widget_config.margin.as_ref().unwrap_or(&self.config.panel.margin);
            let top = margin.top + self.panel_margins.top;
            let right = margin.right + self.panel_margins.right;
            let bottom = margin.bottom + self.panel_margins.bottom;
            let left = margin.left + self.panel_margins.left;

            layer.set_margin(top, right, bottom, left);
            layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            layer.set_exclusive_zone(-1); // Don't reserve space

            layer.commit();

            // Create widget surface
            let surface = WidgetSurface::new(
                layer,
                wl_surface,
                widget_config.width,
                widget_config.height,
                widget_index,
                position,
                widget_config.opacity,
            );

            tracing::info!(
                widget_index = widget_index,
                position = %position,
                width = widget_config.width,
                height = widget_config.height,
                opacity = widget_config.opacity,
                "Created widget surface"
            );

            self.widget_surfaces.push(surface);
        }
    }


    /// Update widget layout positions for hit-testing
    ///
    /// With multi-surface architecture, each widget is in its own surface,
    /// so this method is now a no-op. Kept for backward compatibility during transition.
    fn update_widget_positions(&mut self) {
        self.widget_positions.clear();

        // In multi-surface mode, each widget has its own surface
        // so hit-testing is per-surface rather than global
        for widget in &self.widgets {
            let info = widget.info();
            // Store placeholder positions (will be removed once input handling is updated)
            self.widget_positions.push((0.0, info.preferred_height));
        }

        tracing::debug!(
            widget_count = self.widgets.len(),
            "Widget positions updated (multi-surface mode)"
        );
    }

    /// Reload configuration and update widget state
    ///
    /// This is called when the config file changes. It:
    /// 1. Loads the new configuration
    /// 2. Recreates widgets based on new config
    /// 3. Updates renderer theme
    /// 4. Resizes and repositions the surface if needed
    fn reload_config(&mut self, qh: &QueueHandle<Self>) -> Result<()> {
        tracing::info!("Reloading configuration");

        // Load new configuration
        let new_config = match Config::load() {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::error!(error = %e, "Failed to load config during reload, keeping current config");
                return Err(e);
            }
        };

        // Note: With multi-surface architecture, individual widget changes trigger surface recreation
        // No need to track panel-level size/position changes separately

        // Update theme if changed
        let theme_changed = new_config.panel.theme != self.config.panel.theme
            || new_config.panel.background_opacity != self.config.panel.background_opacity;

        if theme_changed {
            let new_theme = new_config.get_theme();
            self.renderer = Renderer::with_theme(new_theme);
            tracing::info!("Theme updated");
        }

        // Recreate widgets from new config
        let registry = WidgetRegistry::with_builtins();
        let mut new_widgets: Vec<Box<dyn Widget>> = Vec::new();
        let mut new_clock_widget: Option<ClockWidget> = None;
        let mut new_weather_widget: Option<WeatherWidget> = None;

        for instance in new_config.enabled_widgets() {
            match registry.create(&instance.widget_type, &instance.config) {
                Ok(widget) => {
                    tracing::debug!(
                        widget_type = %instance.widget_type,
                        "Created widget from reloaded config"
                    );

                    // Keep references to clock/weather for legacy rendering
                    if instance.widget_type == "clock" {
                        let format = instance
                            .config
                            .get("format")
                            .and_then(|v| v.as_str())
                            .unwrap_or("24h");
                        let show_seconds = instance
                            .config
                            .get("show_seconds")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);
                        let show_date = instance
                            .config
                            .get("show_date")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        new_clock_widget = Some(ClockWidget::new(format, show_seconds, show_date));
                    } else if instance.widget_type == "weather" {
                        let city = instance
                            .config
                            .get("city")
                            .and_then(|v| v.as_str())
                            .unwrap_or("London");
                        let api_key = instance
                            .config
                            .get("api_key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let temp_unit = instance
                            .config
                            .get("temperature_unit")
                            .and_then(|v| v.as_str())
                            .unwrap_or("celsius");
                        let update_interval = instance
                            .config
                            .get("update_interval")
                            .and_then(|v| v.as_integer())
                            .unwrap_or(600) as u64;
                        new_weather_widget = Some(WeatherWidget::new(
                            city,
                            api_key,
                            temp_unit,
                            update_interval,
                        ));
                    }

                    new_widgets.push(widget);
                }
                Err(e) => {
                    tracing::error!(
                        widget_type = %instance.widget_type,
                        error = %e,
                        "Failed to create widget during config reload"
                    );
                }
            }
        }

        // Update widgets
        self.widgets = new_widgets;
        self.clock_widget = new_clock_widget;
        self.weather_widget = new_weather_widget;

        tracing::info!(
            widget_count = self.widgets.len(),
            "Widgets recreated from config"
        );

        // Update config
        self.config = new_config;

        // Recalculate panel margins
        let panel_detection = PanelDetection::detect();
        self.panel_margins = panel_detection.margin_adjustments();

        // Update widget positions for hit-testing
        self.update_widget_positions();

        // Recreate all widget surfaces with new configuration
        tracing::info!("Recreating widget surfaces with new configuration");

        // Drop old surfaces (Wayland cleanup handled automatically)
        self.widget_surfaces.clear();

        // Create new surfaces with updated config
        self.create_widget_surfaces(qh);

        tracing::info!("Configuration reload complete");
        Ok(())
    }

    /// Draw a specific widget surface
    fn draw_widget_surface(&mut self, surface_idx: usize, qh: &QueueHandle<Self>) {
        // Check if surface index is valid
        if surface_idx >= self.widget_surfaces.len() {
            tracing::error!(surface_idx = surface_idx, "Invalid surface index");
            return;
        }

        let surface = &mut self.widget_surfaces[surface_idx];

        if !surface.configured {
            return;
        }

        // Get the widget for this surface
        let widget_index = surface.widget_index;
        if widget_index >= self.widgets.len() {
            tracing::error!(widget_index = widget_index, "Invalid widget index");
            return;
        }

        // Create buffer pool if needed
        if surface.buffer_pool.is_none() {
            if let Err(e) = surface.init_buffer_pool(&self.shm_state, qh) {
                tracing::error!(
                    error = %e,
                    widget_index = widget_index,
                    "Failed to create buffer pool for widget surface"
                );
                return;
            }
        }

        let buffer_pool = match surface.buffer_pool.as_mut() {
            Some(pool) => pool,
            None => {
                tracing::error!(widget_index = widget_index, "Buffer pool not initialized");
                return;
            }
        };

        let (buffer, canvas) = match buffer_pool.get_buffer() {
            Ok(buf) => buf,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    widget_index = widget_index,
                    "Failed to get buffer, skipping frame"
                );
                return;
            }
        };

        // Time the render operation
        let render_timer = Timer::start();

        // Render single widget with its opacity
        let widget = &self.widgets[widget_index];
        self.renderer.render_single_widget(
            canvas,
            surface.width,
            surface.height,
            widget.as_ref(),
            surface.opacity,
        );

        // Record render metrics
        let render_time = render_timer.stop();
        self.metrics.render.record_render(render_time);

        // Log warning if over frame budget
        if render_time.as_millis() > TARGET_RENDER_TIME_MS as u128 {
            tracing::warn!(
                render_ms = %render_time.as_secs_f64() * 1000.0,
                target_ms = %TARGET_RENDER_TIME_MS,
                widget_index = widget_index,
                "Render exceeded frame budget"
            );
        } else {
            tracing::trace!(
                render_ms = %render_time.as_secs_f64() * 1000.0,
                widget_index = widget_index,
                "Widget render complete"
            );
        }

        // Attach buffer and commit
        surface
            .wl_surface
            .damage_buffer(0, 0, surface.width as i32, surface.height as i32);

        if let Err(e) = buffer.attach_to(&surface.wl_surface) {
            tracing::error!(
                error = %e,
                widget_index = widget_index,
                "Failed to attach buffer to surface"
            );
            return;
        }

        surface.wl_surface.commit();

        // Mark first frame as rendered
        if surface.first_frame {
            surface.first_frame = false;
            tracing::info!(widget_index = widget_index, "First frame rendered");
        }
    }

    /// Draw all widget surfaces
    fn draw_all_surfaces(&mut self, qh: &QueueHandle<Self>) {
        // Update all widgets first
        for widget in &mut self.widgets {
            widget.update();
        }

        // Draw each surface
        for i in 0..self.widget_surfaces.len() {
            self.draw_widget_surface(i, qh);
        }

        // Periodically log metrics summary
        self.metrics.maybe_log_summary();
    }
}

impl ProvidesRegistryState for DesktopWidget {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(DesktopWidget);
delegate_output!(DesktopWidget);
delegate_shm!(DesktopWidget);
delegate_layer!(DesktopWidget);
delegate_seat!(DesktopWidget);
delegate_pointer!(DesktopWidget);
delegate_registry!(DesktopWidget);

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting COSMIC Desktop Widget");

    // Load configuration
    let config = Config::load()?;
    tracing::info!(
        widgets = config.widgets.len(),
        panel_width = config.panel.width,
        panel_height = config.panel.height,
        "Configuration loaded"
    );

    // Connect to Wayland
    let conn = Connection::connect_to_env()
        .context("Failed to connect to Wayland compositor. Is a Wayland compositor running?")?;

    tracing::info!("Connected to Wayland");

    let (globals, event_queue) =
        registry_queue_init(&conn).context("Failed to initialize Wayland registry")?;
    let qh = event_queue.handle();

    // Initialize Wayland states
    let registry_state = RegistryState::new(&globals);
    let output_state = OutputState::new(&globals, &qh);
    let compositor_state = CompositorState::bind(&globals, &qh).context(
        "wl_compositor protocol not available. Your compositor may not support required Wayland protocols.",
    )?;
    let shm_state = Shm::bind(&globals, &qh)
        .context("wl_shm protocol not available. Shared memory buffers are required.")?;
    let layer_shell = LayerShell::bind(&globals, &qh).context(
        "zwlr_layer_shell_v1 not available. Your compositor must support the Layer Shell protocol.",
    )?;
    let seat_state = SeatState::new(&globals, &qh);

    let mut widget = DesktopWidget::new(
        registry_state,
        output_state,
        compositor_state,
        shm_state,
        layer_shell,
        seat_state,
        config,
    );

    // Create widget surfaces (one per enabled widget)
    widget.create_widget_surfaces(&qh);

    // Setup config file watcher for hot-reload
    let config_watcher = match Config::config_path() {
        Ok(path) => match ConfigWatcher::new(path) {
            Ok(watcher) => {
                tracing::info!("Config file watcher enabled");
                Some(watcher)
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to create config watcher, hot-reload disabled");
                None
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "Failed to get config path, hot-reload disabled");
            None
        }
    };

    // Setup event loop
    let mut event_loop =
        calloop::EventLoop::<DesktopWidget>::try_new().context("Failed to create event loop")?;

    // Add Wayland source using WaylandSource
    WaylandSource::new(conn.clone(), event_queue)
        .insert(event_loop.handle())
        .context("Failed to insert Wayland event source into event loop")?;

    // Store config watcher in a shared state for the timer callback
    let config_watcher_shared = std::sync::Arc::new(std::sync::Mutex::new(config_watcher));

    // Timer for periodic updates - uses dynamic interval based on widget needs
    // Performance optimization: Instead of fixed 100ms polling, we sleep until
    // the next widget actually needs an update. This dramatically reduces CPU
    // usage when idle.
    let timer = calloop::timer::Timer::from_duration(Duration::from_secs(1));
    let qh_clone = qh.clone();
    let config_watcher_clone = config_watcher_shared.clone();
    event_loop
        .handle()
        .insert_source(timer, move |_deadline, _metadata, widget| {
            // Check for config reload events
            if let Ok(watcher_guard) = config_watcher_clone.lock() {
                if let Some(ref watcher) = *watcher_guard {
                    if let Some(_reload_event) = watcher.try_recv() {
                        tracing::info!("Config reload triggered by file change");

                        // Reload configuration and update widget state
                        if let Err(e) = widget.reload_config(&qh_clone) {
                            tracing::error!(error = %e, "Failed to reload configuration");
                        } else {
                            // Force a redraw after config reload
                            widget.first_frame = true;
                        }
                    }
                }
            }

            // Calculate time until next widget needs updating
            // This is typically 1 second for clock updates, longer for weather
            let next_update = widget.update_scheduler.time_until_next_update();

            // Clamp to reasonable bounds:
            // - Minimum 50ms to avoid busy-looping on edge cases
            // - Maximum 1 second to ensure clock updates stay responsive
            let sleep_duration = next_update.clamp(
                Duration::from_millis(50),
                Duration::from_secs(1),
            );

            tracing::trace!(
                next_update_ms = next_update.as_millis(),
                sleep_ms = sleep_duration.as_millis(),
                "Timer scheduling next wake"
            );

            calloop::timer::TimeoutAction::ToDuration(sleep_duration)
        })
        .map_err(|e| anyhow::anyhow!("Failed to insert timer source: {:?}", e))?;

    // Signal handling for graceful shutdown
    let signals = calloop::signals::Signals::new(&[calloop::signals::Signal::SIGINT])
        .context("Failed to create signal handler for graceful shutdown")?;
    event_loop
        .handle()
        .insert_source(signals, |_signal, _metadata, _widget| {
            tracing::info!("Received SIGINT, exiting gracefully");
            std::process::exit(0);
        })
        .map_err(|e| anyhow::anyhow!("Failed to insert signal handler: {:?}", e))?;

    tracing::info!("Event loop starting");

    // Main loop - WaylandSource handles Wayland events
    // Performance optimization: Use None timeout to let calloop sleep until
    // the next event (Wayland or timer). This allows the process to truly
    // idle between updates instead of busy-waiting.
    loop {
        // Use None for timeout - let calloop manage wake-ups based on:
        // - Wayland events (configure, frame callbacks)
        // - Timer source (widget updates)
        // - Signal handlers (SIGINT)
        // This minimizes CPU usage when idle.
        if let Err(e) = event_loop.dispatch(None, &mut widget) {
            tracing::error!(error = %e, "Event loop dispatch error");
            // For critical errors, we should exit
            // For transient errors, we could continue
            // Currently, we exit on any error as most are fatal
            return Err(e.into());
        }
    }
}
