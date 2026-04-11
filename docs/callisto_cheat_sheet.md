# Callisto Cheat Sheet

Quick reference for day-to-day Callisto work.

## CLI

```sh
# Parse and print AST
callisto parse path/to/main.cal

# Scaffold a Playdate project
callisto init --template playdate my-game

# Typecheck only
callisto check path/to/main.cal [--config path/to/callisto.toml] [--module-root path]...

# Emit Lua
callisto emit-lua path/to/main.cal [-o out.lua|out_dir] [--config path/to/callisto.toml] [--module-root path]... [--playdate-bootstrap]

# Alias of emit-lua
callisto build path/to/main.cal [-o out.lua|out_dir] [--config path/to/callisto.toml] [--module-root path]... [--playdate-bootstrap]

# One-command Playdate build (emit + pdc + optional simulator launch)
callisto build-playdate path/to/game.cal [--source-dir Source] [--pdx Game.pdx] [--pdc pdc] [--run] [--config path/to/callisto.toml] [--module-root path]...
```

Precedence:
- CLI flags override config values.
- `-o` overrides `out_dir`.
- `--module-root` entries override `module_roots`.
- `--playdate-bootstrap` writes a Playdate `main.lua` shim in output directories.
- Bootstrap requires entry exports: `pub fn init() -> S`, `pub fn update(state: S) -> S`, `pub fn render(state: S) -> Unit`.
- `build-playdate --source-dir` overrides output directory selection.
- `build-playdate --pdx` overrides default bundle output path.

## `callisto.toml`

```toml
module_roots = ["../shared", "/absolute/vendor"]
out_dir = "build"
package = "demo.app"
```

## Module and imports

```cal
module game.main

import math.vec2
import math.vec2.{length, normalize}
```

Module path `foo.bar` resolves like:
- `foo/bar.cal` or `foo/bar.luna`
- `foo/bar/mod.cal` or `foo/bar/mod.luna`

## Types

```cal
type Vec2 { x: Float, y: Float }
type Option[T] = | None | Some(T)
type Shape = | Circle { radius: Float } | Rect { w: Float, h: Float }
```

Built-ins:
- `Int`, `Float`, `Bool`, `String`, `Unit`

## Bindings and functions

```cal
let pi: Float = 3.14
var count: Int = 0

fn add(a: Int, b: Int) -> Int do
  a + b
end

let double = fn (x: Int) -> Int => x * 2
```

## Control flow

```cal
if score > 50 then
  "pass"
else
  "retry"
end

while running do
  tick()
end

for i in 0..10 do
  print(i)
end

match value do
  case Some(v) => v
  case None    => 0
end
```

## Records and methods

```cal
type Player { hp: Int, name: String }

impl Player do
  fn damaged(self: Player, amount: Int) -> Player do
    self with { hp = self.hp - amount }
  end
end
```

## Extern interop

```cal
extern module playdate.graphics do
  extern fn clear() -> Unit
  extern fn drawText(text: String, x: Int, y: Int) -> Unit
end
```

## Diagnostics

Diagnostic format:

```text
path/to/file.cal:line:col: error[CAL-XXX-000]: message
```

Stable code prefixes:
- `CAL-CFG-*` config loading/validation
- `CAL-RES-*` module/name resolution
- `CAL-TYP-*` type checking
