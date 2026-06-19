//! Timeline model for V1000.
//!
//! Sequences contain video and audio tracks; tracks contain clips; clips carry
//! in/out points and keyframed parameters. Edit operations (ripple, roll, slip,
//! ripple-delete) maintain timeline consistency. Built on [`v1000_core`] types.
//!
//! Milestone M2 fills this in. Stub for now.

use v1000_core::TimeCode;

/// A half-open time span `[start, end)` on the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Inclusive start.
    pub start: TimeCode,
    /// Exclusive end.
    pub end: TimeCode,
}

impl Span {
    /// Whether `t` falls within `[start, end)`.
    pub fn contains(&self, t: TimeCode) -> bool {
        self.start <= t && t < self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_is_half_open() {
        let span = Span {
            start: TimeCode::new(0, 24, 1),
            end: TimeCode::new(10, 24, 1),
        };
        assert!(span.contains(TimeCode::new(0, 24, 1)));
        assert!(span.contains(TimeCode::new(9, 24, 1)));
        assert!(!span.contains(TimeCode::new(10, 24, 1)));
    }
}
