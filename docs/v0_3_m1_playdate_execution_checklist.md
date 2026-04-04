# Callisto v0.3 M1 Playdate Execution Checklist

Execution checklist for `M1: Bootstrap + Shared Bindings Foundation`.

Use this as the implementation board. Keep tasks checked only when code and tests are done.

## Scope Freeze

- [x] `--playdate-bootstrap` remains explicit opt-in (no default behavior change).
- [x] Existing non-Playdate `emit-lua/build` flows remain backward-compatible.
- [x] Shared bindings are delivered as module-root source files (no package manager scope for `M1`).

## M1 Decisions (Implement As Written)

1. Bootstrap mode behavior
- Only applies when output is a directory.
- Writes `main.lua` shim that imports emitted entry module and calls `update()`.
- Requires entry module `pub fn update() -> Unit`.

2. Shared binding package location
- Package root: `playdate_bindings/src`.
- Projects consume via `module_roots` in `callisto.toml`.

3. Sample strategy
- Keep one manual-shim sample (`playdate_bouncing_ball`).
- Add one auto-bootstrap sample (`playdate_auto_bootstrap`).

## Implementation Tasks

### A) CLI + Compiler Plumbing

- [x] Add `--playdate-bootstrap` parsing for `emit-lua`/`build` in `src/cli.rs`.
- [x] Thread bootstrap option through command handlers in `src/main.rs`.
- [x] Reject bootstrap mode for single-file `-o <file.lua>` output.
- [x] Implement `main.lua` shim generation for directory outputs.
- [x] Add runtime-safe validation for bootstrap prerequisites.

### B) Test Coverage

- [x] Add CLI parser test for `--playdate-bootstrap`.
- [x] Add integration-style emit test asserting generated `main.lua` shim.
- [x] Add integration-style emit test asserting failure when `update()` export is missing.
- [x] Run full suite (`cargo test`) after integration.

### C) Shared Bindings Package

- [x] Add `playdate_bindings/README.md`.
- [x] Add root bindings module `playdate_bindings/src/playdate.cal`.
- [x] Add graphics bindings module `playdate_bindings/src/playdate/graphics.cal`.
- [x] Add sprite bindings module `playdate_bindings/src/playdate/graphics/sprite.cal`.
- [x] Add timer bindings module `playdate_bindings/src/playdate/timer.cal`.

### D) Sample Integration

- [x] Migrate `playdate_bouncing_ball` to shared `module_roots`.
- [x] Remove duplicate local binding module from `playdate_bouncing_ball/src/playdate`.
- [x] Add `playdate_auto_bootstrap` sample project with Makefile/config/source.
- [x] Verify `make build-lua` for both sample projects.

### E) Docs + Tracking

- [x] Update README/cheat-sheet/workflow docs for bootstrap + shared bindings.
- [x] Add `v0.3` draft plan and this execution checklist under `docs/`.
- [x] Update `CHANGELOG.md` Unreleased entries for `M1`.

## Definition of Done (M1)

- [x] `cargo test` passes.
- [x] `make -C playdate_bouncing_ball build-lua` passes.
- [x] `make -C playdate_auto_bootstrap build-lua` passes.
- [x] Workflow docs reflect both manual-shim and auto-shim paths.
- [x] `docs/v0_3_draft_plan.md` and this checklist are current.
