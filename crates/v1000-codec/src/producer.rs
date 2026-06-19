//! Time-addressed frame production.

use std::sync::Arc;

use v1000_core::{Frame, Time};

use crate::source::SourceError;

/// A source of frames addressed by **timeline time**, in contrast to
/// [`FrameSource`](crate::FrameSource) which is addressed by frame index.
///
/// This is the abstraction the preview reads. A timeline `Sequence` implements
/// it by resolving the time to a clip and the clip to a source frame; the
/// result is `None` where nothing is present (a gap).
///
/// No blanket `impl` over `FrameSource` is provided on purpose: a blanket impl
/// would, under Rust's coherence rules, block downstream concrete impls (like
/// `Sequence`'s). Wrap a bare source in a single-clip sequence instead.
pub trait FrameProducer {
    /// Total covered length; times in `[0, duration)` may yield a frame.
    fn duration(&self) -> Time;

    /// The frame visible at `time`, or `None` for a gap.
    ///
    /// # Errors
    /// Propagates a [`SourceError`] from the underlying media.
    fn frame_at(&mut self, time: Time) -> Result<Option<Arc<Frame>>, SourceError>;
}
