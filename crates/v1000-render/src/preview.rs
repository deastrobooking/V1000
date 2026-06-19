//! Preview transport: the playback clock.
//!
//! The transport is intentionally decoupled from the media. It tracks only the
//! playhead and play state; the caller owns the [`FrameProducer`] (a timeline
//! `Sequence`) and asks it for the frame at [`Transport::playhead_time`]. That
//! keeps this crate free of any timeline dependency while the preview reads
//! from the sequence.
//!
//! [`FrameProducer`]: v1000_codec::FrameProducer

use v1000_core::Time;

/// The playback clock: a playhead in seconds, a play/pause flag, and the total
/// duration it loops within.
///
/// The playhead is a floating-point seconds accumulator because it is advanced
/// by wall-clock frame delta-time; convert it to an exact [`Time`] with
/// [`Transport::playhead_time`] when addressing media. Edit points themselves
/// never go through this float path — they are authored as exact `Time`.
#[derive(Debug, Clone)]
pub struct Transport {
    playhead: f64,
    duration: f64,
    playing: bool,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            playhead: 0.0,
            duration: 0.0,
            playing: false,
        }
    }
}

impl Transport {
    /// A paused transport at time zero with no duration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the total duration (e.g. from the sequence), clamping the playhead.
    pub fn set_duration(&mut self, duration: Time) {
        self.duration = duration.as_seconds_f64().max(0.0);
        if self.playhead > self.duration {
            self.playhead = self.duration;
        }
    }

    /// Advances the playhead by `dt` seconds when playing, looping at the end.
    pub fn tick(&mut self, dt: f64) {
        if !self.playing || self.duration <= 0.0 {
            return;
        }
        self.playhead += dt.max(0.0);
        if self.playhead >= self.duration {
            self.playhead %= self.duration;
        }
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

    /// Moves the playhead to an absolute time in seconds (clamped).
    pub fn seek_seconds(&mut self, seconds: f64) {
        self.playhead = seconds.clamp(0.0, self.duration);
    }

    /// Moves the playhead to a fraction `[0, 1]` of the duration.
    pub fn seek_fraction(&mut self, fraction: f64) {
        self.seek_seconds(fraction.clamp(0.0, 1.0) * self.duration);
    }

    /// Current playhead position in seconds.
    pub fn playhead_seconds(&self) -> f64 {
        self.playhead
    }

    /// Current playhead as an exact [`Time`], for addressing media.
    pub fn playhead_time(&self) -> Time {
        Time::from_seconds_f64(self.playhead)
    }

    /// Total duration in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v1000_core::Rational;

    fn three_seconds() -> Transport {
        let mut tr = Transport::new();
        tr.set_duration(Time::from_frame(90, Rational::new(30, 1))); // 3.0s
        tr
    }

    #[test]
    fn paused_does_not_advance() {
        let mut tr = three_seconds();
        tr.tick(1.0);
        assert_eq!(tr.playhead_seconds(), 0.0);
    }

    #[test]
    fn playing_advances_and_maps_to_frame() {
        let mut tr = three_seconds();
        tr.play();
        tr.tick(1.0);
        assert_eq!(tr.playhead_time().to_frame(Rational::new(30, 1)), 30);
    }

    #[test]
    fn playback_loops_at_end() {
        let mut tr = three_seconds();
        tr.play();
        tr.tick(3.5);
        assert!(tr.playhead_seconds() < 3.0, "should have wrapped");
    }

    #[test]
    fn seek_fraction_maps_to_time() {
        let mut tr = three_seconds();
        tr.seek_fraction(0.5);
        assert!((tr.playhead_seconds() - 1.5).abs() < 1e-9);
    }

    #[test]
    fn shrinking_duration_clamps_playhead() {
        let mut tr = three_seconds();
        tr.seek_seconds(2.5);
        tr.set_duration(Time::from_frame(30, Rational::new(30, 1))); // 1.0s
        assert_eq!(tr.playhead_seconds(), 1.0);
    }
}
