// Wayland buffer pool for shared memory rendering
//
// Performance optimizations:
// - Conditional buffer clearing (only when needed)
// - Buffer slot tracking for efficient reuse
// - Reduced debug logging in hot paths

use crate::error::Result;
use smithay_client_toolkit::shm::{
    slot::{Buffer, SlotPool},
    Shm,
};
use tracing::{debug, info, trace};
use wayland_client::QueueHandle;

/// Buffer pool for Wayland shared memory surfaces
///
/// Manages double-buffered rendering for smooth updates.
/// Optimized for minimal allocations and efficient reuse.
pub struct BufferPool {
    pool: SlotPool,
    width: u32,
    height: u32,
    /// Track whether we need to clear the buffer
    needs_clear: bool,
    /// Number of buffers created (for metrics)
    buffer_count: u64,
}

impl BufferPool {
    /// Create a new buffer pool for the given dimensions
    ///
    /// Allocates enough space for double-buffering (2x buffer size).
    pub fn new<T>(width: u32, height: u32, shm: &Shm, _qh: &QueueHandle<T>) -> Result<Self>
    where
        T: 'static,
    {
        let buffer_size = (width * height * 4) as usize * 2; // Double buffering
        info!(
            width = %width,
            height = %height,
            buffer_size_bytes = %buffer_size,
            buffer_size_kb = %(buffer_size / 1024),
            "Creating buffer pool"
        );

        let pool = SlotPool::new(buffer_size, shm).map_err(|e| {
            crate::error::WidgetError::BufferCreation(format!("Failed to create slot pool: {}", e))
        })?;

        debug!("Buffer pool created successfully");

        Ok(Self {
            pool,
            width,
            height,
            needs_clear: true, // First buffer always needs clearing
            buffer_count: 0,
        })
    }

    /// Get a buffer for rendering
    ///
    /// The buffer is only cleared if `needs_clear` is true or `force_clear` is requested.
    /// After the first frame, clearing is typically not needed since the renderer
    /// overwrites the entire buffer content.
    pub fn get_buffer(&mut self) -> Result<(Buffer, &mut [u8])> {
        self.get_buffer_with_clear(self.needs_clear)
    }

    /// Get a buffer with explicit clear control
    ///
    /// Use this when you need fine-grained control over buffer clearing.
    pub fn get_buffer_with_clear(&mut self, clear: bool) -> Result<(Buffer, &mut [u8])> {
        let stride = self.width * 4;

        trace!(
            width = %self.width,
            height = %self.height,
            stride = %stride,
            clear = %clear,
            "Retrieving buffer from pool"
        );

        let (buffer, canvas): (Buffer, &mut [u8]) = self
            .pool
            .create_buffer(
                self.width as i32,
                self.height as i32,
                stride as i32,
                wayland_client::protocol::wl_shm::Format::Argb8888,
            )
            .map_err(|e| {
                crate::error::WidgetError::BufferCreation(format!("Failed to create buffer: {}", e))
            })?;

        // Only clear if needed - the renderer typically overwrites everything
        if clear {
            // Use a more efficient clearing method
            // For large buffers, memset-style operations are faster
            clear_buffer_fast(canvas);
            trace!("Buffer cleared");
        }

        self.buffer_count += 1;
        // After first buffer, we typically don't need to clear
        // The renderer draws the entire surface
        self.needs_clear = false;

        Ok((buffer, canvas))
    }

    /// Mark that the next buffer should be cleared
    ///
    /// Call this when the surface size changes or on theme changes.
    pub fn request_clear(&mut self) {
        self.needs_clear = true;
    }

    /// Get current buffer dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Resize the buffer pool for new dimensions
    ///
    /// This invalidates all existing buffers.
    pub fn resize<T>(
        &mut self,
        width: u32,
        height: u32,
        shm: &Shm,
        qh: &QueueHandle<T>,
    ) -> Result<()>
    where
        T: 'static,
    {
        if self.width == width && self.height == height {
            return Ok(());
        }

        debug!(
            old_width = %self.width,
            old_height = %self.height,
            new_width = %width,
            new_height = %height,
            "Resizing buffer pool"
        );

        // Create new pool with new size
        let new_pool = Self::new(width, height, shm, qh)?;
        self.pool = new_pool.pool;
        self.width = width;
        self.height = height;
        self.needs_clear = true; // New buffers need clearing

        Ok(())
    }

    /// Get the total number of buffers created
    pub fn buffer_count(&self) -> u64 {
        self.buffer_count
    }

    /// Get the buffer size in bytes
    pub fn buffer_size_bytes(&self) -> usize {
        (self.width * self.height * 4) as usize
    }
}

/// Fast buffer clearing using efficient memory operations
///
/// This is optimized for clearing ARGB8888 buffers to transparent black.
#[inline]
fn clear_buffer_fast(canvas: &mut [u8]) {
    // For small buffers, simple iteration is fine
    // For larger buffers, using fill is often optimized by the compiler
    // to use SIMD or memset
    canvas.fill(0);
}
