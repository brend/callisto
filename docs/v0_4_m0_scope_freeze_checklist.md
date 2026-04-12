# Callisto v0.4 M0 Scope Freeze Checklist

Execution checklist for `M0: Scope Freeze + Validation Matrix`.

Use this as the planning gate before implementation milestones start.

## Scope Freeze

- [x] Confirm `v0.4.0` must-ship tracks:
  - sample depth + sample-driven bindings
  - sum-type parser ergonomics
  - nullary generic constructor inference quality in nested contexts
- [x] Confirm no new major feature tracks are introduced before `M4`.
- [x] Confirm all non-goals are explicit in [`docs/v0_4_draft_plan.md`](docs/v0_4_draft_plan.md).

## M0 Decisions (Implement As Written)

1. Sample-driven policy
- Binding additions are allowed only when required by the `M1` sample scenario.
- If a binding is added, a sample usage path must exist in the same milestone.

2. Parser policy
- `M2` only targets sum-type declaration formatting after `=`.
- Existing valid sum/alias/type syntax remains backward-compatible.

3. Inference policy
- `M3` only targets nullary generic constructors when contextual type information is available.
- Unconstrained nullary generic constructors must still emit clear inference diagnostics.

4. Release quality floor
- Every milestone keeps `cargo test` green before moving to the next milestone.
- Playdate sample Lua build smoke remains part of the ongoing gate.

## Validation Matrix (Must Pass Before M4)

- [x] `cargo test`
- [x] `make -C playdate_bouncing_ball build-lua`
- [x] `make -C playdate_auto_bootstrap build-lua`
- [x] `target/debug/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
- [x] `target/debug/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o /tmp/callisto_v0_4_smoke --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`

## Exit Criteria (M0)

- [x] `docs/v0_4_draft_plan.md` includes all milestone checklist links.
- [x] `README.md` links to active `v0.4` execution checklists.
- [x] Maintainer confirms `M0` freeze before `M1` implementation work.
