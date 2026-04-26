# Callisto v0.9 M1 Frozen Language Surface + Conformance Checklist

## Conformance Matrix

- [x] Add `docs/v1_0_language_conformance_matrix.md`.
- [x] Include v0.7 prelude additions: `Option[T]`, `Some`, `None`, `List[T]`, `length`, and `map`.
- [x] Include v0.8 brace-delimited block syntax and migration diagnostics.
- [x] Include v0.9 list indexing plus `append`, `filter`, and `fold`.
- [x] Map frozen features to parser, resolver/typechecker, codegen, and diagnostic coverage where applicable.

## Gap Classification

- [x] Identify unresolved conformance gaps.
- [x] Classify remaining non-goals as post-1.0 exclusions rather than v0.9 blockers.
- [x] Confirm no known conformance blocker remains for the frozen `v1.0.0` surface.

## Coverage

- [x] Collection helper and list-indexing regressions exist.
- [x] Brace-only syntax parser and migration diagnostic regressions exist.
- [x] Module/import, config, emitted Lua, Playdate bootstrap, and binding regressions exist.
