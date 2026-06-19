# V1000

A professional, GPU-accelerated, non-linear video editor written in Rust.

V1000 is a from-scratch NLE (non-linear editor) engine. The goal is a modular,
cross-platform foundation that scales from simple trimming to multi-track
compositing, color grading, and hardware-accelerated export — built on a
single, coherent set of dependencies rather than a pile of overlapping ones.

> **Status:** pre-alpha / architecture. The codebase is being scaffolded
> milestone by milestone (see [Roadmap](#roadmap)). Nothing here is shippable yet.

---

## Design principles

1. **One primary tool per concern.** A GPU abstraction, an audio backend, a GUI
   toolkit, a media I/O layer — each has exactly one default. Direct/low-level
   APIs (CUDA, Vulkan) are added only behind a feature flag for a proven need
   (e.g. NVENC/NVDEC interop), never as parallel paths.
2. **GPU-first, CPU-fallback.** Pixel work runs on the GPU through `wgpu`
   compute/render pipelines. CPU paths exist as portable fallbacks, never as the
   main road.
3. **Portable by default.** No x86-only intrinsics, no Linux-only syscalls in
   core crates. Primary dev/test target includes **macOS on Apple Silicon**, so
   SIMD goes through portable abstractions and allocation through pool
   allocators — not a global `mmap`/`HugeTLB` allocator.
4. **Frames are immutable, reference-counted, and pooled.** Decoded/processed
   frames are `Arc`-shared and recycled through buffer pools to bound memory.
5. **Deterministic core, incremental UI.** The render graph produces the same
   output for the same timeline + time; the GUI is a thin immediate-mode view
   over project state.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Application Layer                      │
│   Timeline · Effects panel · Export queue · Media browser │
├─────────────────────────────────────────────────────────┤
│                     Core Engine                           │
│        Graph-based processing pipeline (deterministic)    │
├─────────────────────────────────────────────────────────┤
│              Hardware Acceleration Layer                  │
│     wgpu (Vulkan/Metal/DX12)  ·  portable SIMD fallback   │
└─────────────────────────────────────────────────────────┘
```

### Workspace layout

| Crate | Responsibility |
|-------|----------------|
| `v1000-core` | Frame types, color management, processing graph, buffer pools |
| `v1000-timeline` | Sequences, tracks, clips, edits (ripple/roll/slip), keyframes |
| `v1000-codec` | Media I/O + decode/encode abstraction over the media backend |
| `v1000-render` | wgpu device/context, render graph execution, preview engine |
| `v1000-effects` | GPU effect trait + built-in effects (shaders in WGSL) |
| `v1000-audio` | Real-time audio engine, mixing graph, resampling |
| `v1000-export` | Render-to-file pipeline, muxing, hardware encoders |
| `v1000-gui` | Editor application shell and widgets |
| `v1000-app` | Binary that wires the crates together |

Dependencies flow downward only: `gui`/`export` → `render`/`timeline`/`audio`
→ `core`/`codec`. No cycles. `core` depends on nothing else in the workspace.

### Committed dependency choices

These are deliberate single picks; alternatives were dropped to avoid redundant
or conflicting paths (see [Decision log](#decision-log)).

- **GPU:** `wgpu` (Metal on macOS, Vulkan/DX12 elsewhere). *Not* vulkano + cuda
  + ocl in parallel — those are feature-gated additions only.
- **GUI:** `egui` + `eframe`. *Not* egui **and** iced.
- **Audio I/O:** `cpal` directly (low latency). *Not* rodio (which wraps cpal).
- **Media I/O:** a single FFmpeg binding (`ffmpeg-next`) as the decode/mux
  backend. Standalone encoder/muxer crates (`x264`, `rav1e`, `mp4`, `matroska`)
  are evaluated later and gated — FFmpeg already wraps most of them, and the
  GPL/LGPL split must be resolved first (see licensing note below).
- **Parallelism:** `rayon` for data-parallel CPU work, `crossbeam` channels for
  pipelines, `parking_lot` locks. `tokio` only at the export-orchestration edge.
- **Serialization:** project files as versioned CBOR (`ciborium`).
- **SIMD:** portable (`wide` / `pulp` / `std::simd`), never `core::arch::x86_64`
  in shared crates.

> **Licensing note (must resolve before any release):** `x264`/`x265` are
> GPL and the GPL FFmpeg build is virally licensed. A proprietary or
> permissively-licensed product cannot link them. The default export path
> should target royalty-free / hardware codecs (AV1 via `rav1e`/`SVT-AV1`,
> VP9, platform NVENC/QSV/VideoToolbox) with GPL codecs as an opt-in build.

---

## Roadmap

Each milestone is independently runnable and demoable. Don't start N+1 before N
plays end-to-end.

- **M0 — Scaffold.** Cargo workspace, the nine crates as stubs, CI (fmt + clippy
  + test), `v1000-app` opens an empty `egui` window. *(no media yet)*
- **M1 — Decode & preview.** `v1000-codec` decodes a single file; `v1000-render`
  uploads frames to a `wgpu` texture and draws them; basic transport
  (play/pause/scrub). Frame cache with LRU + buffer pool.
- **M2 — Timeline core.** `v1000-timeline` model: one video track, multiple
  clips, in/out trim, ripple delete. Playhead reads from the timeline, not a
  raw file.
- **M3 — Processing graph.** `v1000-core` graph executes a clip → transform →
  output chain deterministically per timecode, with branch-parallel execution
  (not naive all-node parallelism).
- **M4 — Effects & compositing.** GPU effect trait, color correction + a blend
  compositor, multiple stacked video tracks with opacity/blend modes.
- **M5 — Audio.** `cpal` output, per-track gain/pan, A/V sync against the
  playhead, sample-rate conversion (`rubato`).
- **M6 — Export.** Offline render of a sequence to a file; software encode first,
  then hardware (VideoToolbox on macOS / NVENC), progress + cancel.
- **M7 — Color management.** Working-space pipeline, 3D LUTs, scopes
  (waveform/vectorscope) — all GPU-side.
- **M8+** — Transitions, keyframe editor, undo/redo (command + diff), media
  browser, plugin sandbox (WASM).

---

## Getting started

> Requires a recent stable Rust toolchain and a GPU/driver with Vulkan, Metal,
> or DX12 support.

```bash
# Build everything
cargo build --workspace

# Run the editor shell (currently: empty window — M0)
cargo run -p v1000-app

# Lint + test (what CI runs)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

FFmpeg system libraries are required for `v1000-codec` once M1 lands; see
that crate's README for platform install steps (`brew install ffmpeg` on macOS).

---

## Decision log

Non-obvious architectural choices live in `docs/decisions/` as short ADRs
(Architecture Decision Records). The most consequential so far:

- **ADR-0001** Single GPU abstraction (`wgpu`), low-level APIs feature-gated.
- **ADR-0002** One GUI toolkit (`egui`), not egui + iced.
- **ADR-0003** Portability constraints: no x86-only SIMD, no Linux-only
  allocation in core crates; macOS/Apple Silicon is a first-class target.
- **ADR-0004** Codec/licensing strategy: royalty-free + hardware default, GPL
  codecs opt-in.

See [AGENT.md](AGENT.md) for conventions, build/test details, and guidance when
working in this repo.

## License

See [LICENSE](LICENSE).
