//! Timeline model for V1000.
//!
//! The editable composition: a [`Sequence`] holds [`Track`]s of [`Clip`]s over
//! a shared media pool. Positions and durations use [`v1000_core::Time`] (exact
//! rational seconds), so edit math is correct across mixed frame rates.
//!
//! A `Sequence` implements [`v1000_codec::FrameProducer`], so the preview reads
//! the timeline rather than a raw media source — this is the boundary between
//! "video player" and "editor".
//!
//! As of M2 this covers a single video track with multiple clips, in/out trim,
//! and ripple delete (plus multi-track storage for the M4 compositor).

mod clip;
mod sequence;
mod track;

pub use clip::Clip;
pub use sequence::Sequence;
pub use track::Track;
