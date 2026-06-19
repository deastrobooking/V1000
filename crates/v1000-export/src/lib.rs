//! Export pipeline for V1000.
//!
//! Renders a sequence offline and muxes it to a file. Software encode lands
//! first, then hardware encoders (VideoToolbox on macOS, NVENC/QSV elsewhere).
//! The default path targets royalty-free / hardware codecs; GPL codecs are an
//! opt-in build (see ADR-0004). `tokio` is used only here, at the orchestration
//! edge — it does not reach into the core.
//!
//! Milestone M6 fills this in. Stub for now.

/// Default container/codec label for the export path. Provisional until M6.
pub fn default_target() -> &'static str {
    let _render = v1000_render::info();
    let _audio = v1000_audio::playhead_default();
    let _timeline = std::mem::size_of::<v1000_timeline::Span>();
    "mp4/h264 (hardware, pending M6)"
}

#[cfg(test)]
mod tests {
    #[test]
    fn has_default_target() {
        assert!(super::default_target().contains("mp4"));
    }
}
