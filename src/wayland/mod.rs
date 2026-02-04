// Wayland buffer pool for shared memory rendering

use crate::error::Result;
use smithay_client_toolkit::shm::{Shm, slot::{Buffer, SlotPool}};
use tracing::{debug, info};
use wayland_client::QueueHandle;

pub struct BufferPool {
    pool: SlotPool,
    width: u32,
    height: u32,
}

impl BufferPool {
    pub fn new<T>(
        width: u32,
        height: u32,
        shm: &Shm,
        _qh: &QueueHandle<T>,
    ) -> Result<Self>
    where
        T: 'static,
    {
        let buffer_size = (width * height * 4) as usize * 2; // Double buffering
        info!(
            width = %width,
            height = %height,
            buffer_size = %buffer_size,
            "Creating buffer pool"
        );

        let pool = SlotPool::new(buffer_size, shm)
            .map_err(|e| crate::error::WidgetError::BufferCreation(format!("Failed to create slot pool: {}", e)))?;

        debug!("Buffer pool created successfully");

        Ok(Self {
            pool,
            width,
            height,
        })
    }

    pub fn get_buffer(&mut self) -> Result<(Buffer, &mut [u8])> {
        let stride = self.width * 4;

        debug!(
            width = %self.width,
            height = %self.height,
            stride = %stride,
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
            .map_err(|e| crate::error::WidgetError::BufferCreation(format!("Failed to create buffer: {}", e)))?;

        // Clear canvas
        canvas.chunks_exact_mut(4).for_each(|chunk: &mut [u8]| {
            chunk.copy_from_slice(&[0, 0, 0, 0]); // Transparent
        });

        debug!("Buffer retrieved and cleared");

        Ok((buffer, canvas))
    }
}
