# Callisto v0.6 M4 Release Checklist

Execution checklist for `M4: Release Readiness`.

Use this as the release gate before cutting `v0.6.0`.

## Scope + Regression Gate

- [x] Freeze `v0.6` scope (no new feature work unless release-blocking).
- [x] Confirm all `M1`/`M2`/`M3` definition-of-done items are complete.
- [x] Run full regression and ensure no open release-blocking failures.

## Validation Commands

- [x] `cargo test`
- [x] `make -C playdate_bouncing_ball build-lua`
- [x] `make -C playdate_auto_bootstrap build-lua`
- [x] Release binary smoke:
  - `target/release/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
  - `target/release/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o <tmp_dir> --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`

## Release Artifacts

- [x] Build release binary with `cargo build --release`.
- [x] Finalize `CHANGELOG.md` `Unreleased` section into dated `0.6.0` release notes.
- [ ] Tag release commit as `v0.6.0`.
- [ ] Build and verify final release artifact from the tagged commit.

## Docs + Planning Consistency

- [x] `docs/v0_6_draft_plan.md` milestone statuses are current.
- [x] `docs/v0_6_m4_release_checklist.md` is linked from planning docs.
- [x] `README.md` release-planning links match active docs.

## Sign-Off

- [ ] Release checklist reviewed by maintainer.
- [ ] Release commit prepared (no unrelated workspace changes).
