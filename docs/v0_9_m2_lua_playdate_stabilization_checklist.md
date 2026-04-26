# Callisto v0.9 M2 Lua + Playdate Stabilization Checklist

## Lua Output Audit

- [x] List literals emit Lua array-style tables.
- [x] List indexing emits direct Lua table indexing.
- [x] `length` emits Lua length usage.
- [x] `map`, `append`, `filter`, and `fold` lower to readable local-loop Lua without external dependencies.
- [x] Core ADT, record, newtype, lambda, extern, import, and record-update lowering remain covered by existing regressions.
- [x] No v0.9 prelude helper requires non-Playdate Lua APIs.

## Playdate Workflow

- [x] Shared Playdate bindings remain in `playdate_bindings/src`.
- [x] Maintained sample projects remain aligned with `callisto.toml` module-root workflows.
- [x] Generated Playdate template still uses `callisto build ... --playdate-bootstrap` as the recommended auto-bootstrap path.
- [x] `build-playdate` remains documented as the one-step Playdate build path.

## Validation

- [x] Compiler regression tests cover emitted Lua for the expanded list helpers.
- [x] CLI smoke commands for sample and Playdate projects are part of the release gate.
- [x] Playdate bootstrap output validation remains covered by tests.
