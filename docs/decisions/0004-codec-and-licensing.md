# ADR-0004: Codec and licensing strategy

- **Status:** Accepted
- **Date:** 2026-06-19

## Context

The sketch depended on `ffmpeg-next` alongside standalone `x264`, `rav1e`,
`mp4`, and `matroska` crates. This is redundant — FFmpeg already wraps most
encoders and muxers — and `x264`/`x265` plus the GPL build of FFmpeg are
**GPL-licensed**. Linking them makes the whole product GPL, which conflicts
with shipping a proprietary or permissively-licensed editor.

## Decision

1. **Media I/O backend:** a single FFmpeg binding for demux/decode/mux, built
   against an **LGPL** FFmpeg configuration. The concrete crate is
   `ffmpeg-the-third` — an actively-maintained `ffmpeg-next` fork — because it
   tracks current FFmpeg (verified against **8.1**, libavcodec 62), which
   `ffmpeg-next` did not at the time of writing. The API is the same; this is
   still "one binding". Gated behind the off-by-default `ffmpeg` feature so the
   default workspace builds without system FFmpeg.
2. **Default export codecs:** royalty-free and hardware encoders —
   AV1 (`rav1e` / `SVT-AV1`), VP9, and platform hardware (VideoToolbox on
   macOS, NVENC/QSV elsewhere).
3. **GPL codecs (`x264`/`x265`) are opt-in only**, behind a `gpl` Cargo feature
   that is off by default, with the licensing implication documented at the
   build boundary.

## Consequences

- The default build is distributable without GPL obligations.
- Standalone encoder/muxer crates are evaluated per-need and gated, not adopted
  wholesale.
- CI and release builds must assert the `gpl` feature is disabled for any
  distributed artifact.
