# Callisto v0.6 M0 Scope Freeze Checklist

Execution checklist for `M0: Scope Freeze + Conformance Baseline`.

Use this as the planning gate before implementation milestones start.

## Scope Freeze

- [ ] Confirm `v0.6.0` must-ship tracks:
  - nominal type identity via `newtype`
  - stronger match-analysis diagnostics (exhaustiveness + reachability)
  - pattern-conformance hardening and coverage depth
- [ ] Confirm no new major non-language tracks are introduced before `M4`.
- [ ] Confirm non-goals are explicit in [`docs/v0_6_draft_plan.md`](docs/v0_6_draft_plan.md).

## M0 Decisions (Implement As Written)

1. Language-first policy
- `v0.6` work prioritizes language semantics/completeness over tooling expansion.
- Non-language items are only allowed if release-blocking for language work.

2. Compatibility policy
- Existing valid syntax and alias behavior remain stable unless an additive change is explicitly documented.
- New diagnostics may be stricter, but should include actionable remediation notes.

3. Test-quality floor
- Every milestone keeps `cargo test` green before moving to the next milestone.
- Language conformance matrix entries must map to concrete test coverage.

## Validation Matrix (Must Pass Before M4)

- [ ] `cargo test`
- [ ] `target/debug/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
- [ ] `target/debug/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o /tmp/callisto_v0_6_smoke --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`

## Exit Criteria (M0)

- [ ] `docs/v0_6_draft_plan.md` links all milestone checklists.
- [ ] `README.md` links active `v0.6` planning docs.
- [ ] Maintainer confirms `M0` freeze before `M1` implementation work.
