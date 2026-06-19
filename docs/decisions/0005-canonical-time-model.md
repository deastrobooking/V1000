# ADR-0005: Canonical time model (exact rational seconds)

- **Status:** Accepted
- **Date:** 2026-06-19

## Context

The M1 `TimeCode` was a frame count plus a frame rate, and it derived `Ord`.
That is wrong for an editor: `24 frames @ 24 fps` and `30 frames @ 30 fps` are
the **same instant** (one second) but compared unequal and ordered by raw frame
count, so any timeline mixing rates (or comparing a clip against a sequence at a
different rate) would misbehave. Floating-point seconds are the usual
alternative but drift over long sequences and make exact edit points
impossible.

## Decision

Timeline positions and durations use an exact, rate-agnostic time:

- **`Rational`** — an `i64/i64` fraction, normalized, positive denominator, with
  `i128` intermediates to avoid overflow. Used for frame rates (e.g.
  `24000/1001`) and as the backing store of `Time`.
- **`Time`** — an exact `Rational` number of **seconds**. Ordering and
  arithmetic are exact and independent of any rate. Conversion to/from a frame
  index is always explicit and names the rate: `Time::from_frame(n, fps)` and
  `Time::to_frame(fps)`.
- **`TimeCode`** is demoted to a **display-only** SMPTE formatter built from a
  `Time` + rate. It no longer implements `Ord` — it is a label, not a
  comparable instant.

Floating-point seconds remain only at two honest boundaries: the wall-clock
playhead accumulator (`Transport`, advanced by frame delta-time) and GPU/UI
upload. Edit points never go through the float path.

## Consequences

- `24f@24` and `30f@30` compare equal; mixed-rate timelines are correct.
- Clips reference media at the media's own rate; the sequence resolves
  `timeline Time → clip → source Time → source frame index` with exact
  conversions, so a 24 fps clip on a 30 fps sequence is sampled correctly.
- This is **edit-time** modeling. It is not yet a decode-time PTS/DTS model:
  variable-frame-rate media, B-frame ordering, and stream timebases on the
  decode path are a separate, later concern (see the roadmap). The canonical
  edit time is the foundation they will map onto.
