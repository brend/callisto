# Callisto v0.4 M1 Sample Depth Checklist

Execution checklist for `M1: Sample Depth + Binding Gaps`.

Use this as the implementation board for sample realism and sample-driven binding expansion.

## Scope Freeze

- [x] Keep sample work focused on persistent state and explicit transitions.
- [x] Add bindings only when demanded by concrete sample behavior.
- [x] Avoid unrelated gameplay/framework expansion in this milestone.

## M1 Decisions (Implement As Written)

1. Sample direction
- Extend first-party Playdate sample coverage with at least one longer-lived state machine flow.
- Require explicit transitions (not only button-hold derived state).

2. Persistence floor
- Include at least one persisted session/gameplay value that survives across update frames.
- Keep data modeling in idiomatic Callisto style (`type` + `match` + record updates).

3. Binding policy
- Prefer existing `playdate_bindings` modules first.
- Add new binding modules/functions only when the sample cannot express required behavior without them.

## Implementation Tasks

### A) Sample Design

- [x] Define target state-machine flow and transitions in a short design note in `docs/`.
- [x] Identify which transitions are event-triggered vs persistent-model driven.
- [x] Identify which Playdate SDK calls are required for the sample scenario.
- Design note: [`docs/v0_4_m1_sample_design_note.md`](docs/v0_4_m1_sample_design_note.md)

### B) Sample Implementation

- [x] Implement/extend sample logic in `playdate_auto_bootstrap/src/game.cal` and/or `playdate_bouncing_ball/src/game.cal`.
- [x] Keep exported sample API aligned with current bootstrap expectations.
- [x] Ensure sample exercises core language features intentionally (sum types, `match`, updates).

### C) Binding Additions (If Required)

- [x] No new binding surfaces were required; sample behavior was implemented with existing bindings.
- [x] No `module_roots` updates were needed because existing shared bindings already covered the scenario.
- [x] No new binding APIs were introduced, so no additional binding-path regression tests were needed.

### D) Docs + Validation

- [x] Update sample README(s) with new behavior and run instructions.
- [x] Record `M1` status in `docs/v0_4_draft_plan.md`.
- [x] Run and record validation commands.

## Definition of Done (M1)

- [x] `cargo test` passes.
- [x] `make -C playdate_bouncing_ball build-lua` passes.
- [x] `make -C playdate_auto_bootstrap build-lua` passes.
- [x] Sample behavior demonstrates explicit transitions plus persisted model state.
- [x] No new binding APIs were required, and existing shared bindings remain exercised by sample code.
