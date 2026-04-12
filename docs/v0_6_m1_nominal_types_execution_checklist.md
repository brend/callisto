# Callisto v0.6 M1 Nominal Types Checklist

Execution checklist for `M1: Nominal Type Identity`.

Use this as the implementation board for `newtype` support.

## Scope Freeze

- [ ] Keep `M1` focused on nominal wrappers and assignability semantics.
- [ ] Preserve transparent alias behavior for existing `type Alias = ...` declarations.
- [ ] Avoid unrelated parser/typechecker expansion in this milestone.

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

- [ ] Add parser/AST support for `newtype` declarations.
- [ ] Keep declaration grammar backward-compatible for existing `type` forms.
- [ ] Add parser regression tests for valid and malformed `newtype` declarations.

### B) Resolver + Typechecker

- [ ] Represent nominal identity in type metadata.
- [ ] Enforce nominal assignability rules in assignment/call/return paths.
- [ ] Ensure generic/unification behavior remains deterministic with mixed alias/newtype usage.

### C) Codegen + Docs

- [ ] Verify emitted Lua for `newtype` flows remains predictable and readable.
- [ ] Update `README.md` language reference with `newtype` syntax/behavior.
- [ ] Record `M1` status in `docs/v0_6_draft_plan.md`.

## Definition of Done (M1)

- [ ] `cargo test` passes with new `newtype` coverage.
- [ ] Nominal-type mismatch diagnostics are clear and stable.
- [ ] Existing alias-based codepaths remain backward-compatible.
