# Callisto v0.1 Completion Checklist

This checklist tracks what is needed to call v0.1 complete against the architecture in `docs/luna_compiler_architecture_v0_1.md`.

## Must-Do Before v0.1

### 1. Generic ADT construction must preserve type arguments
- [x] Infer and propagate type arguments for constructor/record-init expressions (`Some(1)`, `Box { value = 1 }`).
- [x] Ensure resulting expression type is `Named(type_id, inferred_args)` instead of always `Named(type_id, [])`.
- [x] Apply same logic for sum constructors, record constructors, and constructor patterns where needed.
- Exit criteria:
  - `type Option[T] = | None | Some(T)` accepts `Some(1)` as `Option[Int]`.
  - `type Box[T] { value: T }` accepts `Box { value = 1 }` as `Box[Int]`.
  - Add regression tests for both cases.

### 2. Type alias semantics must work in assignability
- [x] Decide v0.1 alias behavior: transparent aliasing vs nominal aliasing.
- [x] Implement chosen behavior consistently in type checking and branch unification.
- [x] Add tests for:
  - simple alias: `type Distance = Int`
  - generic alias: `type Id[T] = T`
- Exit criteria:
  - Alias examples type-check according to documented semantics.

### 3. Import/module behavior must be explicitly finalized for v0.1
- [x] Decide and document scope:
  - [x] `single-file compilation with extern/import stubs`
  - [x] `real multi-file module loading + checking`
- [x] Enforce chosen scope in diagnostics.
  - [x] If single-file: reject unresolved imported symbols with clear errors.
  - [x] If multi-file: add loader + module graph + per-file compilation order.
- Exit criteria:
  - No silent/implicit extern-path fallback for unresolved module members.
  - README and CLI behavior match implementation.

## Should-Do Before Release

### 4. Strengthen diagnostics quality
- [ ] Improve messages around imports/extern path fallback (`attempted to call a non-function value` should identify unresolved import/function).
- [ ] Add targeted notes for constructor/record payload mismatches.
- Exit criteria:
  - Key semantic failures identify the specific unresolved symbol or mismatch source.

### 5. Expand negative-path tests
- [ ] Add tests for:
  - generic ADT inference failures
  - alias mismatch failures
  - import module/item misuse
  - non-exhaustive match with generics
- Exit criteria:
  - New failures are covered by deterministic tests in `cargo test`.

### 6. Stabilize release definition in docs
- [ ] Add a short `v0.1 scope` section in README:
  - supported features
  - known exclusions
  - expected CLI behavior
- Exit criteria:
  - No ambiguity between architecture doc and actual compiler behavior.

## Already Done (Validated)

- [x] End-to-end pipeline is integrated: lexer -> parser -> resolve -> typecheck -> TIR -> Lua.
- [x] Core syntax/features parse and type-check in provided sample programs.
- [x] Lua emission works for records, methods, loops, `if`, `match`, constructors, and extern calls.
- [x] Test suite passes (`cargo test`).
- [x] Sample smoke checks pass:
  - `cargo run -- check samples/*.luna|*.cal`
  - `cargo run -- emit-lua samples/* -o out`

## Suggested Completion Gate

Call v0.1 complete only when all items in **Must-Do Before v0.1** are checked and the following succeeds:

1. `cargo test`
2. `cargo run -- check` on all sample programs
3. `cargo run -- emit-lua` on all sample programs
4. One dedicated fixture each for:
   - generic sum constructor inference
   - generic record constructor inference
   - alias typing behavior
   - import/module scope behavior
