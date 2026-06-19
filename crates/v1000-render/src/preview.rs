//! Preview transport: maps a wall-clock playhead onto frames from a source.

use std::sync::Arc;

use v1000_codec::{FrameSource, SourceError};
use v1000_core::{Frame, TimeCode};

/// Drives playback over a [`FrameSource`].
///
/// Holds a floating-point playhead in seconds advanced by `tick(dt)`, and maps
/// it to a frame index on demand. The GPU upload of the produced frame is the
/// GUI's responsibility this milestone; the wgpu device and render graph this
/// crate will eventually own arrive with M3.
///
/// Playback loops at the end of the source so the preview runs continuously.
pub struct PreviewEngine {
    source: Box<dyn FrameSource>,
    playhead: f64,
    playing: bool,
}

impl PreviewEngine {
    /// Wraps a frame source, paused at the start.
    pub fn new(source: Box<dyn FrameSource>) -> Self {
        Self {
            source,
            playhead: 0.0,
            playing: false,
        }
    }

    /// Replaces the source (e.g. after opening a file) and rewinds to the start.
    pub fn set_source(&mut self, source: Box<dyn FrameSource>) {
        self.source = source;
        self.playhead = 0.0;
    }

    /// Advances the playhead by `dt` seconds when playing, looping at the end.
    pub fn tick(&mut self, dt: f64) {
        if !self.playing {
            return;
        }
        let duration = self.duration_seconds();
        if duration <= 0.0 {
            return;
        }
        self.playhead += dt.max(0.0);
        if self.playhead >= duration {
            self.playhead %= duration;
        }
    }

    /// The frame index the playhead currently lands on, clamped to the source.
    pub fn current_index(&self) -> u64 {
        let (num, den) = self.source.fps();
        let idx = TimeCode::at_seconds(self.playhead.max(0.0), num, den).frames();
        idx.min(self.source.frame_count().saturating_sub(1))
    }

    /// Produces the frame at the current playhead.
    ///
    /// # Errors
    /// Propagates any [`SourceError`] from the underlying source.
    pub fn current_frame(&mut self) -> Result<Arc<Frame>, SourceError> {
        let index = self.current_index();
        self.source.frame(index)
    }

    /// Starts playback.
    pub fn play(&mut self) {
        self.playing = true;
    }

    /// Pauses playback.
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Toggles play/pause.
    pub fn toggle(&mut self) {
        self.playing = !self.playing;
    }

    /// Whether playback is running.
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Moves the playhead to an absolute time in seconds (clamped to `[0, dur]`).
    pub fn seek_seconds(&mut self, seconds: f64) {
        self.playhead = seconds.clamp(0.0, self.duration_seconds());
    }

    /// Moves the playhead to a fraction `[0, 1]` of the total duration.
    pub fn seek_fraction(&mut self, fraction: f64) {
        self.seek_seconds(fraction.clamp(0.0, 1.0) * self.duration_seconds());
    }

    /// Current playhead position in seconds.
    pub fn playhead_seconds(&self) -> f64 {
        self.playhead
    }

    /// Total duration of the source in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.source.duration_seconds()
    }

    /// Source dimensions as `(width, height)`.
    pub fn dimensions(&self) -> (u32, u32) {
        self.source.dimensions()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v1000_codec::TestPatternSource;

    fn engine() -> PreviewEngine {
        // 30 fps, 90 frames => 3.0 s.
        PreviewEngine::new(Box::new(TestPatternSource::new(64, 36, 30, 1, 90)))
    }

    #[test]
    fn paused_does_not_advance() {
        let mut e = engine();
        e.tick(1.0);
        assert_eq!(e.playhead_seconds(), 0.0);
        assert_eq!(e.current_index(), 0);
    }

    #[test]
    fn playing_advances_index() {
        let mut e = engine();
        e.play();
        e.tick(1.0); // 1s @ 30fps -> frame 30
        assert_eq!(e.current_index(), 30);
    }

    #[test]
    fn playback_loops_at_end() {
        let mut e = engine();
        e.play();
        e.tick(3.5); // past the 3.0s duration
        assert!(e.playhead_seconds() < 3.0, "should have wrapped");
    }

    #[test]
    fn seek_fraction_maps_to_time() {
        let mut e = engine();
        e.seek_fraction(0.5);
        assert!((e.playhead_seconds() - 1.5).abs() < 1e-9);
        assert_eq!(e.current_index(), 45);
    }

    #[test]
    fn index_is_clamped_to_last_frame() {
        let mut e = engine();
        e.seek_seconds(100.0); // clamps to duration
        assert_eq!(e.current_index(), 89);
    }
}
