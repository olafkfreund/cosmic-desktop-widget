// Wayland buffer pool for shared memory rendering

use anyhow::{Context, Result};
use smithay_client_toolkit::shm::{Shm, slot::SlotPool};
use wayland_client::{protocol::wl_buffer::WlBuffer, QueueHandle};
use std::io::Write;

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
        qh: &QueueHandle<T>,
    ) -> Result<Self>
    where
        T: 'static,
    {
        let pool = SlotPool::new(
            (width * height * 4) as usize * 2, // Double buffering
            shm,
        )
        .context("Failed to create slot pool")?;

        Ok(Self {
            pool,
            width,
            height,
        })
    }

    pub fn get_buffer(&mut self) -> Result<(&WlBuffer, &mut [u8])> {
        let stride = self.width * 4;
        let size = stride * self.height;

        let (buffer, canvas) = self
            .pool
            .create_buffer(
                self.width as i32,
                self.height as i32,
                stride as i32,
                wayland_client::protocol::wl_shm::Format::Argb8888,
            )
            .context("Failed to create buffer")?;

        // Clear canvas
        canvas.chunks_exact_mut(4).for_each(|chunk| {
            chunk.copy_from_slice(&[0, 0, 0, 0]); // Transparent
        });

        Ok((buffer, canvas))
    }
}
