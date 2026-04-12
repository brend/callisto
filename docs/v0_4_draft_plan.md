# Callisto v0.4 Draft Plan

This document defines the active execution plan for `v0.4.0`.

`v0.3.0` completed the first-party Playdate bootstrap/template/build workflow.  
`v0.4.0` focuses on product-depth polish, parser ergonomics, and type-inference quality.

Status: `v0.4.0` complete.

## Why v0.4

- Expand sample realism to validate longer-lived Playdate gameplay loops.
- Grow bindings only when driven by concrete sample requirements.
- Reduce syntax friction and annotation overhead in common language patterns.

## Release Guardrails

- Keep `v0.4.0` focused on three must-ship tracks only:
  - sample depth + sample-driven bindings
  - sum-type parser ergonomics
  - nullary generic constructor inference quality in nested contexts
- Avoid broad syntax expansion or large inference rewrites in this release.
- Keep all CLI behavior backward-compatible unless explicitly documented as additive.

## Scope (Must-Do)

1. Richer sample coverage with state persistence and explicit transitions beyond input-hold states.
2. Continue sample-driven binding additions only as required by richer samples.
3. Parser ergonomics: allow multi-line ADT/sum formatting after `=`, without requiring the first `|` on the same line.
4. Type inference: improve inference for nullary generic constructors (for example `None`) in nested/record-initializer contexts so temporary annotations are less often required.

## Out of Scope (v0.4)

- New package/dependency-management work.
- Broad parser grammar redesign beyond the targeted ADT/sum formatting change.
- Global bidirectional inference redesign beyond the targeted constructor-context improvements.

## Milestones

1. `M0: Scope Freeze + Validation Matrix`
- Lock `v0.4.0` boundaries and non-goals.
- Define mandatory command/test matrix before feature implementation.
- Checklist: [`docs/v0_4_m0_scope_freeze_checklist.md`](docs/v0_4_m0_scope_freeze_checklist.md)

2. `M1: Sample Depth + Binding Gaps`
- Implement richer Playdate sample state machines with explicit transitions.
- Add only binding modules needed by those sample flows.
- Checklist: [`docs/v0_4_m1_sample_depth_execution_checklist.md`](docs/v0_4_m1_sample_depth_execution_checklist.md)

3. `M2: Parser Ergonomics`
- Relax ADT/sum formatting constraints while preserving diagnostic clarity.
- Add parser regression coverage for multi-line variant declarations.
- Checklist: [`docs/v0_4_m2_parser_ergonomics_execution_checklist.md`](docs/v0_4_m2_parser_ergonomics_execution_checklist.md)

4. `M3: Type Inference Quality`
- Improve inference for nullary generic constructors in nested contexts.
- Add targeted checker regression tests for prior annotation-heavy cases.
- Checklist: [`docs/v0_4_m3_type_inference_execution_checklist.md`](docs/v0_4_m3_type_inference_execution_checklist.md)

5. `M4: Release Readiness`
- Regression/build pass, docs/changelog finalization, and release prep for `v0.4.0`.
- Checklist: [`docs/v0_4_m4_release_checklist.md`](docs/v0_4_m4_release_checklist.md)

## Milestone Status

- `M0`: complete.
- `M1`: complete (explicit-transition sample flow + persisted session state delivered).
- `M2`: complete (sum declarations now support multiline variant formatting after `=`).
- `M3`: complete (nested-context nullary generic constructor inference improved in record/constructor payload flows).
- `M4`: complete (tagged release + artifact verification + maintainer sign-off).
