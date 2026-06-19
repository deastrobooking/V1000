//! A sequence: the editable composition the preview reads from.

use std::sync::Arc;

use v1000_codec::{FrameProducer, FrameSource, SourceError};
use v1000_core::{Frame, Rational, Time};

use crate::track::Track;

/// A composition of tracks over a shared media pool, rendered at a fixed
/// timebase and resolution.
///
/// Tracks are ordered bottom-to-top; the topmost track with a clip under the
/// playhead wins (multi-track blending arrives with the compositor in M4). The
/// sequence owns its media sources so several clips can reference the same
/// source by index.
pub struct Sequence {
    /// Display name.
    pub name: String,
    timebase: Rational,
    width: u32,
    height: u32,
    media: Vec<Box<dyn FrameSource>>,
    tracks: Vec<Track>,
}

impl Sequence {
    /// Creates an empty sequence with one video track.
    ///
    /// # Panics
    /// Panics if `timebase` is not positive or a dimension is zero.
    pub fn new(name: impl Into<String>, timebase: Rational, width: u32, height: u32) -> Self {
        assert!(timebase.numerator() > 0, "timebase must be positive");
        assert!(width != 0 && height != 0, "dimensions must be non-zero");
        Self {
            name: name.into(),
            timebase,
            width,
            height,
            media: Vec::new(),
            tracks: vec![Track::default()],
        }
    }

    /// Builds a sequence that plays a single source full-length on one track,
    /// adopting the source's rate and dimensions as the sequence settings.
    pub fn single(source: Box<dyn FrameSource>) -> Self {
        let (width, height) = source.dimensions();
        let (num, den) = source.fps();
        let timebase = Rational::new(num as i64, den as i64);
        let duration = Time::from_frame(source.frame_count() as i64, timebase);

        let mut seq = Sequence::new("Sequence", timebase, width, height);
        let id = seq.add_media(source);
        seq.tracks[0].append(id, Time::ZERO, duration);
        seq
    }

    /// Adds a media source to the pool and returns its id (for [`Clip::media`]).
    ///
    /// [`Clip::media`]: crate::Clip::media
    pub fn add_media(&mut self, source: Box<dyn FrameSource>) -> usize {
        self.media.push(source);
        self.media.len() - 1
    }

    /// The render/display rate.
    pub fn timebase(&self) -> Rational {
        self.timebase
    }

    /// Render dimensions as `(width, height)`.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// All tracks, bottom-to-top.
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Mutable access to track `index`.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn track_mut(&mut self, index: usize) -> &mut Track {
        &mut self.tracks[index]
    }

    /// Adds an empty track on top and returns its index.
    pub fn add_track(&mut self) -> usize {
        self.tracks.push(Track::default());
        self.tracks.len() - 1
    }

    /// The frame rate of media `id`, as a [`Rational`].
    fn media_rate(media: &dyn FrameSource) -> Rational {
        let (num, den) = media.fps();
        Rational::new(num as i64, den as i64)
    }
}

impl FrameProducer for Sequence {
    fn duration(&self) -> Time {
        self.tracks
            .iter()
            .map(Track::end)
            .max()
            .unwrap_or(Time::ZERO)
    }

    fn frame_at(&mut self, time: Time) -> Result<Option<Arc<Frame>>, SourceError> {
        // Resolve which media and source-time, topmost track first, with only
        // an immutable borrow — then drop it before decoding.
        let hit = self.tracks.iter().rev().find_map(|track| {
            track
                .clip_at(time)
                .map(|c| (c.media, c.source_time_at(time)))
        });

        let Some((media_id, source_time)) = hit else {
            return Ok(None);
        };

        let source = &mut self.media[media_id];
        let index = source_time.to_frame(Sequence::media_rate(source.as_ref()));
        if index < 0 || index as u64 >= source.frame_count() {
            return Ok(None);
        }
        Ok(Some(source.frame(index as u64)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v1000_codec::TestPatternSource;

    #[test]
    fn single_source_sequence_spans_the_media() {
        let src = TestPatternSource::new(64, 36, 30, 1, 90); // 3 seconds
        let mut seq = Sequence::single(Box::new(src));
        assert_eq!(seq.size(), (64, 36));
        assert_eq!(seq.duration(), Time::from_frame(90, Rational::new(30, 1)));
        assert!(seq
            .frame_at(Time::from_frame(0, Rational::new(30, 1)))
            .unwrap()
            .is_some());
        assert!(seq
            .frame_at(Time::from_frame(45, Rational::new(30, 1)))
            .unwrap()
            .is_some());
    }

    #[test]
    fn gap_after_end_yields_no_frame() {
        let src = TestPatternSource::new(64, 36, 30, 1, 30); // 1 second
        let mut seq = Sequence::single(Box::new(src));
        let past_end = Time::from_frame(60, Rational::new(30, 1));
        assert!(seq.frame_at(past_end).unwrap().is_none());
    }

    #[test]
    fn second_clip_reads_from_its_source_in_point() {
        // Two clips from the same 90-frame source; the second starts at the
        // playhead t=10f but reads from source in-point 50f.
        let src = TestPatternSource::new(64, 36, 30, 1, 90);
        let mut seq = Sequence::new("s", Rational::new(30, 1), 64, 36);
        let id = seq.add_media(Box::new(src));
        let fps = Rational::new(30, 1);
        seq.track_mut(0)
            .append(id, Time::ZERO, Time::from_frame(10, fps));
        seq.track_mut(0)
            .append(id, Time::from_frame(50, fps), Time::from_frame(10, fps));

        // Reference: directly decode source frame 55.
        let mut probe = TestPatternSource::new(64, 36, 30, 1, 90);
        let expected = probe.frame(55).unwrap();

        // Timeline t=15f lands in clip 2, 5f past its start, source 50+5 = 55.
        let got = seq.frame_at(Time::from_frame(15, fps)).unwrap().unwrap();
        assert_eq!(got.pixels(), expected.pixels());
    }
}
