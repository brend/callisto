# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- `--playdate-bootstrap` flag on `emit-lua`/`build` to generate a Playdate `main.lua` shim for directory outputs.
- Validation for Playdate bootstrap generation (requires entry module `pub fn update() -> Unit` and avoids `main.lua` overwrite collisions).
- Regression tests for Playdate bootstrap emission and CLI parsing.
- Shared Playdate bindings package under `playdate_bindings/src`.
- New `playdate_auto_bootstrap/` sample project that exercises auto-shim output.
- `v0.3` planning docs: draft plan and `M1` execution checklist.
- Shared `playdate.input` binding helper module.
- Shared `playdate.audio` binding helper module (sample-driven sound wrappers).
- Shared `playdate.system` binding helper module (crank-position wrappers).
- `v0.3` `M2` binding-execution checklist.
- Regression tests for emitted Lua paths in `playdate.input`, `playdate.audio`, and `playdate.system` bindings.
- `callisto init --template playdate <dir>` to scaffold first-party Playdate project structure.
- `callisto build-playdate <entry.cal>` command for one-step Lua emission + `pdc` packaging (+ optional `--run`).
- `v0.3` `M3` Playdate product execution checklist.

### Changed
- Playdate workflow docs and CLI cheat sheet now document bootstrap flow and shared binding module-root usage.
- Playdate workflow docs now treat `build-playdate` as the first-party build path while keeping manual flow documented.
- `playdate_auto_bootstrap` now demonstrates a richer multi-scene HUD flow with crank telemetry labels.
- `playdate_bouncing_ball` now consumes shared bindings via `module_roots`.
- `playdate_bouncing_ball` gameplay now exercises more language features (sum types, `match`, `impl`, and record updates).
- `playdate_bouncing_ball` now includes sample-driven sound effects and a generic helper in gameplay flow.
- `playdate_bouncing_ball` now integrates system wrappers and displays a crank-side indicator.

### Fixed
- `playdate_bouncing_ball` audio path no longer crashes when runtime sound APIs are unavailable; manual shim now guards unsafe calls.

## [0.2.0] - 2026-04-04

### Added
- Project config loading via `callisto.toml` with `module_roots`, `out_dir`, and optional `package`.
- CLI support for `--config` and repeatable `--module-root` on `check`, `emit-lua`, and `build`.
- Multi-root module lookup with attempted-path notes for unresolved imports.
- Golden diagnostics fixtures under `tests/golden/diagnostics`.
- Golden emitted-Lua fixtures under `tests/golden/lua`.
- v0.2 milestone docs for diagnostics and release readiness.

### Changed
- Output directory precedence is now deterministic: `-o` overrides config `out_dir`, otherwise config/default applies.
- Diagnostics now support stable machine-readable error codes (for example `CAL-RES-*`, `CAL-TYP-*`, `CAL-CFG-*`).

### Fixed
- Reduced cascading duplicate diagnostics for imported-item call failures when a primary import/declaration error is already reported.

## [0.1.0] - 2026-04-04

Initial `v0.1` release.

### Added
- End-to-end compiler pipeline: lexer -> parser -> resolver -> typechecker -> TIR -> Lua codegen.
- CLI commands: `parse`, `check`, `emit-lua`, and `build` (alias of `emit-lua`).
- Recursive multi-file module loading from an entry file.
- Core language support for records, sum types, pattern matching, generics, methods (`impl`), lambdas, and record updates.
- Extern interop via `extern module` and `extern fn`.

### Changed
- README `v0.1` scope is now explicit about supported features, exclusions, and expected CLI behavior.
- `v0.1` completion checklist updated to reflect completed release-quality tasks.

### Fixed
- Improved diagnostics for import/module misuse, including calls on imported module aliases.
- Added targeted diagnostic notes for constructor and record payload/field mismatches.
- Expanded negative-path regression coverage for:
  - generic ADT inference failures
  - alias mismatch failures
  - import module/item misuse
  - non-exhaustive generic `match`
