# Callisto v0.8 M4 Release Readiness Checklist

## Validation

- [x] `cargo test`
- [x] `target/debug/callisto check samples/imports_extern_interop.cal`
- [x] `target/debug/callisto emit-lua samples/imports_extern_interop.cal -o /tmp/callisto_v0_8_imports.lua`
- [x] `target/debug/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
- [x] `target/debug/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o /tmp/callisto_v0_8_smoke --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`
- [x] VS Code grammar regression passes.
- [x] Zed grammar regression passes.

## Docs And Metadata

- [x] `CHANGELOG.md` contains a dated `0.8.0` entry.
- [x] `Cargo.toml` version is updated for release.
- [x] README, cheat sheet, Playdate workflow docs, and editor docs describe the v0.8 surface.
- [x] `docs/v0_8_draft_plan.md` milestone statuses are current.

## Release Gate

- [x] No unresolved release-blocking diagnostics or test failures remain.
- [ ] Maintainer sign-off is recorded before tagging.
