# Callisto v0.6 M1 Nominal Types Checklist

Execution checklist for `M1: Nominal Type Identity`.

Use this as the implementation board for `newtype` support.

## Scope Freeze

- [x] Keep `M1` focused on nominal wrappers and assignability semantics.
- [x] Preserve transparent alias behavior for existing `type Alias = ...` declarations.
- [x] Avoid unrelated parser/typechecker expansion in this milestone.

## M1 Decisions (Implement As Written)

1. Language behavior
- Add a `newtype` declaration form with nominal identity distinct from aliases.
- Nominal types are not implicitly assignable to/from underlying representation types.

2. Interop behavior
- Keep runtime representation minimal/no-overhead where possible.
- Ensure generated Lua remains compatible with existing module/extern flows.

3. Diagnostics behavior
- Assignment/call mismatches involving `newtype` should clearly state expected nominal type vs underlying type.

## Implementation Tasks

### A) Parser + AST

- [x] Add parser/AST support for `newtype` declarations.
- [x] Keep declaration grammar backward-compatible for existing `type` forms.
- [x] Add parser regression tests for valid and malformed `newtype` declarations.

### B) Resolver + Typechecker

- [x] Represent nominal identity in type metadata.
- [x] Enforce nominal assignability rules in assignment/call/return paths.
- [x] Ensure generic/unification behavior remains deterministic with mixed alias/newtype usage.

### C) Codegen + Docs

- [x] Verify emitted Lua for `newtype` flows remains predictable and readable.
- [x] Update `README.md` language reference with `newtype` syntax/behavior.
- [x] Record `M1` status in `docs/v0_6_draft_plan.md`.

## Definition of Done (M1)

- [x] `cargo test` passes with new `newtype` coverage.
- [x] Nominal-type mismatch diagnostics are clear and stable.
- [x] Existing alias-based codepaths remain backward-compatible.
