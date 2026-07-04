# PROGRESS

This is an append-only progress log. Keep new entries dated and factual.

## 2026-07-04

- Established the initial product direction for `dun`: a Rust `1.85`
  `ratatui`-based terminal editor for Linux/macOS SSH operations work.
- Confirmed the primary workflow: inspect and edit text files, read and filter
  logs, and support custom operational log filters.
- Confirmed terminal compatibility goals: UTF-8 plus 256 colors by default,
  with 16-color and ASCII fallback profiles.
- Surveyed the neighboring `rum` project and its current `rum-host` direction.
- Decided not to depend on `rum` yet because its release API is not fixed.
- Established the future plugin boundary: `rum` is used only as a pure
  evaluator inside `dun`; roles, policies, resources, and editor API access are
  owned by `dun`.
- Created the initial project documents: `README.md`, `AGENTS.md`, `PLAN.md`,
  `TODO.md`, `PROGRESS.md`, and `AUDIT.md`.
- Initialized the git repository and minimal Cargo binary package.
- Confirmed the local toolchain is `cargo 1.85.0` and `rustc 1.85.0`.
