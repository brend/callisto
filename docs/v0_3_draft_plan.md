# Callisto v0.3 Draft Plan

This document defines the first draft for `v0.3` planning.

`v0.1` established the compiler core, and `v0.2` stabilized project config, diagnostics, and release quality.  
`v0.3` focuses on Playdate-oriented product usability on top of that foundation.

## Why v0.3

- Turn Playdate support from "possible" into "repeatable"
- Reduce hand-written Lua glue in real projects
- Provide a reusable SDK binding surface for teams/projects

## Proposed v0.3 Goals (Must-Do)

### 1) Playdate bootstrap workflow

Deliverables:
- Add explicit CLI support to generate a Playdate `main.lua` update shim for directory outputs.
- Validate entry-module requirements for bootstrap generation.
- Keep non-Playdate Lua emission behavior unchanged by default.

Acceptance criteria:
- `emit-lua/build --playdate-bootstrap` succeeds for projects exporting `pub fn update() -> Unit`.
- Invalid usage produces clear actionable errors.

### 2) Shared Playdate binding modules

Deliverables:
- Introduce a shared `playdate_bindings/` module-root package with common SDK externs.
- Cover initial high-utility surfaces: `playdate`, `playdate.graphics`, `playdate.graphics.sprite`, and `playdate.timer`.
- Wire sample projects to consume shared bindings via `module_roots`.

Acceptance criteria:
- Sample projects compile without local duplicated binding definitions.
- Generated Lua continues to emit verbatim Playdate SDK call paths.

### 3) Auto-bootstrap sample coverage

Deliverables:
- Add at least one sample project that uses `--playdate-bootstrap` end-to-end.
- Keep existing stateful/manual-shim samples to validate both workflows.

Acceptance criteria:
- `make build-lua` succeeds for both manual-shim and auto-shim sample projects.

### 4) Documentation and development tracking

Deliverables:
- Keep milestone/checklist docs for `v0.3` in the same style as `v0.1` and `v0.2`.
- Update workflow docs and cheat sheet as behavior evolves.

Acceptance criteria:
- `docs/` contains a current draft plan and execution checklist for active milestone work.

## Milestones

1. `M1: Bootstrap + Shared Bindings Foundation`
- `--playdate-bootstrap` CLI and emission flow
- Shared `playdate_bindings/` module-root package
- New auto-bootstrap sample project
- Checklist: [`docs/v0_3_m1_playdate_execution_checklist.md`](docs/v0_3_m1_playdate_execution_checklist.md)

2. `M2: Binding Surface Expansion`
- Add broader SDK coverage (input/sound/system APIs as needed)
- Add sample-driven validation for newly added bindings
- Checklist: [`docs/v0_3_m2_bindings_execution_checklist.md`](docs/v0_3_m2_bindings_execution_checklist.md)

3. `M3: Playdate Build UX`
- Improve command/template ergonomics around `callisto build` + `pdc`
- Document recommended project templates and workflows
- Checklist: [`docs/v0_3_m3_playdate_product_execution_checklist.md`](docs/v0_3_m3_playdate_product_execution_checklist.md)

4. `M4: Release Readiness`
- Regression pass
- Docs/changelog finalization
- Tag and announcement prep for `v0.3.0`

## M1 Status (Implemented on Current Branch)

`M1` is implemented on the current branch with:
- `--playdate-bootstrap` added to `emit-lua` and `build`.
- Bootstrap generation of `main.lua` for directory outputs.
- Validation for bootstrap misuse (`-o file.lua`), missing `pub fn update() -> Unit`, and `main.lua` overwrite collisions.
- Shared bindings package at `playdate_bindings/src/playdate/*`.
- Existing `playdate_bouncing_ball` sample migrated to shared module roots.
- New `playdate_auto_bootstrap` sample added with end-to-end auto-shim flow.
- Tests added for CLI parsing and bootstrap emission behavior.

## M2 Status (Implemented on Current Branch)

`M2` progress on the current branch:
- Added shared `playdate.input` module with button helpers.
- Added shared `playdate.audio` module with sample-driven sound helpers.
- Added shared `playdate.system` module with crank-position wrappers.
- Updated `playdate_bouncing_ball` to consume input bindings and exercise additional language features:
  - sum types (`ControlMode`, `StepResult`)
  - exhaustive `match`
  - `impl` method (`Ball.moved`)
  - record updates (`with`)
  - generic helper (`unwrap_or[T]`) used in sound-effect dispatch
  - manual shim preload of `playdate/input`, `playdate/audio`, and `playdate/system` for runtime wiring
  - runtime-safe hand-written audio shim to prevent missing-SDK-path crashes
- Added regression tests guarding emitted Lua paths for:
  - `playdate.input` helpers
  - `playdate.audio` wrappers
  - `playdate.system` wrappers
- Verified sample build smoke for:
  - `make -C playdate_bouncing_ball build-lua`
  - `make -C playdate_auto_bootstrap build-lua`

## M3 Status (Implemented on Current Branch)

`M3` progress on the current branch:
- Added first-party Playdate template scaffolding command:
  - `callisto init --template playdate <dir>`
  - Generates `callisto.toml`, `src/game.cal`, `Source/`, `README.md`, and `Makefile`.
- Added first-party Playdate build UX command:
  - `callisto build-playdate <entry.cal> [--source-dir dir] [--pdx bundle.pdx] [--pdc exe] [--run]`
  - Emits Lua with Playdate bootstrap, runs `pdc`, and can open simulator output with `--run`.
- Added CLI parser tests for `init`/`build-playdate` flows and invalid template handling.
- Added runtime tests for template creation behavior and `build-playdate` invocation with a fake `pdc`.
- Updated docs (`README`, cheat sheet, workflow guide) to reflect the first-party Playdate flow.

## Immediate Next Tasks

1. Start `M4` release-readiness pass for `v0.3.0` (regression sweep + docs/changelog freeze).
2. Add a richer gameplay sample or extend auto-bootstrap demo with multiple gameplay states.
3. Continue sample-driven binding additions only as required by richer samples.
