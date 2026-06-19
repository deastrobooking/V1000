//! Media input/output for V1000.
//!
//! Exposes the [`FrameSource`] abstraction that decoders implement, an animated
//! [`TestPatternSource`] used as the default preview source, and — with the
//! `ffmpeg` feature — a [`FileDecoder`] backed by a single FFmpeg binding
//! (`ffmpeg-the-third`; see ADR-0004).

mod producer;
mod source;

pub use producer::FrameProducer;
pub use source::{FrameSource, SourceError, TestPatternSource};

#[cfg(feature = "ffmpeg")]
mod decoder;

#[cfg(feature = "ffmpeg")]
pub use decoder::FileDecoder;

/// Name of the active decode backend, for diagnostics and the about box.
pub fn backend_name() -> &'static str {
    if cfg!(feature = "ffmpeg") {
        "ffmpeg-the-third"
    } else {
        "test-pattern only (build with --features ffmpeg for file decode)"
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn backend_is_named() {
        assert!(!super::backend_name().is_empty());
    }
}
