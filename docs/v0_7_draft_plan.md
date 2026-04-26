# Callisto v0.7 Draft Plan

This document defines the active execution plan for `v0.7.0`.

`v0.6.1` completed the core language-completeness foundation.  
`v0.7.0` focuses on the first standard prelude surface and more reliable project workflows.

Status: active.

## Why v0.7

- Real programs need one standard representation for optional values and lists.
- Playdate samples should use shared language/library surface instead of redefining small utility types.
- Project configuration, module resolution, output paths, and Playdate build failures should be predictable and actionable.

## Release Guardrails

- Keep the prelude implicit and small: `Option[T]`, `Some`, `None`, `List[T]`, `length`, and `map`.
- Reject user declarations that conflict with reserved prelude names.
- Keep `List[T]` backed by Lua array-style tables.
- Keep `map` as the helper form `map(list, fn)` only in this release.
- Require contextual type information for empty list literals.
- Avoid package management, brace syntax, broad editor work, and large workflow redesigns in this release.

## Scope (Must-Do)

1. Add built-in/predefined `Option[T]` with `Some(T)` and `None`.
2. Add built-in/predefined `List[T]` with list literals.
3. Add prelude helpers:
  - `length(xs: List[T]) -> Int`
  - `map(xs: List[A], f: fn(A) -> B) -> List[B]`
4. Reject declarations of reserved prelude names: `Option`, `Some`, `None`, `List`, `length`, and `map`.
5. Document the initial prelude surface and migration away from project-local `Option` definitions.
6. Harden `callisto.toml`, module-root/import-resolution, output-directory, and Playdate build diagnostics.
7. Keep generated Playdate templates, sample projects, README snippets, and workflow docs aligned.

## Out of Scope (v0.7)

- Package/dependency management.
- Brace-delimited block syntax.
- Additional collection helpers beyond `length` and `map`.
- Method forms such as `xs.map(...)`.
- Broad editor grammar updates beyond docs references for the new prelude names.

## Milestones

1. `M0: Scope Freeze + Prelude Design`
- Lock `v0.7.0` boundaries and prelude rules.
- Document reserved names, list literal typing, and helper signatures.
- Checklist: [`docs/v0_7_m0_scope_freeze_checklist.md`](docs/v0_7_m0_scope_freeze_checklist.md)

2. `M1: Option Prelude`
- Add built-in `Option[T]`, `Some(T)`, and `None`.
- Reject project-local conflicts with prelude names.
- Migrate docs/tests/samples away from local `Option` definitions.
- Checklist: [`docs/v0_7_m1_option_prelude_execution_checklist.md`](docs/v0_7_m1_option_prelude_execution_checklist.md)

3. `M2: List Literals + Helpers`
- Add `List[T]`, list literals, `length`, and `map`.
- Emit normal Lua array tables and helper-specific Lua.
- Add parser/checker/codegen coverage.
- Checklist: [`docs/v0_7_m2_list_helpers_execution_checklist.md`](docs/v0_7_m2_list_helpers_execution_checklist.md)

4. `M3: Workflow Reliability`
- Improve config, module-root/import, output path, and Playdate build diagnostics.
- Add CLI/config and generated Playdate flow regression coverage.
- Checklist: [`docs/v0_7_m3_workflow_reliability_execution_checklist.md`](docs/v0_7_m3_workflow_reliability_execution_checklist.md)

5. `M4: Release Readiness`
- Regression/build pass, docs/changelog finalization, and release prep for `v0.7.0`.
- Checklist: [`docs/v0_7_m4_release_checklist.md`](docs/v0_7_m4_release_checklist.md)

## Milestone Status

- `M0`: complete.
- `M1`: complete.
- `M2`: complete.
- `M3`: in progress.
- `M4`: not started.
