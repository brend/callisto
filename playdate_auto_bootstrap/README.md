# Auto Bootstrap Demo (Playdate + Callisto)

Small Playdate sample that uses Callisto's `--playdate-bootstrap` output mode.

The sample now includes an input-driven multi-scene HUD:
- default `Splash` view (no A/B held)
- `Pilot` view while holding `A`
- `Telemetry` view while holding `B`

It also displays crank direction and crank-side labels every frame.

## Layout

- `src/game.cal`: entry module with `pub fn init() -> State`, `pub fn update(state: State) -> State`, `pub fn render(state: State) -> Unit`
- `../playdate_bindings/src/playdate/*.cal`: shared Playdate extern bindings
- `Source/main.lua`: generated Playdate bootstrap shim
- `Source/game.lua`: generated gameplay module
- `AutoBootstrap.pdx`: built by `pdc`

## Build

From this directory:

```sh
make build-lua
```

Or use the first-party one-command path:

```sh
../target/debug/callisto build-playdate src/game.cal --config callisto.toml --pdx AutoBootstrap.pdx
```

To build `.pdx` (requires Playdate SDK `pdc`):

```sh
make build
```

To open in simulator:

```sh
make run
```
