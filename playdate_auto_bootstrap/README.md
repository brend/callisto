# Auto Bootstrap Demo (Playdate + Callisto)

Small Playdate sample that uses Callisto's `--playdate-bootstrap` output mode.

## Layout

- `src/game.cal`: entry module with `pub fn update() -> Unit`
- `../playdate_bindings/src/playdate/*.cal`: shared Playdate extern bindings
- `Source/main.lua`: generated Playdate bootstrap shim
- `Source/game.lua`: generated gameplay module
- `AutoBootstrap.pdx`: built by `pdc`

## Build

From this directory:

```sh
make build-lua
```

To build `.pdx` (requires Playdate SDK `pdc`):

```sh
make build
```

To open in simulator:

```sh
make run
```
