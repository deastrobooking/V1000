//! Media input/output for V1000.
//!
//! Wraps a single FFmpeg binding (`ffmpeg-next`) as the decode/mux backend and
//! exposes a GOP-aware, cached decoder to the rest of the engine. Standalone
//! encoder/muxer crates are gated behind features and a licensing decision
//! (see ADR-0004) — they are not part of the default path.
//!
//! Milestone M1 fills this in. Stub for now.

/// Returns the crate name. Placeholder until M1 lands the decoder.
pub fn backend_name() -> &'static str {
    "ffmpeg-next (pending M1)"
}

#[cfg(test)]
mod tests {
    #[test]
    fn backend_is_named() {
        assert!(super::backend_name().contains("ffmpeg"));
    }
}
