# Callisto v0.7 M4 Release Readiness Checklist

## Validation

- [ ] `cargo test`
- [ ] `cargo build --release`
- [ ] `target/release/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
- [ ] `target/release/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o <tmp_dir> --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`
- [ ] `target/release/callisto check playdate_bouncing_ball/src/game.cal --config playdate_bouncing_ball/callisto.toml`

## Docs And Metadata

- [ ] `CHANGELOG.md` contains a dated `0.7.0` entry.
- [ ] `Cargo.toml` version is updated for release.
- [ ] README, cheat sheet, and Playdate workflow docs describe the v0.7 surface.
- [ ] `docs/v0_7_draft_plan.md` milestone statuses are current.

## Release Gate

- [ ] No unresolved release-blocking diagnostics or test failures remain.
- [ ] Maintainer sign-off is recorded before tagging.
