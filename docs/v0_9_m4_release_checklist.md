# Callisto v0.9 M4 v1.0 Readiness + Release Gate Checklist

## Validation

- [x] `cargo test`
- [x] `target/debug/callisto check samples/imports_extern_interop.cal`
- [x] `target/debug/callisto emit-lua samples/imports_extern_interop.cal -o /tmp/callisto_v0_9_imports.lua`
- [x] `target/debug/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
- [x] `target/debug/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o /tmp/callisto_v0_9_smoke --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`
- [x] VS Code grammar regression passes.
- [x] Zed grammar regression passes.

## Docs And Metadata

- [x] `CHANGELOG.md` contains a dated `0.9.0` entry.
- [x] `Cargo.toml` version is updated for release.
- [x] `docs/v0_9_draft_plan.md` milestone statuses are current.
- [x] `docs/v1_0_language_conformance_matrix.md` reflects the frozen surface.
- [x] `docs/v1_0_readiness_checklist.md` exists.

## Release Gate

- [x] No unresolved v0.9 release-blocking diagnostics or test failures remain.
- [x] No unresolved `v1.0.0` readiness blockers are listed.
- [ ] Maintainer sign-off is recorded before tagging.
