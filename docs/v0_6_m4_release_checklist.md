# Callisto v0.6 M4 Release Checklist

Execution checklist for `M4: Release Readiness`.

Use this as the release gate before cutting `v0.6.0`.

## Scope + Regression Gate

- [ ] Freeze `v0.6` scope (no new feature work unless release-blocking).
- [ ] Confirm all `M1`/`M2`/`M3` definition-of-done items are complete.
- [ ] Run full regression and ensure no open release-blocking failures.

## Validation Commands

- [ ] `cargo test`
- [ ] `make -C playdate_bouncing_ball build-lua`
- [ ] `make -C playdate_auto_bootstrap build-lua`
- [ ] Release binary smoke:
  - `target/release/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
  - `target/release/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o <tmp_dir> --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`

## Release Artifacts

- [ ] Build release binary with `cargo build --release`.
- [ ] Finalize `CHANGELOG.md` `Unreleased` section into dated `0.6.0` release notes.
- [ ] Tag release commit as `v0.6.0`.
- [ ] Build and verify final release artifact from the tagged commit.

## Docs + Planning Consistency

- [ ] `docs/v0_6_draft_plan.md` milestone statuses are current.
- [ ] `docs/v0_6_m4_release_checklist.md` is linked from planning docs.
- [ ] `README.md` release-planning links match active docs.

## Sign-Off

- [ ] Release checklist reviewed by maintainer.
- [ ] Release commit prepared (no unrelated workspace changes).
