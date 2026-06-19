# ADR-0002: Single GUI toolkit (`egui`/`eframe`)

- **Status:** Accepted
- **Date:** 2026-06-19

## Context

The sketch listed both `egui` and `iced`. They are different paradigms
(immediate-mode vs. Elm-like retained), and shipping both means two rendering
integrations, two event models, and a fractured widget set.

## Decision

Use `egui` with `eframe` as the only GUI toolkit. Immediate-mode fits a
tool-dense, data-driven editor where panels reflect rapidly changing project
state, and `eframe` integrates cleanly with the `wgpu` backend chosen in
ADR-0001.

## Consequences

- The preview surface composes with egui via a `wgpu` callback rather than a
  second windowing stack.
- If a retained-mode need appears later (e.g. a complex node editor), solve it
  within egui or as an isolated component — not by reintroducing a second
  framework.
