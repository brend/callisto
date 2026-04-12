# Callisto v0.6 M2 Match Analysis Checklist

Execution checklist for `M2: Match Analysis Completeness + Parser QoL`.

Use this as the implementation board for exhaustiveness/reachability diagnostics and parser ergonomics.

## Scope Freeze

- [ ] Keep `M2` focused on match-analysis quality plus targeted parser QoL additions.
- [ ] Preserve current valid match syntax and execution semantics.
- [ ] Prioritize diagnostic precision over introducing speculative inference behavior.

## M2 Decisions (Implement As Written)

1. Exhaustiveness behavior
- Strengthen exhaustiveness checks for finite domains already represented in the type system.
- Keep non-finite-domain behavior pragmatic and explicitly documented.

2. Reachability behavior
- Detect and report duplicate constructor arms.
- Detect and report unreachable arms after catch-all patterns.

3. Diagnostic quality
- Use stable error-code paths for new match-analysis failures where appropriate.
- Include actionable notes identifying previously-covered arms/patterns.

4. Parser QoL behavior
- Allow trailing commas in multiline lists for parameters/arguments/payloads/match arms.
- Support record field punning in initializers (for example `Point { x }`).

## Implementation Tasks

### A) Typechecker Match Analysis

- [ ] Extend match-coverage tracking in `src/typecheck.rs` for duplicate and dead-arm detection.
- [ ] Ensure catch-all arm handling is deterministic and diagnostics are non-cascading.
- [ ] Validate behavior for constructor, wildcard, and Bool-pattern combinations.

### B) Regression Coverage

- [ ] Add positive/negative tests in `src/main.rs` for duplicate-constructor arms.
- [ ] Add tests for unreachable arms after wildcard/bind catch-alls.
- [ ] Add parser/typecheck acceptance tests for trailing commas and field punning.
- [ ] Keep existing non-exhaustive-match diagnostics coverage green.

### C) Docs

- [ ] Update `README.md` diagnostics/language reference with match-analysis behavior.
- [ ] Record `M2` status in `docs/v0_6_draft_plan.md`.

## Definition of Done (M2)

- [ ] `cargo test` passes with expanded match-analysis coverage.
- [ ] Duplicate/unreachable arm diagnostics are clear and deterministic.
- [ ] Trailing-comma and field-punning syntax paths parse/check cleanly.
- [ ] Existing valid match programs remain unaffected.
