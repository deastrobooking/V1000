//! A track: an ordered, non-overlapping row of clips.

use v1000_core::Time;

use crate::clip::Clip;

/// A horizontal row of clips, kept sorted by start time and non-overlapping.
///
/// Edit operations preserve those invariants. M2 covers the essentials:
/// appending clips, locating the clip under the playhead, ripple-delete, and
/// ripple trims of a clip's head/tail.
#[derive(Debug, Default, Clone)]
pub struct Track {
    clips: Vec<Clip>,
}

impl Track {
    /// The clips on this track, in timeline order.
    pub fn clips(&self) -> &[Clip] {
        &self.clips
    }

    /// Whether the track has no clips.
    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }

    /// The (exclusive) end of the last clip, or [`Time::ZERO`] if empty.
    pub fn end(&self) -> Time {
        self.clips.last().map_or(Time::ZERO, Clip::end)
    }

    /// Appends a clip of `media[source_in..source_in+duration]` directly after
    /// the current last clip (no gap) and returns its index.
    pub fn append(&mut self, media: usize, source_in: Time, duration: Time) -> usize {
        let start = self.end();
        self.clips
            .push(Clip::new(media, start, source_in, duration));
        self.clips.len() - 1
    }

    /// The index of the clip covering timeline time `t`, if any.
    pub fn clip_index_at(&self, t: Time) -> Option<usize> {
        self.clips.iter().position(|c| c.covers(t))
    }

    /// The clip covering timeline time `t`, if any.
    pub fn clip_at(&self, t: Time) -> Option<&Clip> {
        self.clip_index_at(t).map(|i| &self.clips[i])
    }

    /// Removes the clip at `index` and shifts every later clip left by its
    /// duration, closing the gap. Returns the removed clip.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn ripple_delete(&mut self, index: usize) -> Clip {
        let removed = self.clips.remove(index);
        for clip in &mut self.clips[index..] {
            clip.timeline_start = clip.timeline_start - removed.duration;
        }
        removed
    }

    /// Trims the tail of clip `index` to `new_duration`, rippling all later
    /// clips by the change so the track stays gap-free and non-overlapping.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds or `new_duration <= 0`.
    pub fn trim_out(&mut self, index: usize, new_duration: Time) {
        assert!(new_duration > Time::ZERO, "clip duration must be positive");
        let delta = new_duration - self.clips[index].duration;
        self.clips[index].duration = new_duration;
        for clip in &mut self.clips[index + 1..] {
            clip.timeline_start = clip.timeline_start + delta;
        }
    }

    /// Trims the head of clip `index` by `amount` (advancing its in-point and
    /// shortening it), rippling all later clips so the track stays gap-free.
    ///
    /// `amount` may be negative to extend the head back toward the media start,
    /// bounded by the available source head.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds or the trim would leave a
    /// non-positive duration.
    pub fn trim_in(&mut self, index: usize, amount: Time) {
        let clip = self.clips[index];
        let new_duration = clip.duration - amount;
        assert!(new_duration > Time::ZERO, "clip duration must be positive");
        self.clips[index].source_in = clip.source_in + amount;
        self.clips[index].duration = new_duration;
        for later in &mut self.clips[index + 1..] {
            later.timeline_start = later.timeline_start - amount;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v1000_core::Rational;

    const FPS: Rational = Rational::from_int(30);

    fn t(frames: i64) -> Time {
        Time::from_frame(frames, FPS)
    }

    /// Track with three 10-frame clips back to back: [0,10) [10,20) [20,30).
    fn three_clips() -> Track {
        let mut track = Track::default();
        track.append(0, Time::ZERO, t(10));
        track.append(0, Time::ZERO, t(10));
        track.append(0, Time::ZERO, t(10));
        track
    }

    #[test]
    fn append_places_clips_back_to_back() {
        let track = three_clips();
        assert_eq!(track.clips()[1].timeline_start, t(10));
        assert_eq!(track.end(), t(30));
    }

    #[test]
    fn clip_lookup_is_half_open() {
        let track = three_clips();
        assert_eq!(track.clip_index_at(t(0)), Some(0));
        assert_eq!(track.clip_index_at(t(10)), Some(1)); // boundary belongs to next
        assert_eq!(track.clip_index_at(t(29)), Some(2));
        assert_eq!(track.clip_index_at(t(30)), None); // past the end
    }

    #[test]
    fn ripple_delete_closes_the_gap() {
        let mut track = three_clips();
        let removed = track.ripple_delete(1);
        assert_eq!(removed.duration, t(10));
        assert_eq!(track.clips().len(), 2);
        // The former third clip slides left into the gap: [0,10) [10,20).
        assert_eq!(track.clips()[1].timeline_start, t(10));
        assert_eq!(track.end(), t(20));
    }

    #[test]
    fn trim_out_ripples_following_clips() {
        let mut track = three_clips();
        track.trim_out(0, t(4)); // first clip 10 -> 4 frames
        assert_eq!(track.clips()[0].duration, t(4));
        assert_eq!(track.clips()[1].timeline_start, t(4));
        assert_eq!(track.end(), t(24));
    }

    #[test]
    fn trim_in_advances_source_and_ripples() {
        let mut track = three_clips();
        track.trim_in(0, t(3)); // drop first 3 frames of head
        assert_eq!(track.clips()[0].source_in, t(3));
        assert_eq!(track.clips()[0].duration, t(7));
        assert_eq!(track.clips()[1].timeline_start, t(7));
    }
}
