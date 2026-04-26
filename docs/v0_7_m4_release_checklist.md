# Callisto v0.7 M4 Release Readiness Checklist

## Validation

- [x] `cargo test`
- [x] `cargo build --release`
- [x] `target/release/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
- [x] `target/release/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o <tmp_dir> --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`
- [x] `target/release/callisto check playdate_bouncing_ball/src/game.cal --config playdate_bouncing_ball/callisto.toml`

## Docs And Metadata

- [x] `CHANGELOG.md` contains a dated `0.7.0` entry.
- [x] `Cargo.toml` version is updated for release.
- [x] README, cheat sheet, and Playdate workflow docs describe the v0.7 surface.
- [x] `docs/v0_7_draft_plan.md` milestone statuses are current.

## Release Gate

- [x] No unresolved release-blocking diagnostics or test failures remain.
- [x] Maintainer sign-off is recorded before tagging.

Validation recorded on 2026-04-26. Tagging is a separate git operation.
