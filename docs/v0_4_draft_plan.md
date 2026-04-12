# Callisto v0.4 Draft Plan

This document defines the first draft for `v0.4.0` planning.

`v0.3.0` completed the first-party Playdate bootstrap/template/build workflow.  
`v0.4.0` focuses on product-depth polish, parser ergonomics, and type-inference quality.

## Why v0.4

- Expand sample realism to validate longer-lived Playdate gameplay loops.
- Grow bindings only when driven by concrete sample requirements.
- Reduce syntax friction and annotation overhead in common language patterns.

## Proposed v0.4.0 Scope Candidates

1. Richer sample coverage with state persistence and explicit transitions beyond input-hold states.
2. Continue sample-driven binding additions only as required by richer samples.
3. Parser ergonomics: allow more flexible ADT/sum formatting (for example, permitting variants on lines after `=` without requiring the first `|` on the same line).
4. Type inference: improve inference for nullary generic constructors (for example `Nil`) in nested/record-initializer contexts so explicit temporary annotations are less often required.

## Milestone Sketch

1. `M1: Sample Depth + Binding Gaps`
- Implement richer Playdate sample state machines.
- Add only binding modules needed by those samples.

2. `M2: Parser Ergonomics`
- Relax ADT/sum formatting constraints while preserving diagnostic clarity.
- Add parser regression coverage for multi-line variant declarations.

3. `M3: Type Inference Quality`
- Improve inference for nullary generic constructors in nested contexts.
- Add targeted checker regression tests for prior annotation-heavy cases.

4. `M4: Release Readiness`
- Regression/build pass, docs/changelog finalization, and release prep for `v0.4.0`.
