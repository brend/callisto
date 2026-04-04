# Callisto → Playdate: End-to-End Development Workflow

## Context

Callisto compiles `.cal` source to Lua. The Panic Playdate runs Lua 5.4 via its SDK, where games are structured as a folder with `main.lua` (plus assets), compiled by `pdc` into a `.pdx` bundle. The goal is to define the full loop: write Callisto → emit Lua → run on simulator/device.

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

The SDK is accessed through Lua globals (`playdate.graphics.sprite.new()`, etc.). The cleanest shared-binding pattern is module-path files with `pub extern fn` declarations:

**Create `src/playdate/graphics.cal`:**
```callisto
module playdate.graphics

pub extern fn clear() -> Unit
pub extern fn setColor(color: Int) -> Unit
pub extern fn drawText(text: String, x: Int, y: Int) -> Unit
```

Then use it from game code with:
```callisto
import playdate.graphics
```

Calls still emit as `playdate.graphics.clear()`, `playdate.graphics.sprite.new()`, etc.

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

## What to Build Next (Priority Order)

1. **Expand shared SDK bindings** — Grow `playdate/*` module coverage (`graphics.sprite`, `input`, `sound`, `timer`) with `pub extern fn` declarations.

2. **A small real game** (e.g., Pong or a bouncing ball) — Acts as a living integration test and drives what bindings are missing.

3. **Bootstrap customization** — Extend `--playdate-bootstrap` with configurable update target and optional multi-import preloads.

4. **Playdate-oriented build glue** — Add a first-party command or template that runs `callisto build` + `pdc` with stable output paths.

---

## Verification

- `make run` opens a game in the Playdate Simulator without errors
- Type errors in `.cal` are caught before `pdc` ever runs
- SDK calls like `playdate.graphics.clear()` appear verbatim in emitted Lua
- `cargo test` continues to pass after any compiler changes
