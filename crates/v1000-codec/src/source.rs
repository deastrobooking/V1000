//! The frame-source abstraction every decoder (and synthetic generator)
//! implements, plus a built-in animated test pattern.

use std::sync::Arc;

use v1000_core::Frame;

/// Errors a [`FrameSource`] can return.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    /// The requested frame index is past the end of the source.
    #[error("frame index {index} out of range (0..{count})")]
    OutOfRange { index: u64, count: u64 },

    /// The underlying media could not be opened or read.
    #[error("media error: {0}")]
    Media(String),
}

/// A source of decoded RGBA8 frames addressed by frame index.
///
/// Implemented by both real decoders (e.g. the FFmpeg-backed file decoder) and
/// synthetic generators like [`TestPatternSource`]. The preview engine treats
/// them uniformly, so playback, scrubbing, and caching are decoder-agnostic.
pub trait FrameSource {
    /// Frame rate as `(numerator, denominator)`.
    fn fps(&self) -> (u32, u32);

    /// Total number of frames in the source.
    fn frame_count(&self) -> u64;

    /// Pixel dimensions as `(width, height)`.
    fn dimensions(&self) -> (u32, u32);

    /// Produces the frame at `index`, returning a shared handle.
    ///
    /// # Errors
    /// Returns [`SourceError::OutOfRange`] if `index >= frame_count()`, or
    /// [`SourceError::Media`] if decoding fails.
    fn frame(&mut self, index: u64) -> Result<Arc<Frame>, SourceError>;

    /// Total duration in seconds, derived from frame count and rate.
    fn duration_seconds(&self) -> f64 {
        let (num, den) = self.fps();
        self.frame_count() as f64 * den as f64 / num as f64
    }
}

/// An animated SMPTE-style color-bar generator.
///
/// Procedural and deterministic: the same index always yields the same pixels,
/// with a bright vertical bar sweeping left-to-right so motion is visible
/// during playback. Used as the default preview source until a file is opened.
pub struct TestPatternSource {
    width: u32,
    height: u32,
    fps_num: u32,
    fps_den: u32,
    frame_count: u64,
}

impl TestPatternSource {
    /// 8 vertical bars, top-left origin (RGBA, opaque).
    const BARS: [[u8; 4]; 8] = [
        [192, 192, 192, 255], // gray
        [255, 255, 0, 255],   // yellow
        [0, 255, 255, 255],   // cyan
        [0, 255, 0, 255],     // green
        [255, 0, 255, 255],   // magenta
        [255, 0, 0, 255],     // red
        [0, 0, 255, 255],     // blue
        [16, 16, 16, 255],    // near-black
    ];

    /// Creates a generator of the given size, rate, and length.
    ///
    /// # Panics
    /// Panics if any dimension or rate component is zero.
    pub fn new(width: u32, height: u32, fps_num: u32, fps_den: u32, frame_count: u64) -> Self {
        assert!(width != 0 && height != 0, "dimensions must be non-zero");
        assert!(fps_num != 0 && fps_den != 0, "frame rate must be non-zero");
        Self {
            width,
            height,
            fps_num,
            fps_den,
            frame_count,
        }
    }

    /// A 640×360, 30 fps, 10-second default pattern.
    pub fn default_preview() -> Self {
        Self::new(640, 360, 30, 1, 300)
    }

    fn render(&self, index: u64) -> Frame {
        let w = self.width as usize;
        let h = self.height as usize;
        let mut frame = Frame::new(self.width, self.height);
        let px = frame.pixels_mut();

        let bar_w = (w / Self::BARS.len()).max(1);
        // Sweep column position advances 4 px/frame and wraps.
        let sweep = ((index as usize) * 4) % w;
        let sweep_half = (w / 80).max(1);

        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * Frame::BYTES_PER_PIXEL;
                let dist = x.abs_diff(sweep);
                let color = if dist <= sweep_half {
                    [255, 255, 255, 255] // bright sweep bar
                } else {
                    Self::BARS[(x / bar_w).min(Self::BARS.len() - 1)]
                };
                px[i..i + 4].copy_from_slice(&color);
            }
        }
        frame
    }
}

impl FrameSource for TestPatternSource {
    fn fps(&self) -> (u32, u32) {
        (self.fps_num, self.fps_den)
    }

    fn frame_count(&self) -> u64 {
        self.frame_count
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn frame(&mut self, index: u64) -> Result<Arc<Frame>, SourceError> {
        if index >= self.frame_count {
            return Err(SourceError::OutOfRange {
                index,
                count: self.frame_count,
            });
        }
        Ok(Arc::new(self.render(index)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_is_deterministic() {
        let mut src = TestPatternSource::new(64, 36, 30, 1, 100);
        let a = src.frame(10).unwrap();
        let b = src.frame(10).unwrap();
        assert_eq!(a.pixels(), b.pixels());
    }

    #[test]
    fn pattern_animates_between_frames() {
        let mut src = TestPatternSource::new(64, 36, 30, 1, 100);
        let a = src.frame(0).unwrap();
        let b = src.frame(5).unwrap();
        assert_ne!(a.pixels(), b.pixels(), "sweep should move the image");
    }

    #[test]
    fn out_of_range_is_rejected() {
        let mut src = TestPatternSource::new(64, 36, 30, 1, 10);
        assert!(matches!(src.frame(10), Err(SourceError::OutOfRange { .. })));
    }

    #[test]
    fn duration_matches_count_and_rate() {
        let src = TestPatternSource::new(64, 36, 30, 1, 90);
        assert!((src.duration_seconds() - 3.0).abs() < 1e-9);
    }
}
