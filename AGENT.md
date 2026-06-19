# AGENT.md

Guidance for AI agents (and humans) working in the **V1000** repository. Read
this before making changes. For project vision and architecture, see
[README.md](README.md).

---

## What this project is

V1000 is a from-scratch, GPU-accelerated non-linear video editor in Rust,
organized as a Cargo workspace of focused crates. It is in **pre-alpha**: most
crates are stubs being filled in milestone by milestone (see the README
roadmap). Prefer building the next milestone end-to-end over adding breadth.

---

## Golden rules

1. **One tool per concern.** Before adding a dependency, check whether an
   existing committed choice already covers it. Do **not** introduce a second
   GPU layer, GUI toolkit, audio backend, or media library. The committed picks:
   `wgpu`, `egui`/`eframe`, `cpal`, `ffmpeg-next`, `rayon`/`crossbeam`,
   `parking_lot`, `ciborium`. New parallel paths require an ADR.
2. **Stay portable.** macOS/Apple Silicon is a first-class target.
   - No `core::arch::x86_64` / x86-only intrinsics in shared crates — use
     portable SIMD (`wide`, `pulp`, or `std::simd`).
   - No Linux-only syscalls (`MAP_HUGETLB`, etc.) and **no custom
     `#[global_allocator]`**. Pool frame buffers locally instead.
   - Gate anything genuinely platform-specific behind `#[cfg(...)]` with a
     working fallback.
3. **GPU-first for pixels.** Per-pixel work (color, blend, LUT, transforms)
   belongs in WGSL compute/render passes, not CPU loops over `f16` chunks. CPU
   is the fallback only.
4. **Keep the dependency graph acyclic.** `core`/`codec` are the bottom;
   `render`/`timeline`/`audio` sit above; `gui`/`export`/`app` on top. Never
   add an upward or sideways dependency that creates a cycle.
5. **The core stays deterministic.** Given the same timeline and timecode, the
   processing graph must produce identical output. Keep wall-clock time, RNG,
   and I/O out of `v1000-core`'s render path.
6. **Don't over-scaffold.** Implement what the current milestone needs. Leave a
   `// TODO(Mx):` marker rather than speculative abstractions for later phases.

---

## Workspace map

| Crate | Depends on | Notes |
|-------|-----------|-------|
| `v1000-core` | — | Frame types, color, processing graph, buffer pools. No workspace deps. |
| `v1000-codec` | `core` | FFmpeg-backed decode/encode + frame cache. |
| `v1000-timeline` | `core` | Sequences/tracks/clips, edit ops, keyframes. |
| `v1000-render` | `core`, `codec` | wgpu context, render-graph execution, preview. |
| `v1000-effects` | `core`, `render` | GPU effect trait + WGSL shaders. |
| `v1000-audio` | `core` | cpal engine, mixing, resampling. |
| `v1000-export` | `render`, `timeline`, `audio` | Offline render + mux + HW encode. |
| `v1000-gui` | everything above | egui widgets and app shell. |
| `v1000-app` | `gui` | Binary entry point. |

Shaders live in `v1000-effects/src/shaders/*.wgsl` and are `include_str!`'d.

---

## Build, lint, test

These are the commands CI runs; run them before declaring work done.

```bash
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings   # warnings are errors
cargo test --workspace
cargo run -p v1000-app                                   # smoke-test the shell
```

Work on **one crate** when you can: `cargo test -p v1000-timeline` is faster
than the whole workspace. FFmpeg system libraries are needed once `v1000-codec`
is non-stub (`brew install ffmpeg` on macOS).

---

## Conventions

- **Errors:** libraries return `Result<T, E>` with `thiserror` enums per crate;
  the `app` binary uses `anyhow`. No `unwrap()`/`expect()` outside tests, build
  scripts, and `main`. A real-time audio callback must never panic or allocate.
- **Concurrency:** `parking_lot` mutexes/rwlocks, not `std::sync`. `rayon` for
  data-parallel CPU loops, `crossbeam` channels for pipeline stages. Reserve
  `tokio` for the export-orchestration edge; don't pull async into the core.
- **Frames:** share as `Arc<Frame>`; recycle backing buffers through a pool.
  Treat frame contents as immutable once produced.
- **Timecode:** rational (frames + fps), never floating-point seconds for edit
  math, to avoid drift.
- **`unsafe`:** allowed only for SIMD/FFI/GPU interop, must carry a `// SAFETY:`
  comment, and must stay behind a safe API. Prefer a safe portable crate first.
- **Formatting:** default `rustfmt`. Module/file names `snake_case`, types
  `CamelCase`. Keep comment density and idiom consistent with surrounding code.
- **Public API:** document every `pub` item with a `///` doc comment.

---

## Known traps (carried over from the initial design sketch)

The original architecture draft contained code that compiles wrong or isn't
portable. When you implement these areas, do them correctly:

- **Graph execution:** you cannot `crossbeam::scope` over all nodes each calling
  `self.process_node(..)` mutably — it won't borrow-check, and a topological
  graph isn't embarrassingly parallel. Execute in dependency order and
  parallelize only independent branches; cache per-node outputs behind shared
  references.
- **Adaptive preview quality:** `frame_time > 16ms` means **under** 60fps →
  *lower* quality. Don't invert it, and don't `fetch_min(level - 1)` on a `u8`
  (it underflows). Clamp explicitly.
- **Allocator:** no global `mmap`/`HugeTLB` allocator — non-portable and the
  wrong scope. Use a dedicated frame-buffer pool.
- **SIMD:** no `_mm256_*` intrinsics in shared code; use portable SIMD so Apple
  Silicon builds.
- **Codecs/licensing:** don't link `x264`/`x265` or a GPL FFmpeg build into the
  default product path — they're GPL. Default to royalty-free / hardware
  encoders; gate GPL codecs behind an opt-in feature.

---

## Definition of done for a change

- Builds clean: `cargo build --workspace` and `clippy -D warnings` pass.
- Formatted: `cargo fmt --all -- --check` passes.
- Tested: new logic has tests; `cargo test --workspace` is green. If you changed
  rendering/preview, also run `cargo run -p v1000-app` and confirm it launches.
- Scoped: no new redundant dependency; no cross-crate cycle; no upward dep.
- Documented: public items have doc comments; a non-obvious architectural choice
  gets a short ADR in `docs/decisions/`.
- Honest reporting: if something is skipped, stubbed, or failing, say so plainly
  with the relevant output — don't claim verified work that wasn't run.

---

## Working agreement for agents

- Keep a task list for multi-step work and the milestone you're targeting.
- Make independent edits/searches in parallel; only serialize on real
  dependencies.
- Confirm before destructive or hard-to-reverse actions (rewriting history,
  deleting files you didn't create, force-push). Branch before committing on
  `main`; commit/push only when asked.
- When the design sketch and this file disagree, **this file wins** — the sketch
  is historical context, not a spec.
