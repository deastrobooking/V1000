//! Core types shared across the V1000 engine.
//!
//! This crate sits at the bottom of the workspace dependency graph and depends
//! on no other workspace crate. Everything here must stay deterministic: given
//! the same inputs the engine must produce identical output, so wall-clock
//! time, RNG, and I/O do not belong on the render path.
//!
//! As of milestone M1 this exposes [`TimeCode`], the [`Frame`] pixel type, and
//! the playback support types [`FramePool`] and [`FrameCache`]. The color
//! pipeline and processing graph land in later milestones.

mod cache;
mod frame;
mod pool;

pub use cache::FrameCache;
pub use frame::Frame;
pub use pool::FramePool;

/// A position or duration on a timeline, expressed as a whole number of frames
/// at a given frame rate.
///
/// Edit math uses rational frame counts rather than floating-point seconds to
/// avoid accumulating drift across long sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeCode {
    frames: u64,
    /// Frame-rate numerator (e.g. 24000 for 23.976 fps).
    fps_num: u32,
    /// Frame-rate denominator (e.g. 1001 for 23.976 fps).
    fps_den: u32,
}

impl TimeCode {
    /// Creates a timecode at `frames` for the rate `fps_num / fps_den`.
    ///
    /// # Panics
    /// Panics if either rate component is zero.
    pub fn new(frames: u64, fps_num: u32, fps_den: u32) -> Self {
        assert!(fps_num != 0 && fps_den != 0, "frame rate must be non-zero");
        Self {
            frames,
            fps_num,
            fps_den,
        }
    }

    /// The nearest whole-frame timecode to `seconds` at the rate `fps_num /
    /// fps_den`. Used to map a wall-clock playhead onto a frame index.
    ///
    /// # Panics
    /// Panics if either rate component is zero, or if `seconds` is negative.
    pub fn at_seconds(seconds: f64, fps_num: u32, fps_den: u32) -> Self {
        assert!(fps_num != 0 && fps_den != 0, "frame rate must be non-zero");
        assert!(seconds >= 0.0, "timecode seconds must be non-negative");
        let frames = (seconds * fps_num as f64 / fps_den as f64).round() as u64;
        Self {
            frames,
            fps_num,
            fps_den,
        }
    }

    /// The frame index.
    pub fn frames(self) -> u64 {
        self.frames
    }

    /// The position in seconds. Lossy — use only for display, never edit math.
    pub fn as_seconds(self) -> f64 {
        self.frames as f64 * self.fps_den as f64 / self.fps_num as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_from_frames() {
        let tc = TimeCode::new(48, 24, 1);
        assert!((tc.as_seconds() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn ordering_is_by_frame() {
        assert!(TimeCode::new(1, 24, 1) < TimeCode::new(2, 24, 1));
    }

    #[test]
    fn at_seconds_rounds_to_nearest_frame() {
        assert_eq!(TimeCode::at_seconds(1.0, 30, 1).frames(), 30);
        // 1.51s @ 30fps = frame 45.3 -> 45
        assert_eq!(TimeCode::at_seconds(1.51, 30, 1).frames(), 45);
        assert_eq!(TimeCode::at_seconds(0.0, 30, 1).frames(), 0);
    }
}
