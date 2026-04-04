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

The SDK is accessed through Lua globals (`playdate.graphics.sprite.new()`, etc.). Callisto's `extern module` declarations map directly to this — extern function paths join with `.` and emit verbatim as Lua calls. No glue needed.

**Create `src/playdate.cal` (shared bindings):**
```callisto
module playdate

extern module playdate.graphics do
  extern fn clear() -> ()
  extern fn setColor(color: Int) -> ()
end

extern module playdate.graphics.sprite do
  extern fn new() -> Int          -- returns sprite handle (opaque for now)
  extern fn update() -> ()
end

extern module playdate.timer do
  extern fn updateTimers() -> ()
end

extern module playdate do
  -- playdate.update is set by assigning a function; use a helper
end
```

These emit as `playdate.graphics.clear()`, `playdate.graphics.sprite.new()`, etc. — exactly the right Lua.

---

## Update Loop Pattern

Playdate games work by assigning to `playdate.update`. In Callisto today, the cleanest approach is a thin `main.lua` shim that bootstraps the compiled module:

```lua
-- Source/main.lua (hand-written shim, ~5 lines)
local game = import "game"   -- loads game.lua emitted by Callisto
function playdate.update()
  game.update()
end
```

Then `src/game.cal` exports a `pub fn update()` that Callisto compiles to `game.lua`. This keeps the Callisto code self-contained and testable.

**Alternative (single-module, no shim):** Put everything in `src/main.cal` compiled to `Source/main.lua` with `-o Source/main.lua`. The emitted Lua includes all functions. Then a one-line extern + call at the bottom sets up the loop. Simpler for small games.

---

## Multi-Module Loading (Current Gap)

When Callisto emits multiple files, it does NOT emit `require()`/`import` calls. Cross-module calls become extern path expressions (e.g., `player.move()`), which need `player` to be a Lua global.

**Workaround today:** The `main.lua` shim loads each module and assigns it to a global:
```lua
local player = import "player"    -- sets _ENV.player implicitly under Playdate's import
local level  = import "level"
```

Playdate's `import` (not standard Lua `require`) executes the file in the global scope if it returns nothing, or assigns the return value. Since each Callisto module returns `M`, you need explicit assignment.

**v0.2 fix:** When `callisto.toml` is wired in, codegen can emit the correct `import` calls (or a boot loader) automatically.

---

## Iteration Loop (Day-to-Day)

```sh
# In one terminal: watch + auto-recompile
fswatch -o src/ | xargs -n1 -I{} make build

# In Makefile:
build:
    callisto build src/main.cal -o Source/
    pdc Source/ MyGame.pdx

run: build
    open MyGame.pdx
```

The Playdate Simulator has a "Reload Game" hotkey (`⌘R`) — combine with fswatch for a near-instant feedback loop without leaving the simulator.

---

## What to Build Next (Priority Order)

1. **`playdate.cal` bindings module** — Write extern declarations for the core SDK surface (`graphics`, `sprite`, `input`, `sound`, `timer`). This is the highest-leverage thing: it unlocks type-safe SDK calls immediately.

2. **A small real game** (e.g., Pong or a bouncing ball) — Acts as a living integration test and drives what bindings are missing.

3. **v0.2 M1 CLI wiring** — Wire `--module-root` and `callisto.toml` into the pipeline so you can eliminate the manual shim and have proper multi-module output.

4. **Codegen: emit Playdate `import` calls** — After M1, extend codegen to emit `import "module_name"` in `main.lua` automatically, removing the hand-written shim entirely.

---

## Verification

- `make run` opens a game in the Playdate Simulator without errors
- Type errors in `.cal` are caught before `pdc` ever runs
- SDK calls like `playdate.graphics.clear()` appear verbatim in emitted Lua
- `cargo test` continues to pass after any compiler changes
