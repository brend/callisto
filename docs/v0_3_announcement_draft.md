# Callisto v0.3.0 Announcement (Draft)

Callisto `v0.3.0` makes Playdate development first-party, repeatable, and faster to start.

## Highlights

- First-party Playdate project scaffolding:
  - `callisto init --template playdate <dir>`
  - generates `callisto.toml`, `src/game.cal`, `Source/`, `README.md`, and `Makefile`
- First-party Playdate build UX:
  - `callisto build-playdate <entry.cal> [--source-dir] [--pdx] [--pdc] [--run]`
  - emits Lua with bootstrap, runs `pdc`, and can launch simulator output
- Playdate bootstrap integration:
  - `--playdate-bootstrap` for `emit-lua` / `build`
  - validation for entry shape and safe shim generation
- Expanded shared Playdate bindings:
  - `playdate`, `playdate.graphics`, `playdate.graphics.sprite`, `playdate.timer`
  - `playdate.input`, `playdate.audio`, `playdate.system`

## Sample Coverage

- `playdate_bouncing_ball` now uses shared bindings and exercises language features (sum types, `match`, `impl`, generics, record updates).
- `playdate_auto_bootstrap` demonstrates auto-shim flow and a richer multi-scene HUD with crank telemetry labels.

## Reliability

- Full test suite passing on the release candidate (`cargo test`).
- Playdate sample Lua-build smoke checks passing:
  - `make -C playdate_bouncing_ball build-lua`
  - `make -C playdate_auto_bootstrap build-lua`
- Release binary build and smoke checks passing (`cargo build --release` and `target/release/callisto` checks for `check`, `emit-lua`, `init`, and `build-playdate`).

## Upgrade Notes

- Existing `v0.2` projects remain compatible.
- Playdate users can keep manual `build + pdc` flow, but `build-playdate` is now the recommended default.
- For shared SDK bindings, ensure `module_roots` includes `../playdate_bindings/src` (or pass `--module-root` flags).
