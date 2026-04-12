# Callisto v0.4 M2 Parser Ergonomics Checklist

Execution checklist for `M2: Parser Ergonomics`.

Use this as the implementation board for sum-type declaration formatting improvements.

## Scope Freeze

- [x] Keep parser change targeted to sum declaration formatting after `=`.
- [x] Preserve compatibility for all currently valid type declarations.
- [x] Avoid broad grammar expansion in this milestone.

## M2 Decisions (Implement As Written)

1. Supported formatting
- Allow:
  - `type Option[T] = | None | Some(T)` (existing)
  - `type Option[T] =` newline `| None` newline `| Some(T)` (new)
- Continue requiring `|` markers for each variant.

2. Error behavior
- Empty sum declaration after `=` remains an error with actionable diagnostics.
- Alias parsing behavior remains unchanged when no sum-variant `|` sequence follows.

3. Regression floor
- Add parser regression tests for single-line and multi-line forms.
- Add at least one negative parser test for malformed multi-line sum declarations.

## Implementation Tasks

### A) Parser Logic

- [x] Update sum-body parsing flow in `src/parser.rs` so sum variants may start on following lines after `=`.
- [x] Ensure newline handling does not break alias or record type declarations.
- [x] Preserve AST spans and diagnostic locations for affected paths.

### B) Tests

- [x] Add regression tests covering new accepted multi-line sum formatting.
- [x] Add negative tests for malformed variants or empty variant lists.
- [x] Verify existing parser/typechecker tests remain green.

### C) Docs

- [x] Update syntax examples in `README.md` and/or `docs/callisto_cheat_sheet.md` with the new formatting option.
- [x] Record `M2` status in `docs/v0_4_draft_plan.md`.

## Definition of Done (M2)

- [x] `cargo test` passes with new parser regression coverage.
- [x] Multi-line sum declarations parse successfully.
- [x] Existing single-line declarations remain unchanged.
- [x] Diagnostics for malformed declarations remain clear and stable.
