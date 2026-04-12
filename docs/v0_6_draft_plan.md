# Callisto v0.6 Draft Plan

This document defines the active execution plan for `v0.6.0`.

`v0.5.0` completed deeper Playdate workflow/productization paths.  
`v0.6.0` focuses on language completeness and semantic consistency.

Status: active.

## Why v0.6

- Prior releases improved workflow and ergonomics; the next bottleneck is language-surface completeness.
- The language reference should map cleanly to checker/codegen behavior with fewer "edge-case surprises."
- Closing long-standing semantic gaps is higher value than adding new tooling tracks in this cycle.

## Release Guardrails

- Keep `v0.6.0` focused on three must-ship language tracks only:
  - nominal type identity (`newtype`) alongside existing transparent aliases
  - stronger match analysis (exhaustiveness + reachability diagnostics)
  - pattern-conformance hardening (constructor/record-pattern validation + regression depth)
- Avoid package/dependency-management or build-system expansion in this release.
- Keep existing syntax backward-compatible unless explicitly documented as additive.

## Scope (Must-Do)

1. Add nominal wrappers (`newtype`) so domain-specific type distinctions are expressible without runtime overhead.
2. Add parser QoL ergonomics:
  - trailing commas for multiline lists (parameters, arguments, payloads, match arms)
  - record field punning in initializers (`Point { x }` -> `Point { x = x }`)
3. Expand match diagnostics to catch unreachable/duplicate constructor arms and tighten Bool-pattern exhaustiveness behavior.
4. Harden pattern semantics for constructor payloads and record-constructor fields with clearer diagnostics and broader regression coverage.
5. Improve fix-it diagnostics for common mistakes (constructor arity, field typos/duplicates/missing fields).
6. Publish a language conformance matrix tying each language-reference construct to parser/checker/codegen tests.

## Out of Scope (v0.6)

- New package/dependency management.
- Incremental/watch compilation.
- Broad syntax redesign outside the targeted language-completeness tracks above.

## Milestones

1. `M0: Scope Freeze + Conformance Baseline`
- Lock `v0.6.0` boundaries and validation matrix.
- Define initial language-conformance baseline and required test commands.
- Conformance matrix: [`docs/v0_6_language_conformance_matrix.md`](docs/v0_6_language_conformance_matrix.md)
- Checklist: [`docs/v0_6_m0_scope_freeze_checklist.md`](docs/v0_6_m0_scope_freeze_checklist.md)

2. `M1: Nominal Type Identity`
- Introduce `newtype` semantics and typechecker/codegen handling.
- Preserve existing alias behavior for `type Alias = ...`.
- Checklist: [`docs/v0_6_m1_nominal_types_execution_checklist.md`](docs/v0_6_m1_nominal_types_execution_checklist.md)

3. `M2: Match Analysis Completeness`
- Improve exhaustiveness/reachability diagnostics for constructor and Bool-pattern matches.
- Deliver parser QoL additions: trailing commas in multiline lists and record field punning.
- Add targeted regression coverage for duplicate and dead-arm cases plus new syntax acceptance paths.
- Checklist: [`docs/v0_6_m2_match_analysis_execution_checklist.md`](docs/v0_6_m2_match_analysis_execution_checklist.md)

4. `M3: Pattern Conformance Hardening`
- Tighten constructor/record pattern validation and diagnostics quality.
- Improve fix-it notes for common constructor/pattern/field mistakes.
- Expand language-conformance matrix coverage for pattern-heavy paths.
- Checklist: [`docs/v0_6_m3_pattern_conformance_execution_checklist.md`](docs/v0_6_m3_pattern_conformance_execution_checklist.md)

5. `M4: Release Readiness`
- Regression/build pass, docs/changelog finalization, and release prep for `v0.6.0`.
- Checklist: [`docs/v0_6_m4_release_checklist.md`](docs/v0_6_m4_release_checklist.md)

## Milestone Status

- `M0`: in progress.
- `M1`: pending.
- `M2`: pending.
- `M3`: pending.
- `M4`: pending.
