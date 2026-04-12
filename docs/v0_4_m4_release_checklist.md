# Callisto v0.4 M4 Release Checklist

Execution checklist for `M4: Release Readiness`.

Use this as the release gate before cutting `v0.4.0`.

## Scope Freeze

- [x] Freeze `v0.4` scope (no new feature work unless release-blocking).
- [x] Confirm `M0`/`M1`/`M2`/`M3` docs reflect implemented behavior.
- [x] Confirm `CHANGELOG.md` tracks `v0.4` additions/changes/fixes only.

## Quality Gate

- [x] `cargo test` passes on the release candidate branch.
- [x] `make -C playdate_bouncing_ball build-lua` passes.
- [x] `make -C playdate_auto_bootstrap build-lua` passes.
- [x] Debug binary smoke:
  - `target/debug/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
  - `target/debug/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o <tmp_dir> --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`
- [x] Release binary smoke:
  - `target/release/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
  - `target/release/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o <tmp_dir> --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`

## Release Artifacts

- [x] Build release binary with `cargo build --release`.
- [x] Smoke-test `target/release/callisto` for representative `check` and `emit-lua` paths.
- [x] Finalize `CHANGELOG.md` `Unreleased` section into dated `0.4.0` release notes.
- [x] Tag release commit as `v0.4.0`.
- [x] Build and verify final release artifact from the tagged commit.

## Documentation

- [x] README documents final `v0.4` syntax/inference/sample updates.
- [x] `docs/callisto_cheat_sheet.md` and Playdate workflow docs reflect final behavior.
- [x] `docs/v0_4_draft_plan.md` milestone statuses are current.
- [x] `docs/v0_4_m4_release_checklist.md` is linked from planning docs.

## Sign-off

- [x] Release checklist reviewed by maintainer.
- [x] Release commit prepared (no unrelated workspace changes).
- [x] `v0.4.0` announcement notes drafted.
