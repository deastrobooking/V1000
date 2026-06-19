//! A single clip placed on a track.

use v1000_core::Time;

/// A reference to a span of source media, placed at a position on a track.
///
/// The clip shows `[source_in, source_in + duration)` of its media, positioned
/// at `[timeline_start, timeline_start + duration)` on the sequence. All four
/// quantities are exact [`Time`] values, so edit math never drifts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clip {
    /// Index into the owning [`Sequence`](crate::Sequence)'s media pool.
    pub media: usize,
    /// Where the clip begins on the sequence timeline.
    pub timeline_start: Time,
    /// In-point within the source media.
    pub source_in: Time,
    /// Length of the clip on the timeline (and of the media span shown).
    pub duration: Time,
}

impl Clip {
    /// Creates a clip.
    pub fn new(media: usize, timeline_start: Time, source_in: Time, duration: Time) -> Self {
        Self {
            media,
            timeline_start,
            source_in,
            duration,
        }
    }

    /// The (exclusive) end of the clip on the timeline.
    pub fn end(&self) -> Time {
        self.timeline_start + self.duration
    }

    /// Whether the clip covers timeline time `t` (half-open `[start, end)`).
    pub fn covers(&self, t: Time) -> bool {
        self.timeline_start <= t && t < self.end()
    }

    /// The source time corresponding to timeline time `t` (assumes [`covers`]).
    ///
    /// [`covers`]: Clip::covers
    pub fn source_time_at(&self, t: Time) -> Time {
        self.source_in + (t - self.timeline_start)
    }
}
