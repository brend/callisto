# Callisto v0.4.0 Announcement (Draft)

Callisto `v0.4.0` improves day-to-day language ergonomics and sample realism while preserving the existing compiler and Playdate workflows.

## Highlights

- Deeper Playdate sample flow in `playdate_auto_bootstrap`:
  - explicit scene transitions using `buttonJustPressed` (`A` next, `B` previous)
  - persisted session metrics in model state (`ticks`, transition count, per-scene frame totals)
- Parser ergonomics:
  - sum types now support multiline declaration formatting after `=`
  - existing single-line sum syntax remains supported
- Type inference quality:
  - improved contextual inference for nullary generic constructors (for example `None`) in nested contexts
  - covered in record initializers, constructor payloads, and record updates

## Sample and Docs

- `playdate_auto_bootstrap` README now documents explicit transitions and persisted state behavior.
- Playdate workflow docs reflect the richer sample model.
- `v0.4` planning/checklist docs now track `M0` through `M4` execution status.

## Reliability

- `cargo test` passing (`98` tests).
- Sample build smokes passing:
  - `make -C playdate_bouncing_ball build-lua`
  - `make -C playdate_auto_bootstrap build-lua`
- Debug and release binary smoke checks passing for `check` and `emit-lua --playdate-bootstrap`.

## Upgrade Notes

- No breaking CLI changes in `v0.4.0`.
- Existing single-line ADT/sum declarations continue to work.
- Existing diagnostics for unconstrained nullary generic constructors remain in place.
