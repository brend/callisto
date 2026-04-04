# Callisto v0.2 Draft Plan

This document defines a practical first draft for `v0.2` planning. It is intentionally concrete enough to execute, while still allowing scope adjustments before implementation starts.

## Why v0.2

`v0.1` established a stable typed-to-Lua pipeline and core language ergonomics.  
`v0.2` should focus on project-scale usability and release confidence:

- stronger module/import ergonomics for real projects
- clearer diagnostics and testing at scale
- explicit packaging/release workflow

## Proposed v0.2 Goals (Must-Do)

### 1) Configurable module resolution roots

Current state: module resolution is entry-root-relative only.

Deliverables:
- Add configurable import roots (for example via CLI flag and/or project config file).
- Keep existing `foo.bar -> foo/bar.{luna|cal}` behavior, but search across configured roots in deterministic order.
- Improve diagnostics to show attempted lookup paths when import resolution fails.

Acceptance criteria:
- Deterministic resolution order with tests.
- Existing `v0.1` module-loading behavior remains backward-compatible by default.

### 2) Project config file (minimal)

Deliverables:
- Add a minimal project manifest (for example `callisto.toml`) containing:
  - module roots
  - output directory default
  - optional package/module name
- CLI commands read config when present and allow explicit flags to override it.

Acceptance criteria:
- Config parsing failures produce actionable diagnostics with file/line location.
- `check` and `emit-lua` behave consistently with and without config overrides.

### 3) Diagnostic quality pass (project-scale)

Deliverables:
- Consistent diagnostics format for resolve/typecheck errors.
- Add secondary notes for common fix paths (missing import member, wrong constructor payload shape, wrong alias/type argument usage).
- Add error-code tags (lightweight identifiers) to make docs and issue triage easier.

Acceptance criteria:
- Golden tests for representative diagnostics in resolver/typechecker.
- No regressions in current error clarity from `v0.1`.

### 4) Release/test hardening

Deliverables:
- Add integration tests that compile multi-module sample projects end-to-end.
- Add snapshot/golden coverage for emitted Lua in key language features.
- Add a release checklist doc for tagging and artifact verification.

Acceptance criteria:
- CI-equivalent local command passes deterministically.
- Clear release gate for `v0.2` documented in this file.

## Candidate Stretch Goals (Only if Must-Do is complete)

- Nominal/newtype alias mode (opt-in, explicitly non-default initially).
- Better module privacy controls beyond `pub`.
- Optional Lua target profile switches (e.g. output style/perf knobs).

## Non-Goals for v0.2

- Full package registry ecosystem.
- Incremental compilation daemon/watch mode.
- Major IR/pipeline redesign (keep AST -> resolve/typecheck -> TIR -> Lua flow).

## Milestones

1. `M1: Config + Resolver Foundation`
- Config loader + schema validation
- Import root search and diagnostics
- Backward-compat tests
- Execution checklist: [`docs/v0_2_m1_execution_checklist.md`](docs/v0_2_m1_execution_checklist.md)

2. `M2: CLI + UX Integration`
- CLI/config precedence rules
- Output path behavior cleanup
- User-facing docs update

3. `M3: Diagnostics + Goldens`
- Error-code tags
- Golden diagnostics tests
- Golden emitted-Lua tests

4. `M4: Release Readiness`
- Full regression pass
- Release checklist completion
- Changelog + tag preparation

## Proposed v0.2 Completion Gate

Call `v0.2` complete when all of the following are true:

1. All Must-Do goals in this document are checked.
2. `cargo test` passes, including new integration/golden tests.
3. `cargo run -- check` passes on all sample projects and new multi-module fixtures.
4. `cargo run -- emit-lua` emits deterministic output for golden fixtures.
5. README and docs match the final behavior (config, module resolution, diagnostics format).

## Immediate Next Tasks

1. Decide config filename/schema (`callisto.toml`) and CLI precedence rules.
2. Implement resolver root-search abstraction behind current import loading.
3. Add first integration fixture with at least three modules across two roots.
4. Add diagnostics golden harness for resolver/typechecker errors.
