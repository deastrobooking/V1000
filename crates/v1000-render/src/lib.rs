//! Rendering for V1000.
//!
//! As of M2 this provides the [`Transport`] — the playback clock. It is
//! decoupled from the media: the caller owns the timeline `Sequence` (a
//! [`v1000_codec::FrameProducer`]) and asks it for the frame at the transport's
//! playhead, which keeps this crate independent of the timeline crate.
//!
//! Owning the `wgpu` device, executing the processing graph in dependency order
//! (parallelizing only independent branches — never all nodes naively), and
//! driving custom GPU passes arrive with M3. For now the GUI uploads the
//! produced frame to the GPU through eframe's wgpu backend.

mod preview;

pub use preview::Transport;

/// One-line description of the render backend, including its decode source.
pub fn info() -> String {
    format!(
        "wgpu render (M1: preview transport) over {}",
        v1000_codec::backend_name()
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn info_mentions_backend() {
        assert!(super::info().contains("ffmpeg"));
    }
}
