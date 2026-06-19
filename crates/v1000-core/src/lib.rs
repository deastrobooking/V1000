//! Core types shared across the V1000 engine.
//!
//! This crate sits at the bottom of the workspace dependency graph and depends
//! on no other workspace crate. Everything here must stay deterministic: given
//! the same inputs the engine must produce identical output, so wall-clock
//! time, RNG, and I/O do not belong on the render path.
//!
//! As of milestone M2 this exposes the canonical time model ([`Time`],
//! [`Rational`]), a display-only [`TimeCode`] formatter, the [`Frame`] pixel
//! type, and the playback support types [`FramePool`] and [`FrameCache`]. The
//! color pipeline and processing graph land in later milestones.

mod cache;
mod frame;
mod pool;
mod time;

pub use cache::FrameCache;
pub use frame::Frame;
pub use pool::FramePool;
pub use time::{Rational, Time};

/// A SMPTE-style timecode (`HH:MM:SS:FF`) for **display only**.
///
/// This is a labeled frame index at a specific rate, not a comparable instant —
/// two timecodes at different rates that name the same moment are different
/// *labels*. For ordering and edit math use [`Time`], which is rate-agnostic
/// and exact; build a `TimeCode` with [`TimeCode::from_time`] only when
/// rendering a position to the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeCode {
    frames: u64,
    fps_num: u32,
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

    /// The timecode label for an instant at the display rate `fps`.
    ///
    /// Negative instants clamp to zero (the timeline starts at zero).
    ///
    /// # Panics
    /// Panics if `fps` is not positive.
    pub fn from_time(time: Time, fps: Rational) -> Self {
        assert!(fps.numerator() > 0, "frame rate must be positive");
        let frames = time.to_frame(fps).max(0) as u64;
        Self {
            frames,
            fps_num: fps.numerator() as u32,
            fps_den: fps.denominator() as u32,
        }
    }

    /// The frame index this label carries.
    pub fn frames(self) -> u64 {
        self.frames
    }

    /// Nominal whole-frame rate used to split frames into the `:FF` field.
    fn nominal_fps(self) -> u64 {
        ((self.fps_num as f64 / self.fps_den as f64).round() as u64).max(1)
    }
}

impl std::fmt::Display for TimeCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fps = self.nominal_fps();
        let ff = self.frames % fps;
        let total_secs = self.frames / fps;
        let ss = total_secs % 60;
        let mm = (total_secs / 60) % 60;
        let hh = total_secs / 3600;
        write!(f, "{hh:02}:{mm:02}:{ss:02}:{ff:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timecode_formats_smpte() {
        // 1 second + 5 frames at 30 fps.
        let t = Time::from_frame(35, Rational::new(30, 1));
        let tc = TimeCode::from_time(t, Rational::new(30, 1));
        assert_eq!(tc.to_string(), "00:00:01:05");
        assert_eq!(tc.frames(), 35);
    }

    #[test]
    fn timecode_rolls_over_minutes_and_hours() {
        let fps = Rational::new(25, 1);
        let one_hour = Time::from_frame(25 * 3600, fps);
        assert_eq!(
            TimeCode::from_time(one_hour, fps).to_string(),
            "01:00:00:00"
        );
    }
}
