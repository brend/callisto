# Callisto v0.3 M3 Playdate Product Checklist

Execution checklist for `M3: Playdate Build UX`.

Use this as the implementation board for first-party Playdate template and build ergonomics.

## Scope Freeze

- [x] Keep existing `check`, `emit-lua`, and `build` command behavior backward compatible.
- [x] Add first-party UX as additive commands (no breaking CLI rename).
- [x] Keep template output minimal and hand-editable.

## M3 Decisions (Implement As Written)

1. Template command
- Add `callisto init --template playdate <dir>`.
- Reject unknown template names with explicit diagnostics.
- Reject initialization in non-empty directories.

2. Build command
- Add `callisto build-playdate <entry.cal>` as the first-party path.
- Default to Playdate-oriented output (`Source/` + `.pdx`) while preserving explicit overrides.
- Support `--pdc` override and `--run` simulator launch.

3. Validation floor
- Every M3 implementation change must pass:
  - `cargo test`
  - `make -C playdate_bouncing_ball build-lua`
  - `make -C playdate_auto_bootstrap build-lua`

## Implementation Tasks

### A) CLI Surface

- [x] Add `init --template playdate` parsing and usage text in `src/cli.rs`.
- [x] Add `build-playdate` parsing with `--source-dir`, `--pdx`, `--pdc`, `--run`, `--config`, and `--module-root`.
- [x] Add CLI parser tests for valid/invalid M3 command paths.

### B) Runtime Behavior

- [x] Implement Playdate template scaffold creation in `src/main.rs`.
- [x] Implement `build-playdate` execution flow (`emit-lua` bootstrap + `pdc` + optional run) in `src/main.rs`.
- [x] Add command-level tests for template creation and build invocation behavior.

### C) Documentation

- [x] Update `README.md` with first-party template/build workflow.
- [x] Update `docs/callisto_cheat_sheet.md` with M3 command reference.
- [x] Update `docs/playdate_workflow.md` with `build-playdate` as the first-party path.
- [x] Record M3 status in `docs/v0_3_draft_plan.md`.

## Definition of Done (M3)

- [x] Callisto can scaffold a Playdate-ready project from a single command.
- [x] Callisto can emit, package, and optionally run Playdate output from a single command.
- [x] Existing manual and auto-bootstrap samples still build cleanly.
- [x] M3 docs/checklists are current at merge time.
