# Callisto v0.2 M4 Release Checklist

Execution checklist for `M4: Release Readiness`.

Use this as the release gate before cutting `v0.2.0`.

## Scope Freeze

- [x] Freeze `v0.2` scope (no new feature work unless release-blocking).
- [x] Confirm candidate stretch goals are deferred.
- [x] Confirm docs and changelog reflect implemented behavior only.

## Quality Gate

- [x] `cargo test` passes on the release candidate branch.
- [x] `cargo run -- check <entry>` passes on representative sample projects.
- [x] `cargo run -- emit-lua <entry>` output matches Lua goldens for key fixtures.
- [x] Verify deterministic module resolution behavior with config/CLI overrides.

## Release Artifacts

- [x] Finalize `CHANGELOG.md` `Unreleased` section into `0.2.0` dated entry.
- [x] Tag release commit as `v0.2.0`.
- [x] Build release binary with `cargo build --release`.
- [x] Smoke-test `target/release/callisto` for `check` and `emit-lua`.

## Documentation

- [x] README documents final `v0.2` config and diagnostics behavior.
- [x] `docs/v0_2_draft_plan.md` milestone statuses are final.
- [x] Any sample workflow docs referencing pre-v0.2 behavior are updated.

## Sign-off

- [x] Release checklist reviewed by maintainer.
- [x] Release commit prepared (no unrelated workspace changes).
- [x] `v0.2.0` announcement notes drafted.
