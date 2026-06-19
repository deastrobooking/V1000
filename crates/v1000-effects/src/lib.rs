//! GPU effects for V1000.
//!
//! Defines the effect trait that built-in effects (color correction, blur,
//! blend compositor, …) implement, each backed by a WGSL shader under
//! `src/shaders/`. Effects plug into the [`v1000_render`] graph.
//!
//! Milestone M4 fills this in. Stub for now.

/// Number of built-in effects currently registered. Zero until M4.
pub fn builtin_count() -> usize {
    let _render = v1000_render::info();
    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn no_builtins_yet() {
        assert_eq!(super::builtin_count(), 0);
    }
}
