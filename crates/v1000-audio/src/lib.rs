//! Audio engine for V1000.
//!
//! Uses `cpal` directly for low-latency output, mixes per-track gain/pan, and
//! resamples with `rubato`. The real-time callback must never allocate or
//! panic. A/V sync is driven against the shared playhead, expressed in
//! [`v1000_core`] timecode.
//!
//! Milestone M5 fills this in. Stub for now.

use v1000_core::TimeCode;

/// Placeholder: the playhead position the audio engine would render from.
pub fn playhead_default() -> TimeCode {
    TimeCode::new(0, 48_000, 1)
}

#[cfg(test)]
mod tests {
    #[test]
    fn playhead_starts_at_zero() {
        assert_eq!(super::playhead_default().frames(), 0);
    }
}
