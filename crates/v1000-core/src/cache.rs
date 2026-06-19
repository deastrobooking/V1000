//! LRU frame cache.

use std::num::NonZeroUsize;
use std::sync::Arc;

use lru::LruCache;

use crate::frame::Frame;

/// An LRU cache of decoded frames keyed by frame index.
///
/// Frames are stored as `Arc<Frame>` so the preview, scrubbing, and (later)
/// the processing graph can share a decoded frame without copying. When the
/// cache is full the least-recently-used frame is evicted.
pub struct FrameCache {
    inner: LruCache<u64, Arc<Frame>>,
}

impl FrameCache {
    /// Creates a cache holding up to `capacity` frames.
    ///
    /// # Panics
    /// Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).expect("cache capacity must be non-zero");
        Self {
            inner: LruCache::new(cap),
        }
    }

    /// Returns the cached frame at `index`, marking it most-recently-used.
    pub fn get(&mut self, index: u64) -> Option<Arc<Frame>> {
        self.inner.get(&index).cloned()
    }

    /// Inserts a frame and returns the shared handle, evicting the LRU entry if
    /// the cache is full.
    pub fn insert(&mut self, index: u64, frame: Frame) -> Arc<Frame> {
        let shared = Arc::new(frame);
        self.inner.put(index, Arc::clone(&shared));
        shared
    }

    /// Whether a frame at `index` is currently cached (without touching LRU order).
    pub fn contains(&self, index: u64) -> bool {
        self.inner.contains(&index)
    }

    /// Number of frames currently cached.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_least_recently_used() {
        let mut cache = FrameCache::new(2);
        cache.insert(0, Frame::new(2, 2));
        cache.insert(1, Frame::new(2, 2));
        // Touch 0 so 1 becomes the LRU entry.
        assert!(cache.get(0).is_some());
        cache.insert(2, Frame::new(2, 2));

        assert!(cache.contains(0));
        assert!(!cache.contains(1), "frame 1 should have been evicted");
        assert!(cache.contains(2));
    }

    #[test]
    fn insert_returns_shared_handle() {
        let mut cache = FrameCache::new(1);
        let handle = cache.insert(7, Frame::new(2, 2));
        assert_eq!(Arc::strong_count(&handle), 2, "cache + returned handle");
    }
}
