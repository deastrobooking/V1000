# ADR-0003: Portability constraints

- **Status:** Accepted
- **Date:** 2026-06-19

## Context

The sketch used `std::arch::x86_64` AVX2 intrinsics for frame blending and a
custom `#[global_allocator]` backed by `mmap` with `MAP_HUGETLB`. Both are
non-portable: the intrinsics do not compile on ARM, and `MAP_HUGETLB` is
Linux-only. The primary development and test environment includes **macOS on
Apple Silicon**, where neither works.

## Decision

Core and shared crates must be portable:

1. **SIMD** goes through a portable abstraction (`wide`, `pulp`, or
   `std::simd`) — never architecture-specific intrinsics in shared code.
2. **No custom global allocator.** Frame buffers are recycled through a
   dedicated pool allocator with a well-defined scope, not a process-wide one.
3. **No OS-specific syscalls** in core crates. Anything genuinely
   platform-specific is `#[cfg(...)]`-gated with a working fallback.

## Consequences

- Builds and tests pass on macOS/Apple Silicon, Linux, and Windows from M0.
- Hand-tuned per-arch kernels, if ever needed, live behind `cfg` with a
  portable default — they are an optimization, not the baseline.
