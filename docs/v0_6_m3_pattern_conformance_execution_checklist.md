# Callisto v0.6 M3 Pattern Conformance Checklist

Execution checklist for `M3: Pattern Conformance Hardening`.

Use this as the implementation board for constructor/record-pattern validation depth.

## Scope Freeze

- [x] Keep `M3` focused on pattern-conformance and diagnostics hardening.
- [x] Avoid broad new control-flow or module-system features in this milestone.
- [x] Keep parser/typechecker behavior aligned with existing language-reference framing.

## M3 Decisions (Implement As Written)

1. Pattern validation behavior
- Tighten validation for constructor payload shape mismatches and record-pattern field issues.
- Keep diagnostics explicit about expected payload/field shape.

2. Fix-it diagnostic behavior
- Add actionable fix-it notes for common constructor/field mistakes (arity, typos, duplicates, missing fields).
- Prefer suggestion text that is directly editable by users.

3. Conformance behavior
- Build a language conformance matrix mapping syntax features to parser/checker/codegen tests.
- Every matrix entry must reference at least one concrete test location.

4. Regression behavior
- Add focused regression tests for edge-case pattern combinations that previously produced weak diagnostics.

## Implementation Tasks

### A) Pattern Validation

- [x] Audit constructor/record-pattern checking paths in `src/typecheck.rs`.
- [x] Improve diagnostics for field-name mistakes, payload-shape mismatches, and inconsistent pattern usage.
- [x] Add fix-it style notes for constructor arity and field-typo cases.
- [x] Ensure improvements do not introduce duplicate/noisy cascading errors.

### B) Conformance Matrix + Tests

- [x] Add `docs/v0_6_language_conformance_matrix.md`.
- [x] Populate matrix rows for declarations, expressions, patterns, modules, and extern interop.
- [x] Add/extend regression tests where matrix coverage gaps are identified.

### C) Docs

- [x] Update language examples in `README.md` and/or `docs/callisto_cheat_sheet.md` when behavior wording changes.
- [x] Record `M3` status in `docs/v0_6_draft_plan.md`.

## Definition of Done (M3)

- [x] `cargo test` passes with expanded conformance and pattern diagnostics coverage.
- [x] Conformance matrix exists and maps language surface areas to concrete tests.
- [x] Pattern diagnostics are actionable and stable for key mismatch cases, with fix-it notes for common mistakes.
