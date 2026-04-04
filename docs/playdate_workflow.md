# Callisto → Playdate: End-to-End Development Workflow

## Context

Callisto compiles `.cal` source to Lua. The Panic Playdate runs Lua 5.4 via its SDK, where games are structured as a folder with `main.lua` (plus assets), compiled by `pdc` into a `.pdx` bundle. The goal is to define the full loop: write Callisto → emit Lua → run on simulator/device.

Current planning docs:
- [`docs/v0_3_draft_plan.md`](docs/v0_3_draft_plan.md)
- [`docs/v0_3_m1_playdate_execution_checklist.md`](docs/v0_3_m1_playdate_execution_checklist.md)
- [`docs/v0_3_m2_bindings_execution_checklist.md`](docs/v0_3_m2_bindings_execution_checklist.md)

---

## Build Pipeline

```
*.cal  →  callisto build  →  *.lua  →  pdc  →  MyGame.pdx  →  Simulator / Device
```

Concretely:
```sh
callisto build src/main.cal -o Source/
pdc Source/ MyGame.pdx
open MyGame.pdx          # opens in Playdate Simulator
```

A `Makefile` is the right glue here — one `make run` target that chains all three steps.

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
- `playdate.graphics.sprite`
- `playdate.timer`

Then import the modules you need:
```callisto
import playdate
import playdate.audio
import playdate.graphics
import playdate.input
```

Calls emit as `playdate.graphics.clear()`, `playdate.getCrankChange()`, etc.

---

## Update Loop Pattern

Playdate games work by assigning to `playdate.update`. You now have two options:

1. **Auto shim:** `callisto build src/game.cal -o Source/ --playdate-bootstrap`
2. **Manual shim:** keep a hand-written `Source/main.lua` file

Auto shim writes `Source/main.lua` that imports the compiled entry module and calls `game.update()` every frame. It requires the entry module to export:

```callisto
pub fn update() -> Unit do
  ()
end
```

Manual shim (same as before):

```lua
local game = import "game"   -- loads game.lua emitted by Callisto
function playdate.update()
  game.update()
end
```

**Alternative (single-module, no shim):** Put everything in `src/main.cal` compiled to `Source/main.lua` with `-o Source/main.lua`. The emitted Lua includes all functions. Then a one-line extern + call at the bottom sets up the loop. Simpler for small games.

---

## Multi-Module Loading

When Callisto emits multiple files, cross-module calls still rely on Playdate `import` at runtime.

`--playdate-bootstrap` closes the most common gap by generating a `main.lua` shim automatically. For custom startup (multiple module preloads, init order, runtime state wiring), keep using a manual `main.lua` shim.

Example manual preload shim:
```lua
local player = import "player"    -- sets _ENV.player implicitly under Playdate's import
local level  = import "level"
playdate.audio = import "playdate/audio"
playdate.input = import "playdate/input"
```

Playdate's `import` (not standard Lua `require`) executes the file in the global scope if it returns nothing, or assigns the return value. Since each Callisto module returns `M`, you need explicit assignment.

---

## Iteration Loop (Day-to-Day)

```sh
# In one terminal: watch + auto-recompile
fswatch -o src/ | xargs -n1 -I{} make build

# In Makefile:
build:
    callisto build src/main.cal -o Source/ --playdate-bootstrap
    pdc Source/ MyGame.pdx

run: build
    open MyGame.pdx
```

The Playdate Simulator has a "Reload Game" hotkey (`⌘R`) — combine with fswatch for a near-instant feedback loop without leaving the simulator.

---

## Reference Projects

- `playdate_bouncing_ball/`: manual `Source/main.lua` shim pattern (state owned by Lua).
  Uses records, `impl` methods, sum types, and `match` in gameplay logic.
- `playdate_auto_bootstrap/`: auto-shim pattern using `--playdate-bootstrap`.

## What to Build Next (Priority Order)

1. **Expand SDK coverage** — Add shared bindings for the next concrete APIs needed by samples.
2. **Richer sample game** — Build a larger game loop that drives binding gaps and ergonomics.
3. **Bootstrap customization** — Extend `--playdate-bootstrap` with configurable update target and optional preload imports.
4. **Playdate build UX** — Add first-party template/command ergonomics for `callisto build` + `pdc`.

---

## Verification

- `make run` opens a game in the Playdate Simulator without errors
- Type errors in `.cal` are caught before `pdc` ever runs
- SDK calls like `playdate.graphics.clear()` appear verbatim in emitted Lua
- `cargo test` continues to pass after any compiler changes
