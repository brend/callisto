# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [0.6.0] - 2026-04-15

### Added
- `v0.6` draft planning docs centered on language completeness:
  - `docs/v0_6_draft_plan.md`
  - `docs/v0_6_m0_scope_freeze_checklist.md`
  - `docs/v0_6_m1_nominal_types_execution_checklist.md`
  - `docs/v0_6_m2_match_analysis_execution_checklist.md`
  - `docs/v0_6_m3_pattern_conformance_execution_checklist.md`
  - `docs/v0_6_m4_release_checklist.md`
- Initial language conformance matrix scaffold in `docs/v0_6_language_conformance_matrix.md`.

### Changed
- Parser now accepts trailing commas in multiline list contexts across:
  - function/type parameter lists
  - call arguments and constructor payload arguments
  - match arms (optional trailing comma after each `case` arm)
- Record initializer field punning is now supported (`Point { x }` is treated as `Point { x = x }`).
- Type diagnostics now include stronger fix-it notes for common mistakes:
  - constructor argument arity mismatch includes a suggested constructor call shape
  - unknown/duplicate/missing record fields include more actionable correction notes
- Added nominal `newtype` declarations (`newtype Name = Inner`) with constructor support (`Name(value)`).
- Newtype values now remain nominally distinct from their underlying types in assignability checks.
- Match analysis now reports duplicate constructor arms (`CAL-TYP-031`) and unreachable arms (`CAL-TYP-032`) with actionable notes.
- Bool `match` exhaustiveness now reports missing literal cases under `CAL-TYP-030`.
- Added regression and golden diagnostics coverage for duplicate/unreachable/bool match-analysis paths.
- Constructor-pattern diagnostics now include stable error codes:
  - `CAL-TYP-023` for pattern payload-shape mismatches and constructor-pattern arity mismatch fix-its.
  - `CAL-TYP-024` for constructor-pattern record-field unknown/duplicate/missing diagnostics.
- Constructor-pattern shape errors now avoid noisy cascades while still binding pattern locals in error paths.
- Expanded v0.6 language conformance matrix with concrete parser/typechecker/codegen test references across declarations, expressions, patterns, modules, and extern interop.
- Regression test for all-shorthand record field punning in initializers (`record_field_punning_all_shorthand_compiles_and_codegen`).
- `playdate_bouncing_ball` sample now demonstrates `v0.6` language features: `newtype SfxCode = Int` for nominal sound-effect codes, trailing commas in multiline type field lists, and all-shorthand field punning (`Integrated { ball, sfx }`).

## [0.5.0] - 2026-04-12

### Added
- Shared `playdate.graphics` bindings now include `drawLine(x1, y1, x2, y2)`.
- Shared `playdate.graphics` bindings now include shape primitives `drawRect(x, y, width, height)` and `fillRect(x, y, width, height)`.
- Richer sample flow now explicitly exercises shared `playdate.timer.updateTimers()` usage in update loops.
- Playdate bootstrap customization flags:
  - `--playdate-bootstrap-target <lua.path>`
  - `--playdate-bootstrap-preload <module/path|lua.path=module/path>` (repeatable)
- Playdate template init options:
  - `--workflow auto-bootstrap|manual-shim`
  - `--starter-assets` for starter `Source/images`, `Source/sounds`, and `Source/fonts` folders.

### Changed
- `playdate_auto_bootstrap` now runs a richer mission-style loop with persisted resources/progression (`energy`, `heat`, `score`, `combo`, `laps`) and renders HUD overlays/gauges using shared graphics bindings (`drawLine`, `drawRect`, `fillRect`).
- Generated Playdate bootstrap `main.lua` can now emit optional preload imports/assignments before entry import and assign update logic to a custom target (default remains `playdate.update`).
- `callisto init --template playdate` now generates workflow-specific Makefiles/READMEs and can emit a manual `Source/main.lua` shim template.

## [0.4.0] - 2026-04-12

### Added
- `v0.4` draft planning document with scope candidates carried over from post-`v0.3.0` follow-on work.
- `v0.4` execution-board docs (`M0` scope freeze, `M1` sample depth, `M2` parser ergonomics, `M3` type inference, `M4` release gate) plus README planning links.

### Changed
- `v0.3` release checklist marked complete after maintainer review and post-release planning moved to `v0.4`.
- Repository metadata/docs now align with the completed `v0.3.0` release (`Cargo.toml` version and README release-planning links).
- `playdate_auto_bootstrap` now uses explicit `buttonJustPressed` scene transitions (`A` next, `B` previous) and persists session counters (`ticks`, scene changes, per-scene frame totals) in its model.
- Parser now accepts multiline sum-type declarations after `=` (for example `type Option[T] =` followed by `|` variants on subsequent lines) while preserving existing single-line syntax.
- Type inference now propagates contextual field/payload types into nested expressions, reducing annotation needs for nullary generic constructors (for example `None`) inside record initializers, constructor payloads, and record updates.

## [0.3.0] - 2026-04-04

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
- `v0.3` `M4` release-readiness checklist.

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
