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
    button_code_to_mouse_button,
    config::Config,
    execute_action, hit_test_widgets,
    metrics::{Timer, WidgetMetrics, TARGET_RENDER_TIME_MS},
    panel::{MarginAdjustments, PanelDetection},
    render::Renderer,
    scroll_to_direction,
    update::UpdateScheduler,
    wayland,
    widget::{ClockWidget, MouseButton, WeatherWidget, Widget, WidgetRegistry},
    InputState,
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

    // Our layer surface
    layer: Option<LayerSurface>,

    // Rendering
    renderer: Renderer,
    width: u32,
    height: u32,
    buffer_pool: Option<wayland::BufferPool>,

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
    configured: bool,
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
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(conn, qh);
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
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        tracing::info!("Layer surface closed");
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        if configure.new_size.0 > 0 && configure.new_size.1 > 0 {
            self.width = configure.new_size.0;
            self.height = configure.new_size.1;
        }

        self.configured = true;
        tracing::info!("Configured: {}x{}", self.width, self.height);

        self.draw(conn, qh);
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

                    // Update hover state
                    let widget_index =
                        hit_test_widgets(x, y, &self.widgets, &self.widget_positions);
                    self.input_state
                        .update_hover(widget_index, &mut self.widgets);
                }
                PointerEventKind::Press {
                    time: _,
                    button,
                    serial: _,
                } => {
                    let (x, y) = self.input_state.pointer_position();
                    let widget_index =
                        hit_test_widgets(x, y, &self.widgets, &self.widget_positions);

                    if let Some(index) = widget_index {
                        if let Some(widget) = self.widgets.get_mut(index) {
                            // Calculate normalized coordinates within widget
                            let (y_offset, height) = self.widget_positions[index];
                            let widget_y = (y - y_offset as f64) / height as f64;
                            let widget_x = x / self.width as f64;

                            let mouse_button = button_code_to_mouse_button(*button);
                            if let Some(action) =
                                widget.on_click(mouse_button, widget_x as f32, widget_y as f32)
                            {
                                tracing::info!(
                                    widget = widget.info().id,
                                    button = ?mouse_button,
                                    "Widget click action"
                                );
                                if let Err(e) = execute_action(action) {
                                    tracing::error!(error = %e, "Failed to execute widget action");
                                }

                                // Redraw to show updated widget state
                                if let Some(layer) = &self.layer {
                                    layer
                                        .wl_surface()
                                        .frame(_qh, layer.wl_surface().clone());
                                    layer.wl_surface().commit();
                                }
                            }
                        }
                    }
                }
                PointerEventKind::Release { .. } => {
                    // Currently no action on release
                }
                PointerEventKind::Axis {
                    time: _,
                    horizontal,
                    vertical,
                    source: _,
                } => {
                    let (x, y) = self.input_state.pointer_position();
                    let widget_index =
                        hit_test_widgets(x, y, &self.widgets, &self.widget_positions);

                    if let Some(index) = widget_index {
                        if let Some(widget) = self.widgets.get_mut(index) {
                            // Handle vertical scroll (discrete is i32, non-zero means scroll)
                            let scroll_amount = vertical.discrete;
                            if scroll_amount != 0 {
                                if let Some(direction) = scroll_to_direction(scroll_amount as f64)
                                {
                                    let (y_offset, height) = self.widget_positions[index];
                                    let widget_y = (y - y_offset as f64) / height as f64;
                                    let widget_x = x / self.width as f64;

                                    if let Some(action) = widget.on_scroll(
                                        direction,
                                        widget_x as f32,
                                        widget_y as f32,
                                    ) {
                                        tracing::info!(
                                            widget = widget.info().id,
                                            direction = ?direction,
                                            "Widget scroll action"
                                        );
                                        if let Err(e) = execute_action(action) {
                                            tracing::error!(
                                                error = %e,
                                                "Failed to execute widget action"
                                            );
                                        }

                                        // Redraw to show updated widget state
                                        if let Some(layer) = &self.layer {
                                            layer
                                                .wl_surface()
                                                .frame(_qh, layer.wl_surface().clone());
                                            layer.wl_surface().commit();
                                        }
                                    }
                                }
                            }

                            // Handle horizontal scroll (future use)
                            let _h_scroll = horizontal.discrete;
                            // Could handle left/right scroll in future
                        }
                    }
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
            layer: None,
            renderer: Renderer::with_theme(theme),
            width: config.panel.width,
            height: config.panel.height,
            buffer_pool: None,
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
            configured: false,
            first_frame: true,
        }
    }

    fn create_layer_surface(&mut self, qh: &QueueHandle<Self>) {
        let surface = self.compositor_state.create_surface(qh);

        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Bottom, // Below windows, above wallpaper
            Some("cosmic-desktop-widget"),
            None, // All outputs
        );

        // Configure position based on config
        // Use the type-safe Position enum to convert to Layer Shell anchors
        let anchor = self.config.panel.position.to_anchor();
        layer.set_anchor(anchor);
        layer.set_size(self.width, self.height);

        // Combine config margins with auto-detected panel margins
        let top = self.config.panel.margin.top + self.panel_margins.top;
        let right = self.config.panel.margin.right + self.panel_margins.right;
        let bottom = self.config.panel.margin.bottom + self.panel_margins.bottom;
        let left = self.config.panel.margin.left + self.panel_margins.left;

        layer.set_margin(top, right, bottom, left);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_exclusive_zone(-1); // Don't reserve space

        layer.commit();
        self.layer = Some(layer);

        // Calculate widget positions for hit-testing
        // Simple vertical stacking for now
        self.update_widget_positions();

        tracing::info!(
            margin_top = top,
            margin_right = right,
            "Layer surface created with panel-aware margins"
        );
    }

    /// Update widget layout positions for hit-testing
    ///
    /// Calculates the (y_offset, height) for each widget based on simple
    /// vertical stacking. This should be called when widgets change or surface resizes.
    fn update_widget_positions(&mut self) {
        self.widget_positions.clear();

        let mut y_offset = 0.0;
        for widget in &self.widgets {
            let info = widget.info();
            let height = info.preferred_height.min(self.height as f32);

            self.widget_positions.push((y_offset, height));
            y_offset += height;
        }

        tracing::debug!(
            widget_count = self.widgets.len(),
            positions = ?self.widget_positions,
            "Widget positions updated"
        );
    }

    fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        if !self.configured {
            return;
        }

        let Some(layer) = &self.layer else {
            return;
        };

        // Check which widgets need updating
        let flags = self.update_scheduler.check_updates();

        // Update all dynamic widgets
        for widget in &mut self.widgets {
            widget.update();
        }

        // Update legacy widgets for backward compatibility
        if let Some(ref mut clock) = self.clock_widget {
            if flags.clock || self.first_frame {
                clock.update();
            }
        }
        if let Some(ref mut weather) = self.weather_widget {
            if flags.weather || self.first_frame {
                weather.update();
            }
        }

        // Only redraw if something changed OR this is the first frame
        if !flags.needs_redraw() && !self.first_frame {
            return;
        }

        // Mark first frame as rendered
        if self.first_frame {
            self.first_frame = false;
            tracing::info!("Rendering first frame");
        }

        // Create buffer pool if needed
        if self.buffer_pool.is_none() {
            match wayland::BufferPool::new(self.width, self.height, &self.shm_state, qh) {
                Ok(pool) => {
                    self.buffer_pool = Some(pool);
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        width = self.width,
                        height = self.height,
                        "Failed to create buffer pool, skipping frame"
                    );
                    return;
                }
            }
        }

        let pool = self
            .buffer_pool
            .as_mut()
            .expect("Buffer pool must exist after creation check");
        let (buffer, canvas) = match pool.get_buffer() {
            Ok(buf) => buf,
            Err(e) => {
                tracing::error!(error = %e, "Failed to get buffer, skipping frame");
                return;
            }
        };

        // Time the render operation
        let render_timer = Timer::start();

        // Use legacy rendering path (renderer still expects specific widget types)
        // TODO: Update renderer to use generic Widget trait
        self.renderer.render(
            canvas,
            self.width,
            self.height,
            self.clock_widget.as_ref(),
            self.weather_widget.as_ref(),
            &self.config,
        );

        // Record render metrics
        let render_time = render_timer.stop();
        self.metrics.render.record_render(render_time);

        // Log warning if over frame budget
        if render_time.as_millis() > TARGET_RENDER_TIME_MS as u128 {
            tracing::warn!(
                render_ms = %render_time.as_secs_f64() * 1000.0,
                target_ms = %TARGET_RENDER_TIME_MS,
                "Render exceeded frame budget"
            );
        } else {
            tracing::trace!(
                render_ms = %render_time.as_secs_f64() * 1000.0,
                "Render complete"
            );
        }

        // Periodically log metrics summary
        self.metrics.maybe_log_summary();

        // Attach buffer and commit
        layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);

        if let Err(e) = buffer.attach_to(layer.wl_surface()) {
            tracing::error!(error = %e, "Failed to attach buffer to surface");
            return;
        }

        layer.wl_surface().commit();
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

    // Create the layer surface
    widget.create_layer_surface(&qh);

    // Setup event loop
    let mut event_loop =
        calloop::EventLoop::<DesktopWidget>::try_new().context("Failed to create event loop")?;

    // Add Wayland source using WaylandSource
    WaylandSource::new(conn.clone(), event_queue)
        .insert(event_loop.handle())
        .context("Failed to insert Wayland event source into event loop")?;

    // Timer for periodic updates - uses dynamic interval based on widget needs
    // Performance optimization: Instead of fixed 100ms polling, we sleep until
    // the next widget actually needs an update. This dramatically reduces CPU
    // usage when idle.
    let timer = calloop::timer::Timer::from_duration(Duration::from_secs(1));
    event_loop
        .handle()
        .insert_source(timer, |_deadline, _metadata, widget| {
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
