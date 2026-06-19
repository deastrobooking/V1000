//! Core types shared across the V1000 engine.
//!
//! This crate sits at the bottom of the workspace dependency graph and depends
//! on no other workspace crate. Everything here must stay deterministic: given
//! the same inputs the engine must produce identical output, so wall-clock
//! time, RNG, and I/O do not belong on the render path.
//!
//! At milestone M0 this is a stub exposing only [`TimeCode`]. Frame types, the
//! color pipeline, and the processing graph land in later milestones.

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
}
