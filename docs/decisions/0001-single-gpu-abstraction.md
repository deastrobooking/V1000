# ADR-0001: Single GPU abstraction (`wgpu`)

- **Status:** Accepted
- **Date:** 2026-06-19

## Context

The initial architecture sketch listed `wgpu`, `vulkano`, `cuda-sys`, and `ocl`
as dependencies simultaneously. Maintaining four GPU paths multiplies the
surface area for bugs, testing, and platform support, and none of them
individually justify the others.

## Decision

`wgpu` is the single GPU abstraction for all pixel processing and preview. It
targets Metal on macOS and Vulkan/DX12 elsewhere through one API, which matches
our portability requirement (ADR-0003).

Low-level APIs are added **only behind a feature flag for a proven, specific
need** — primarily codec interop (NVENC/NVDEC, VideoToolbox) where the encoder
must share GPU surfaces with the decoder. They are never a parallel general
compute path.

## Consequences

- One shader language (WGSL) and one resource model to learn and maintain.
- Some bleeding-edge vendor features may be unavailable until exposed by wgpu;
  acceptable for now.
- Hardware-codec interop will require careful, feature-gated unsafe FFI later.
