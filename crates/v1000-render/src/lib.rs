//! Rendering for V1000.
//!
//! Owns the `wgpu` device/queue, executes the processing graph in dependency
//! order (parallelizing only independent branches — never all nodes naively),
//! and drives the real-time preview. Pixel work runs in WGSL compute/render
//! passes; CPU is a portable fallback only.
//!
//! Milestone M1 fills this in. Stub for now.

/// One-line description of the render backend, including its decode source.
pub fn info() -> String {
    format!(
        "wgpu render (pending M1) over {}",
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
