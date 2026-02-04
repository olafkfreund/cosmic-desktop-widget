// COSMIC Desktop Widget - Wayland Layer Shell Implementation
// A true desktop widget that lives on your desktop background

use anyhow::{Result, Context};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{Shm, ShmHandler},
};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_surface},
    Connection, QueueHandle,
};
use std::time::Duration;

use cosmic_desktop_widget::{
    config::Config,
    metrics::{Timer, WidgetMetrics, TARGET_RENDER_TIME_MS},
    render::Renderer,
    update::UpdateScheduler,
    wayland,
    widget::{ClockWidget, WeatherWidget},
};

/// Main application state
struct DesktopWidget {
    // Wayland states
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: Shm,
    layer_shell: LayerShell,

    // Our layer surface
    layer: Option<LayerSurface>,
    
    // Rendering
    renderer: Renderer,
    width: u32,
    height: u32,
    buffer_pool: Option<wayland::BufferPool>,
    
    // Widget data
    clock_widget: ClockWidget,
    weather_widget: WeatherWidget,

    // Update coordination
    update_scheduler: UpdateScheduler,

    // Configuration
    config: Config,

    // Performance metrics
    metrics: WidgetMetrics,

    // State
    configured: bool,
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

impl DesktopWidget {
    fn new(
        registry_state: RegistryState,
        output_state: OutputState,
        compositor_state: CompositorState,
        shm_state: Shm,
        layer_shell: LayerShell,
        config: Config,
    ) -> Self {
        // Get theme from config
        let theme = config.get_theme();

        Self {
            registry_state,
            output_state,
            compositor_state,
            shm_state,
            layer_shell,
            layer: None,
            renderer: Renderer::with_theme(theme),
            width: config.width,
            height: config.height,
            buffer_pool: None,
            clock_widget: ClockWidget::new(
                &config.clock_format,
                config.show_seconds,
                config.show_date,
            ),
            weather_widget: WeatherWidget::new(
                &config.weather_city,
                &config.weather_api_key,
                &config.temperature_unit,
                config.update_interval,
            ),
            update_scheduler: UpdateScheduler::new(
                Duration::from_secs(1),              // Clock updates every second
                Duration::from_secs(config.update_interval), // Weather from config
            ),
            config,
            metrics: WidgetMetrics::new(),
            configured: false,
        }
    }

    fn create_layer_surface(&mut self, qh: &QueueHandle<Self>) {
        let surface = self.compositor_state.create_surface(qh);

        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Bottom,  // Below windows, above wallpaper
            Some("cosmic-desktop-widget"),
            None, // All outputs
        );

        // Configure position based on config
        let anchor = match self.config.position.as_str() {
            "top-left" => Anchor::TOP | Anchor::LEFT,
            "top-right" => Anchor::TOP | Anchor::RIGHT,
            "bottom-left" => Anchor::BOTTOM | Anchor::LEFT,
            "bottom-right" => Anchor::BOTTOM | Anchor::RIGHT,
            "center" => Anchor::empty(),
            _ => Anchor::TOP | Anchor::RIGHT,
        };

        layer.set_anchor(anchor);
        layer.set_size(self.width, self.height);
        layer.set_margin(
            self.config.margin.top,
            self.config.margin.right,
            self.config.margin.bottom,
            self.config.margin.left,
        );
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_exclusive_zone(-1); // Don't reserve space

        layer.commit();
        self.layer = Some(layer);
        
        tracing::info!("Layer surface created");
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

        // Only update widgets that need it
        if flags.clock {
            self.clock_widget.update();
        }
        if flags.weather {
            self.weather_widget.update();
        }

        // Only redraw if something changed
        if !flags.needs_redraw() {
            return;
        }

        // Create buffer pool if needed
        if self.buffer_pool.is_none() {
            match wayland::BufferPool::new(
                self.width,
                self.height,
                &self.shm_state,
                qh,
            ) {
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

        let pool = self.buffer_pool.as_mut()
            .expect("Buffer pool must exist after creation check");
        let (buffer, canvas) = match pool.get_buffer() {
            Ok(buf) => buf,
            Err(e) => {
                tracing::error!(error = %e, "Failed to get buffer, skipping frame");
                return;
            }
        };

        // Render the widget - pass Option based on config settings
        let clock_opt = if self.config.show_clock {
            Some(&self.clock_widget)
        } else {
            None
        };
        let weather_opt = if self.config.show_weather {
            Some(&self.weather_widget)
        } else {
            None
        };

        // Time the render operation
        let render_timer = Timer::start();

        self.renderer.render(
            canvas,
            self.width,
            self.height,
            clock_opt,
            weather_opt,
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
    registry_handlers![OutputState];
}

delegate_compositor!(DesktopWidget);
delegate_output!(DesktopWidget);
delegate_shm!(DesktopWidget);
delegate_layer!(DesktopWidget);
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
    tracing::info!("Configuration loaded: {:?}", config);

    // Connect to Wayland
    let conn = Connection::connect_to_env()
        .context("Failed to connect to Wayland compositor. Is a Wayland compositor running?")?;

    tracing::info!("Connected to Wayland");

    let (globals, event_queue) = registry_queue_init(&conn)
        .context("Failed to initialize Wayland registry")?;
    let qh = event_queue.handle();

    // Initialize Wayland states
    let registry_state = RegistryState::new(&globals);
    let output_state = OutputState::new(&globals, &qh);
    let compositor_state = CompositorState::bind(&globals, &qh)
        .context("wl_compositor protocol not available. Your compositor may not support required Wayland protocols.")?;
    let shm_state = Shm::bind(&globals, &qh)
        .context("wl_shm protocol not available. Shared memory buffers are required.")?;
    let layer_shell = LayerShell::bind(&globals, &qh)
        .context("zwlr_layer_shell_v1 not available. Your compositor must support the Layer Shell protocol.")?;

    let mut widget = DesktopWidget::new(
        registry_state,
        output_state,
        compositor_state,
        shm_state,
        layer_shell,
        config,
    );

    // Create the layer surface
    widget.create_layer_surface(&qh);

    // Setup event loop
    let mut event_loop = calloop::EventLoop::<DesktopWidget>::try_new()
        .context("Failed to create event loop")?;

    // Add Wayland source using WaylandSource
    WaylandSource::new(conn.clone(), event_queue)
        .insert(event_loop.handle())
        .context("Failed to insert Wayland event source into event loop")?;

    // Timer for periodic updates - updates happen during draw() calls
    // This timer just ensures we wake up to check for updates
    let timer = calloop::timer::Timer::from_duration(Duration::from_millis(100));
    event_loop
        .handle()
        .insert_source(timer, |_deadline, _metadata, widget| {
            // The actual update logic is in draw() which checks the scheduler
            // This just ensures we wake up periodically
            // Calculate time until next check
            let next_check = widget.update_scheduler.time_until_next_update();
            calloop::timer::TimeoutAction::ToDuration(next_check.max(Duration::from_millis(100)))
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
    // Add error recovery to prevent crashes on transient errors
    loop {
        if let Err(e) = event_loop.dispatch(Duration::from_millis(16), &mut widget) {
            tracing::error!(error = %e, "Event loop dispatch error");
            // For critical errors, we should exit
            // For transient errors, we could continue
            // Currently, we exit on any error as most are fatal
            return Err(e.into());
        }
    }
}
