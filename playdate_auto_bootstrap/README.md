# Auto Bootstrap Demo (Playdate + Callisto)

Small Playdate sample that uses Callisto's `--playdate-bootstrap` output mode.

The sample includes an explicit-transition multi-scene HUD:
- press `A` (just-pressed) to move to the next scene
- press `B` (just-pressed) to move to the previous scene
- scene state persists until another transition event occurs

It also tracks persisted session metrics:
- total ticks (`update` frames)
- scene transition count
- accumulated `Pilot`/`Telemetry` frame counts
- last transition direction

Crank direction and crank-side labels are still rendered every frame.

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
