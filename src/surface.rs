//! Individual widget surface management
//!
//! Each widget gets its own Layer Shell surface with independent:
//! - Position and anchoring
//! - Size (width/height)
//! - Opacity/transparency
//! - Buffer pool for rendering

use anyhow::Result;
use smithay_client_toolkit::{
    shell::wlr_layer::LayerSurface,
    shm::Shm,
};
use wayland_client::{
    protocol::wl_surface,
    QueueHandle,
};

use crate::position::Position;
use crate::wayland::BufferPool;

/// Represents a single widget's Layer Shell surface
pub struct WidgetSurface {
    /// Layer shell surface handle
    pub layer: LayerSurface,

    /// Wayland surface
    pub wl_surface: wl_surface::WlSurface,

    /// Buffer pool for this surface
    pub buffer_pool: Option<BufferPool>,

    /// Surface dimensions
    pub width: u32,
    pub height: u32,

    /// Whether the surface has been configured by the compositor
    pub configured: bool,

    /// Index of the widget this surface displays
    pub widget_index: usize,

    /// Position configuration
    pub position: Position,

    /// Opacity (0.0 = transparent, 1.0 = opaque)
    pub opacity: f32,

    /// Whether this is the first frame
    pub first_frame: bool,
}

impl WidgetSurface {
    /// Create a new widget surface (without buffer pool)
    pub fn new(
        layer: LayerSurface,
        wl_surface: wl_surface::WlSurface,
        width: u32,
        height: u32,
        widget_index: usize,
        position: Position,
        opacity: f32,
    ) -> Self {
        Self {
            layer,
            wl_surface,
            buffer_pool: None,
            width,
            height,
            configured: false,
            widget_index,
            position,
            opacity,
            first_frame: true,
        }
    }

    /// Initialize the buffer pool for this surface
    pub fn init_buffer_pool<T: 'static>(
        &mut self,
        shm_state: &Shm,
        qh: &QueueHandle<T>,
    ) -> Result<()> {
        let pool = BufferPool::new(self.width, self.height, shm_state, qh)?;
        self.buffer_pool = Some(pool);
        Ok(())
    }

    /// Check if the surface is ready to render
    pub fn is_ready(&self) -> bool {
        self.configured && self.buffer_pool.is_some()
    }

    /// Update surface size and mark buffer pool for recreation
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            // Drop old buffer pool - will be recreated on next draw
            self.buffer_pool = None;
        }
    }
}

impl Drop for WidgetSurface {
    fn drop(&mut self) {
        // Layer surface cleanup is automatic via smithay-client-toolkit
        tracing::debug!(
            widget_index = self.widget_index,
            position = %self.position,
            "Dropping widget surface"
        );
    }
}
