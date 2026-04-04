# Callisto v0.3 M2 Binding Expansion Checklist

Execution checklist for `M2: Binding Surface Expansion`.

Use this as the implementation board for post-`M1` Playdate product work.

## Scope Freeze

- [x] Keep shared bindings as source modules under `playdate_bindings/src`.
- [x] Keep existing sample workflows (`manual shim` and `auto bootstrap`) intact.
- [x] Do not add uncertain SDK APIs without sample-driven usage.

## M2 Decisions (Implement As Written)

1. Binding growth is sample-driven
- Add APIs only when they are used in real samples or immediate template flows.

2. Bouncing-ball policy
- Keep upgrading `playdate_bouncing_ball` to use language features implemented so far.
- Treat this sample as a language-feature integration surface, not just SDK smoke.

3. Validation floor
- Every binding expansion must pass:
  - `cargo test`
  - `make -C playdate_bouncing_ball build-lua`
  - `make -C playdate_auto_bootstrap build-lua`

## Implementation Tasks

### A) Shared Bindings

- [x] Add `playdate.input` helper module.
- [x] Keep `playdate`, `playdate.graphics`, `playdate.graphics.sprite`, and `playdate.timer` organized under `playdate_bindings/src`.
- [x] Add next concrete surface (`playdate.sound` and/or additional system wrappers) driven by sample need.

### B) Bouncing Ball Feature Utilization

- [x] Use sum types in game-state/control flow logic.
- [x] Use `match` for explicit control-mode/result handling.
- [x] Use `impl` methods for domain behavior.
- [x] Use record update expressions for state transitions.
- [x] Add one generic helper or additional ADT pattern where it improves clarity without overfitting.

### C) Sample and Docs

- [x] Update sample docs to reflect new controls and generated outputs.
- [x] Keep workflow docs linked to active `v0.3` planning/checklist docs.
- [x] Record M2 progress in `docs/v0_3_draft_plan.md`.

### D) Regression Coverage

- [x] Add emission regression tests for `playdate.input` Lua paths.
- [x] Add emission regression tests for `playdate.audio` Lua paths.

## Definition of Done (M2)

- [x] Shared bindings cover required surfaces for at least one richer gameplay sample.
- [x] `playdate_bouncing_ball` meaningfully exercises the implemented language feature set.
- [x] Validation floor commands pass from a clean checkout.
- [x] Draft plan and execution checklist are current at merge time.
