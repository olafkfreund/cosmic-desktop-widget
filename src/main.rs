// COSMIC Desktop Widget - Wayland Layer Shell Implementation
// A true desktop widget that lives on your desktop background

use anyhow::Result;
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
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, QueueHandle,
};

mod config;
mod render;
mod wayland;
mod widget;

use config::Config;
use render::Renderer;
use widget::{ClockWidget, WeatherWidget};

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
    
    // Configuration
    config: Config,
    
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
        Self {
            registry_state,
            output_state,
            compositor_state,
            shm_state,
            layer_shell,
            layer: None,
            renderer: Renderer::new(),
            width: config.width,
            height: config.height,
            buffer_pool: None,
            clock_widget: ClockWidget::new(),
            weather_widget: WeatherWidget::new(&config.weather_city, &config.weather_api_key),
            config,
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

        // Update widgets
        self.clock_widget.update();
        self.weather_widget.update();

        // Create buffer pool if needed
        if self.buffer_pool.is_none() {
            self.buffer_pool = Some(
                wayland::BufferPool::new(
                    self.width,
                    self.height,
                    &self.shm_state,
                    qh,
                )
                .expect("Failed to create buffer pool"),
            );
        }

        let pool = self.buffer_pool.as_mut().unwrap();
        let (buffer, canvas) = pool.get_buffer().expect("Failed to get buffer");

        // Render the widget
        self.renderer.render(
            canvas,
            self.width,
            self.height,
            &self.clock_widget,
            &self.weather_widget,
            &self.config,
        );

        // Attach buffer and commit
        layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);
        layer.wl_surface().attach(Some(buffer), 0, 0);
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
        .expect("Failed to connect to Wayland compositor");
    
    tracing::info!("Connected to Wayland");

    let (globals, mut event_queue) = registry_queue_init(&conn)
        .expect("Failed to initialize registry");
    let qh = event_queue.handle();

    // Initialize Wayland states
    let registry_state = RegistryState::new(&globals);
    let output_state = OutputState::new(&globals, &qh);
    let compositor_state = CompositorState::bind(&globals, &qh)
        .expect("wl_compositor not available");
    let shm_state = Shm::bind(&globals, &qh)
        .expect("wl_shm not available");
    let layer_shell = LayerShell::bind(&globals, &qh)
        .expect("layer_shell not available");

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
        .expect("Failed to create event loop");

    // Add Wayland source
    let _wayland_source = calloop::generic::Generic::new(
        event_queue.display().get_connection_fd(),
        calloop::Interest::READ,
        calloop::Mode::Level,
    );

    // Timer for periodic updates (every second)
    let timer = calloop::timer::Timer::new()
        .expect("Failed to create timer");
    let timer_handle = timer.handle();
    event_loop
        .handle()
        .insert_source(timer, |_deadline, _metadata, widget| {
            widget.clock_widget.update();
            // Trigger redraw if needed
            calloop::timer::TimeoutAction::ToDuration(std::time::Duration::from_secs(1))
        })
        .expect("Failed to insert timer");
    
    timer_handle.add_timeout(std::time::Duration::from_secs(1), ());

    // Signal handling for graceful shutdown
    let signals = calloop::signals::Signals::new(&[calloop::signals::Signal::SIGINT])
        .expect("Failed to create signal source");
    event_loop
        .handle()
        .insert_source(signals, |_signal, _metadata, _widget| {
            tracing::info!("Received SIGINT, exiting");
            std::process::exit(0);
        })
        .expect("Failed to insert signal source");

    tracing::info!("Event loop starting");

    // Main loop
    loop {
        // Dispatch Wayland events
        event_queue.blocking_dispatch(&mut widget)
            .expect("Failed to dispatch events");

        // Dispatch event loop
        event_loop
            .dispatch(std::time::Duration::from_millis(16), &mut widget)
            .expect("Failed to dispatch event loop");
    }
}
