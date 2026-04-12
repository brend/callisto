# Callisto → Playdate: End-to-End Development Workflow

## Context

Callisto compiles `.cal` source to Lua. The Panic Playdate runs Lua 5.4 via its SDK, where games are structured as a folder with `main.lua` (plus assets), compiled by `pdc` into a `.pdx` bundle. The goal is to define the full loop: write Callisto → emit Lua → run on simulator/device.

Current planning docs:
- [`docs/v0_3_draft_plan.md`](docs/v0_3_draft_plan.md)
- [`docs/v0_3_m1_playdate_execution_checklist.md`](docs/v0_3_m1_playdate_execution_checklist.md)
- [`docs/v0_3_m2_bindings_execution_checklist.md`](docs/v0_3_m2_bindings_execution_checklist.md)
- [`docs/v0_3_m3_playdate_product_execution_checklist.md`](docs/v0_3_m3_playdate_product_execution_checklist.md)

---

## Build Pipeline

```
*.cal  →  callisto build-playdate  →  *.lua + pdc  →  MyGame.pdx  →  Simulator / Device
```

Concretely:
```sh
callisto init --template playdate my-game --workflow auto-bootstrap --starter-assets
cd my-game
callisto build-playdate src/game.cal --config callisto.toml --pdx Game.pdx --run
```

`callisto init --template playdate` template knobs:
- `--workflow auto-bootstrap` (default): Makefile uses generated bootstrap `main.lua`.
- `--workflow manual-shim`: template includes `Source/main.lua` and manual-shim build flow.
- `--starter-assets`: creates starter folders under `Source/` (`images`, `sounds`, `fonts`).

`build-playdate` is the first-party happy-path command. It emits Lua with bootstrap, runs `pdc`, and optionally opens the simulator.
You can still use `callisto build` + `pdc` manually when you need more control.

**Folder layout:**
```
my-game/
  callisto.toml          # (v0.2) module roots, out_dir
  src/
    main.cal             # entry module
    player.cal
    level.cal
  Source/                # Callisto emits Lua here; also holds assets
    main.lua
    player.lua
    level.lua
    images/
    sounds/
  MyGame.pdx             # pdc output
```

---

## Playdate SDK Bindings (Works Today)

The SDK is accessed through Lua globals (`playdate.graphics.sprite.new()`, etc.).  
Use the shared bindings package at `playdate_bindings/src` and add it to project module roots:

```toml
module_roots = ["src", "../playdate_bindings/src"]
```

Current shared modules:
- `playdate`
- `playdate.audio`
- `playdate.graphics`
- `playdate.input`
- `playdate.system`
- `playdate.graphics.sprite`
- `playdate.timer`

Then import the modules you need:
```callisto
import playdate
import playdate.audio
import playdate.graphics
import playdate.input
import playdate.system
```

Calls emit as `playdate.graphics.clear()`, `playdate.getCrankChange()`, etc.

---

## Update Loop Pattern

Playdate games work by assigning to `playdate.update`. You now have two options:

1. **Auto shim:** `callisto build src/game.cal -o Source/ --playdate-bootstrap`
2. **Manual shim:** keep a hand-written `Source/main.lua` file

Auto shim customization flags:
- `--playdate-bootstrap-target <lua.path>` to assign a custom update function (default `playdate.update`).
- `--playdate-bootstrap-preload <module/path>` to preload modules before entry import.
- `--playdate-bootstrap-preload <lua.path=module/path>` to preload and assign returned modules (for example `playdate.input=playdate/input`).

Auto shim writes `Source/main.lua` that imports the compiled entry module and runs an explicit state loop each frame. It requires the entry module to export:

```callisto
pub fn init() -> State do
  State { ... }
end

pub fn update(state: State) -> State do
  ...
end

pub fn render(state: State) -> Unit do
  ()
end
```

Manual shim (same as before):

```lua
local game = import "game"   -- loads game.lua emitted by Callisto
local state = game.init()
function playdate.update()
  state = game.update(state)
  game.render(state)
end
```

**Alternative (single-module, no shim):** Put everything in `src/main.cal` compiled to `Source/main.lua` with `-o Source/main.lua`. The emitted Lua includes all functions. Then a one-line extern + call at the bottom sets up the loop. Simpler for small games.

---

## Multi-Module Loading

When Callisto emits multiple files, cross-module calls still rely on Playdate `import` at runtime.

`--playdate-bootstrap` closes the most common gap by generating a `main.lua` shim automatically, and now supports optional preload imports plus custom update assignment targets.
Keep using a manual `main.lua` shim when you need startup logic beyond import/assignment (for example custom Lua control flow before/after each frame).

Example manual preload shim:
```lua
local player = import "player"    -- sets _ENV.player implicitly under Playdate's import
local level  = import "level"
playdate.audio = import "playdate/audio"
playdate.input = import "playdate/input"
playdate.system = import "playdate/system"
```

Playdate's `import` (not standard Lua `require`) executes the file in the global scope if it returns nothing, or assigns the return value. Since each Callisto module returns `M`, you need explicit assignment.

---

## Iteration Loop (Day-to-Day)

```sh
# In one terminal: watch + auto-recompile
fswatch -o src/ | xargs -n1 -I{} make build

# In Makefile:
build:
    callisto build-playdate src/game.cal --config callisto.toml --pdx Game.pdx

run: build
    callisto build-playdate src/game.cal --config callisto.toml --pdx Game.pdx --run
```

The Playdate Simulator has a "Reload Game" hotkey (`⌘R`) — combine with fswatch for a near-instant feedback loop without leaving the simulator.

---

## Reference Projects

- `playdate_bouncing_ball/`: manual `Source/main.lua` shim pattern (state owned by Lua).
  Uses records, `impl` methods, sum types, generics, and `match` in gameplay logic.
- `playdate_auto_bootstrap/`: auto-shim pattern using `--playdate-bootstrap`.
  Includes explicit-transition scene navigation (`A` next, `B` previous), persisted mission-loop counters/resources (`score`, `combo`, `laps`, `energy`, `heat`), crank telemetry labels, `playdate.timer.updateTimers()` usage, and graphics overlays driven by shared `playdate.graphics.drawLine/drawRect/fillRect` bindings.

## What to Build Next (Priority Order)

1. **Expand SDK coverage** — Add shared bindings for the next concrete APIs needed by samples.
2. **Richer sample game** — Build a larger game loop that drives binding gaps and ergonomics.
3. **Bootstrap customization** — Extend `--playdate-bootstrap` with configurable update target and optional preload imports.
4. **Template hardening** — Add optional starter assets/workflow variants for first-party Playdate templates.

---

## Verification

- `make run` opens a game in the Playdate Simulator without errors
- Type errors in `.cal` are caught before `pdc` ever runs
- SDK calls like `playdate.graphics.clear()` appear verbatim in emitted Lua
- `cargo test` continues to pass after any compiler changes
