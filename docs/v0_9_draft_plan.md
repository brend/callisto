# Callisto v0.9 Draft Plan

This document records the execution plan for `v0.9.0`.

`v0.9.0` is the compatibility-freeze and stabilization release before `v1.0.0`. It freezes the intended stable language surface, includes the already-started collection additions, audits emitted Lua and Playdate workflows, and creates the final `v1.0.0` readiness checklist.

Status: implementation validation complete as of 2026-04-26; awaiting maintainer sign-off/tagging.

## Why v0.9

- `v1.0.0` needs a clearly frozen language and prelude surface.
- The post-`v0.8` list additions should either be stabilized now or deferred; this release stabilizes them.
- Playdate-oriented generated Lua, bindings, templates, and samples need one final drift check before stable release.
- `v1.0.0` readiness should be explicit before release work starts.

## Release Guardrails

- No further syntax breaks are accepted in `v0.9.0`.
- Brace-delimited blocks remain the only accepted source block syntax.
- The frozen prelude is `Option[T]`, `Some`, `None`, `List[T]`, `length`, `map`, `append`, `filter`, and `fold`.
- List indexing uses Lua-style 1-based table indexes.
- Keep collection helpers as free functions; do not add method-style helpers.
- Do not add package management, broad Lua ecosystem support, or an LSP implementation.

## Scope (Must-Do)

1. Freeze and document the intended `v1.0.0` language surface.
2. Include list indexing plus `append`, `filter`, and `fold` in the frozen surface.
3. Publish a v1.0-oriented conformance matrix that maps frozen features to parser, resolver/typechecker, diagnostics, and Lua codegen coverage.
4. Audit emitted Lua and Playdate workflows for compatibility with the documented stable surface.
5. Update docs, samples, editor docs, and editor syntax fixtures for the frozen surface.
6. Create the `v1.0.0` readiness checklist.
7. Run compiler, CLI smoke, and editor regression checks before release.

## Out of Scope (v0.9)

- New syntax forms or syntax redesigns.
- Package or dependency management.
- Method-style collection helpers such as `xs.map(...)`.
- Additional large standard library/prelude expansion.
- Language-server implementation.
- Broad Lua backend redesign.

## Frozen v1.0 Language Surface

- Declarations: `type`, transparent aliases, `newtype`, `fn`, `pub`, `impl`, `extern type`, `extern fn`, and `extern module`.
- Types: primitives, `Unit`, function types, generics, records, sums, nominal wrappers, extern-only `Nil` and `not`, `Option[T]`, and `List[T]`.
- Expressions/statements: literals, locals, mutable locals, assignment, calls, method calls, field access, lambdas, `if`, `while`, `for`, `match`, record construction, record update, list literals, list indexing, string interpolation, and returns.
- Modules/imports: module declarations, module imports, item imports, configured module roots, and emitted multi-module Lua.
- Prelude: `Option[T]`, `Some`, `None`, `List[T]`, `length`, `map`, `append`, `filter`, and `fold`.
- Diagnostics policy: stable `CAL-*` codes for parser, resolver, typechecker, config, and workflow errors that are part of documented behavior.

## Milestones

1. `M0: Scope Freeze + Compatibility Policy`
- Lock v0.9 boundaries and compatibility policy.
- Checklist: [`docs/v0_9_m0_scope_freeze_checklist.md`](docs/v0_9_m0_scope_freeze_checklist.md)

2. `M1: Frozen Language Surface + Conformance`
- Publish the v1.0-oriented conformance matrix and classify any remaining gaps.
- Checklist: [`docs/v0_9_m1_conformance_freeze_checklist.md`](docs/v0_9_m1_conformance_freeze_checklist.md)

3. `M2: Lua + Playdate Stabilization`
- Audit emitted Lua, shared bindings, templates, and maintained samples.
- Checklist: [`docs/v0_9_m2_lua_playdate_stabilization_checklist.md`](docs/v0_9_m2_lua_playdate_stabilization_checklist.md)

4. `M3: Docs, Samples + Editor Drift`
- Align docs, sample references, and editor fixtures with the frozen surface.
- Checklist: [`docs/v0_9_m3_docs_editor_drift_checklist.md`](docs/v0_9_m3_docs_editor_drift_checklist.md)

5. `M4: v1.0 Readiness + Release Gate`
- Final validation, version metadata, changelog, and `v1.0.0` readiness checklist.
- Checklist: [`docs/v0_9_m4_release_checklist.md`](docs/v0_9_m4_release_checklist.md)

## Milestone Status

- `M0`: complete.
- `M1`: complete.
- `M2`: complete.
- `M3`: complete.
- `M4`: in progress.
