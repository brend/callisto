# Callisto v0.3 M4 Release Checklist

Execution checklist for `M4: Release Readiness`.

Use this as the release gate before cutting `v0.3.0`.

## Scope Freeze

- [x] Freeze `v0.3` scope (no new feature work unless release-blocking).
- [x] Confirm `M1`/`M2`/`M3` docs reflect implemented behavior.
- [x] Confirm `CHANGELOG.md` tracks `v0.3` additions/changes/fixes.

## Quality Gate

- [x] `cargo test` passes on the release candidate branch.
- [x] `make -C playdate_bouncing_ball build-lua` passes.
- [x] `make -C playdate_auto_bootstrap build-lua` passes.
- [x] Debug binary smoke:
  - `target/debug/callisto init --template playdate <tmp>`
  - `target/debug/callisto build-playdate ... --pdc <fake-pdc>`
- [x] Release binary smoke:
  - `target/release/callisto check <sample>`
  - `target/release/callisto emit-lua <sample> --playdate-bootstrap`
  - `target/release/callisto init --template playdate <tmp>`
  - `target/release/callisto build-playdate ... --pdc <fake-pdc>`

## Release Artifacts

- [x] Build release binary with `cargo build --release`.
- [x] Smoke-test `target/release/callisto` for `check`, `emit-lua`, `init`, and `build-playdate`.
- [ ] Finalize `CHANGELOG.md` `Unreleased` section into `0.3.0` dated entry.
- [ ] Tag release commit as `v0.3.0`.
- [ ] Build and verify final release artifact from the tagged commit.

## Documentation

- [x] README documents first-party Playdate template/build workflow.
- [x] `docs/playdate_workflow.md` and `docs/callisto_cheat_sheet.md` reflect `build-playdate` usage.
- [x] `docs/v0_3_draft_plan.md` milestone statuses are current.
- [x] `docs/v0_3_m4_release_checklist.md` added and linked from planning docs.

## Sign-off

- [ ] Release checklist reviewed by maintainer.
- [ ] Release commit prepared (no unrelated workspace changes).
- [ ] `v0.3.0` announcement notes drafted.
