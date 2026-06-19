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

    /// Returns a zeroed frame of the requested size.
    ///
    /// Reuses a pooled buffer only when one already has enough capacity, so a
    /// request for a larger frame never forces a too-small buffer to
    /// reallocate (which matters with mixed resolutions). When nothing fits, a
    /// fresh buffer is allocated and the smaller ones stay pooled for smaller
    /// requests.
    pub fn acquire(&mut self, width: u32, height: u32) -> Frame {
        let needed = Frame::byte_len(width, height);
        match self.free.iter().position(|buf| buf.capacity() >= needed) {
            Some(pos) => {
                let mut buf = self.free.swap_remove(pos);
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

    #[test]
    fn larger_request_does_not_grab_a_too_small_buffer() {
        let mut pool = FramePool::with_capacity(4);
        pool.release(Frame::new(8, 8)); // small buffer pooled
                                        // Requesting a larger frame must not reuse (and realloc) the small one.
        let _large = pool.acquire(64, 64);
        assert_eq!(
            pool.available(),
            1,
            "small buffer stays for a small request"
        );
        // A small request reuses it.
        let _small = pool.acquire(8, 8);
        assert_eq!(pool.available(), 0);
    }
}
