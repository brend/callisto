# Callisto v0.2 M3 Diagnostics Plan

This document defines the proposed error-code scheme and golden-test structure for `M3`.

## Goals

- Make diagnostics stable enough to reference in docs/issues.
- Keep messages human-readable while adding machine-searchable identifiers.
- Prevent accidental regressions in wording/notes/location rendering.

## Proposed Error-Code Scheme

Format: `CAL-<PHASE>-<NNN>`

- `PHASE` is one of:
  - `LEX` for lexer
  - `PAR` for parser
  - `RES` for resolver/import/module
  - `TYP` for typechecker
  - `CFG` for config/CLI config loading

Examples:
- `CAL-CFG-001` missing explicit config file
- `CAL-RES-010` unresolved import module file
- `CAL-TYP-021` constructor payload shape mismatch

Rules:
- Codes are append-only; do not reuse retired numbers.
- Message text may be refined without changing code semantics.
- Code appears near the primary diagnostic message (rendered output).

## Golden Diagnostics Test Layout

Fixtures live at:
- `tests/golden/diagnostics/*.txt`

Harness behavior:
- Generate rendered diagnostics from a deterministic in-memory source filename.
- Compare output byte-for-byte against fixture files.
- Support fixture refresh via `UPDATE_GOLDENS=1`.

Initial scaffolding:
- constructor payload note mismatch
- unresolved imported module member

## Implementation Phases

1. Add optional `code` field to `Diagnostic` and render it.
2. Introduce code constants per phase/module.
3. Attach codes to existing high-frequency diagnostics first:
- config errors
- unresolved imports/module members
- constructor/record payload mismatches
4. Expand golden fixtures to include code-bearing output.

## Current Progress

- Phase 1 is implemented:
  - `Diagnostic` now supports optional `code`.
  - Renderer prints coded diagnostics as `error[CODE]`.
- Initial phase-3 attachment is started:
  - `CAL-CFG-001..004` for config loader/display errors.
  - `CAL-RES-010` unresolved import module file.
  - `CAL-RES-013..015` module loader errors (read failure, module path mismatch, duplicate module definition).
  - `CAL-RES-020..021` duplicate import alias/item.
  - `CAL-RES-011` unknown imported module member/function.
  - `CAL-RES-012` imported symbol path with no matching declaration.
  - `CAL-TYP-021` constructor payload shape mismatches.
  - `CAL-TYP-001` unresolved name.
  - `CAL-TYP-010..012` call target/arity/type mismatches.
  - `CAL-TYP-022` constructor generic inference failures.
  - `CAL-TYP-030` non-exhaustive matches.
- Golden diagnostics scaffolding and initial fixtures are active under `tests/golden/diagnostics/`.
- Golden emitted-Lua fixtures are active under `tests/golden/lua/`:
  - `record_update.lua`
  - `sum_match.lua`
- Imported-item diagnostics now suppress follow-on call-target noise when the primary
  import/declaration failure already explains the issue.

## Acceptance Criteria

- New diagnostics include stable codes for top failure paths.
- Golden tests fail on accidental formatting/message drift.
- README/docs can reference concrete error codes for troubleshooting.
