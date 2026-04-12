# Callisto v0.4 M3 Type Inference Checklist

Execution checklist for `M3: Type Inference Quality`.

Use this as the implementation board for nullary generic constructor inference in nested contexts.

## Scope Freeze

- [x] Restrict inference work to nullary generic constructors with available context.
- [x] Preserve existing explicit diagnostics for unconstrained constructor usage.
- [x] Avoid full inference engine redesign in this milestone.

## M3 Decisions (Implement As Written)

1. Targeted inference behavior
- Improve inference where constructor type can be derived from expected context in nested expressions.
- Prioritize record/variant payload and initializer field paths where expected field types exist.

2. Safety behavior
- Keep error `cannot infer generic type arguments for constructor '<Name>' without context` for unconstrained cases.
- Do not change function signature/type-annotation requirements outside this focused path.

3. Regression floor
- Add positive tests for nested inference cases that previously needed temporary annotations.
- Keep/extend negative tests that verify unconstrained inference still errors.

## Implementation Tasks

### A) Typechecker Context Propagation

- [x] Audit expression-checking paths in `src/typecheck.rs` for record/variant field evaluation without expected type hints.
- [x] Pass expected field types into `check_expr_with_expected` where contextual types are known.
- [x] Ensure constructor payload validation still runs after inference changes.

### B) Tests

- [x] Add regression tests in `src/main.rs` for nested nullary constructor inference in record initializers.
- [x] Add regression tests for nested nullary constructor inference in constructor payload contexts.
- [x] Preserve negative-path tests for unconstrained `None`/nullary generic constructor usage.

### C) Diagnostics + Docs

- [x] Verify diagnostic messaging remains actionable and consistent for failed inference.
- [x] Record `M3` status in `docs/v0_4_draft_plan.md`.

## Definition of Done (M3)

- [x] `cargo test` passes with new inference regression coverage.
- [x] Previously annotation-heavy nested constructor cases compile without added temporary annotations.
- [x] Unconstrained nullary generic constructor usage still emits clear inference diagnostics.
