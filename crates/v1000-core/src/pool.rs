//! Frame buffer recycling.
//!
//! Decoding allocates a new RGBA8 buffer per frame, which churns the allocator
//! during playback. [`FramePool`] recycles the backing `Vec<u8>` of dropped
//! frames so steady-state playback stops allocating.

use crate::frame::Frame;

/// A pool of reusable pixel buffers.
///
/// Buffers are reused by capacity: [`acquire`](FramePool::acquire) pops any
/// stored buffer large enough and resizes it, falling back to a fresh
/// allocation. [`release`](FramePool::release) returns a frame's buffer to the
/// pool. Not internally synchronized — wrap in a lock if shared across threads.
#[derive(Default)]
pub struct FramePool {
    free: Vec<Vec<u8>>,
    capacity: usize,
}

impl FramePool {
    /// Creates an empty pool that retains at most `capacity` free buffers.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            free: Vec::new(),
            capacity,
        }
    }

    /// Returns a zeroed frame of the requested size, reusing a pooled buffer
    /// when one is available.
    pub fn acquire(&mut self, width: u32, height: u32) -> Frame {
        let needed = Frame::byte_len(width, height);
        match self.free.pop() {
            Some(mut buf) => {
                buf.clear();
                buf.resize(needed, 0);
                Frame::from_pixels(width, height, buf)
            }
            None => Frame::new(width, height),
        }
    }

    /// Returns a frame's backing buffer to the pool for reuse.
    ///
    /// Dropped if the pool is already holding `capacity` buffers, so memory
    /// stays bounded.
    pub fn release(&mut self, frame: Frame) {
        if self.free.len() < self.capacity {
            self.free.push(frame.into_pixels());
        }
    }

    /// Number of buffers currently available for reuse.
    pub fn available(&self) -> usize {
        self.free.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_then_acquire_reuses_buffer() {
        let mut pool = FramePool::with_capacity(4);
        let frame = pool.acquire(16, 16);
        let ptr = frame.pixels().as_ptr();
        pool.release(frame);
        assert_eq!(pool.available(), 1);

        let reused = pool.acquire(16, 16);
        assert_eq!(
            reused.pixels().as_ptr(),
            ptr,
            "should reuse the same allocation"
        );
        assert_eq!(pool.available(), 0);
    }

    #[test]
    fn pool_is_bounded() {
        let mut pool = FramePool::with_capacity(1);
        pool.release(Frame::new(8, 8));
        pool.release(Frame::new(8, 8));
        assert_eq!(pool.available(), 1, "excess buffers are dropped");
    }
}
