# Callisto

Callisto is a statically-typed programming language that compiles to Lua. It brings type safety, algebraic data types, and pattern matching to the Lua ecosystem while emitting clean, readable Lua code.

## Features

- **Static type system** — primitives (`Int`, `Float`, `Bool`, `String`), record types, sum types, and generics
- **Pattern matching** — exhaustive `match`/`case` on sum types, literals, and constructors
- **Algebraic data types** — record types and sum types with positional or named payloads
- **Immutable/mutable bindings** — `let` for immutable, `var` for mutable
- **Method syntax** — `impl` blocks for attaching methods to types
- **Lambda expressions** — first-class functions with explicit types
- **Record update syntax** — non-destructive field updates with `with`
- **Extern interop** — typed bindings to existing Lua APIs via `extern`
- **Module system** — `module` declarations and `import` statements
- **Readable output** — emits idiomatic, human-readable Lua

## Installation

Requires [Rust](https://rustup.rs/) (edition 2024).

```sh
git clone <repo>
cd callisto
cargo build --release
# binary: target/release/callisto
```

## Usage

```
callisto parse    <file.cal>                  # Parse and dump the AST
callisto check    <file.cal> [--config path] [--module-root path]...
callisto emit-lua <file.cal> [-o out.lua|dir] [--config path] [--module-root path]...
callisto build    <file.cal> [-o out.lua|dir] [--config path] [--module-root path]...
```

Default output precedence:
- `-o` flag overrides everything.
- If `-o` is not provided and config has `out_dir`, config `out_dir` is used.
- Otherwise output defaults to `out/`.

## Configuration (v0.2 M1)

`callisto.toml` is supported for project-level configuration.

Discovery:
- Use explicit `--config <path>` if provided.
- Otherwise, check for `callisto.toml` in the entry file directory.
- If no config exists, defaults are used.

Example:

```toml
module_roots = ["../shared", "/absolute/path/to/vendor"]
out_dir = "build"
package = "demo.app"
```

Resolution precedence:
- Module roots: CLI `--module-root` entries (in order) override config `module_roots`; if neither is provided, the entry file directory root is used.
- Output directory: `-o` overrides config `out_dir`; config `out_dir` overrides default `out`.

## v0.1 Scope (Baseline)

Supported in `v0.1`:
- Single-entry compilation with recursive module loading from the entry directory root (`foo.bar` -> `foo/bar.luna|.cal` or `foo/bar/mod.luna|.cal`).
- End-to-end compiler pipeline (`parse`, `check`, `emit-lua`/`build`) with diagnostics surfaced in CLI output.
- Type checking for records, sums, pattern matching, aliases (transparent), and generic constructor inference for common ADT use.
- Imports resolved via loaded modules or explicit `extern module` declarations.

Known exclusions in `v0.1`:
- No package/dependency manager. Configurable import roots were introduced later in `v0.2 M1`.
- No implicit import/extern fallback: unresolved imported members are hard type errors.
- No nominal alias/newtype behavior (aliases are transparent in assignability/unification).

Expected CLI behavior:
- `callisto parse <file>` prints AST or syntax diagnostics.
- `callisto check <file>` runs full semantic checking and emits diagnostics without Lua output.
- `callisto emit-lua <file> [-o out.lua|dir]` (or `build`) writes Lua for the entry module and loaded imports when output is a directory.

## Examples

### Records and functions

```
module geometry

type Vec2 { x: Float, y: Float }

fn length_sq(v: Vec2) -> Float do
  v.x * v.x + v.y * v.y
end

pub fn translate(v: Vec2, dx: Float, dy: Float) -> Vec2 do
  v with { x = v.x + dx, y = v.y + dy }
end
```

Transpiles to:

```lua
local M = {}

local function length_sq(v)
    return v.x * v.x + v.y * v.y
end

local function translate(v, dx, dy)
    return (function(__base)
        local __tmp = {}
        for k, val in pairs(__base) do __tmp[k] = val end
        __tmp.x = __base.x + dx
        __tmp.y = __base.y + dy
        return __tmp
    end)(v)
end
M.translate = translate

return M
```

### Sum types and pattern matching

```
module option

type Option[T] = | None | Some(T)

impl Option do
  fn unwrap_or(self: Option[Int], fallback: Int) -> Int do
    match self do
      case Some(v) => v
      case None    => fallback
    end
  end
end

pub fn safe_div(a: Int, b: Int) -> Option[Int] do
  if b == 0 then
    None
  else
    Some(a / b)
  end
end
```

### Extern interop

Bind to an existing Lua API without writing boilerplate:

```
extern module playdate.graphics {
  fn clear() -> Unit
  fn drawText(text: String, x: Int, y: Int) -> Unit
}

pub fn render(msg: String) -> Unit do
  playdate.graphics.clear()
  playdate.graphics.drawText(msg, 10, 10)
end
```

## Language reference

### Types

| Syntax | Description |
|---|---|
| `Int`, `Float`, `Bool`, `String` | Primitive types |
| `type Point { x: Int, y: Int }` | Record type |
| `type Shape = \| Circle(Float) \| Rect { w: Float, h: Float }` | Sum type |
| `type Option[T] = \| None \| Some(T)` | Generic sum type |

### Bindings

```
let x = 42           -- immutable
var count: Int = 0   -- mutable, explicit annotation optional
count = count + 1
```

### Functions

```
fn add(a: Int, b: Int) -> Int do
  a + b
end

let double = fn (x: Int) -> Int => x * 2
```

### Control flow

```
-- if expression
if score > 100 then
  "great"
elseif score > 50 then
  "ok"
else
  "try again"
end

-- while loop
while alive do
  tick()
end

-- range for loop
for i in 0..10 do
  process(i)
end

-- match expression
match shape do
  case Circle(r)         => 3.14 * r * r
  case Rect { w, h }     => w * h
end
```

### Modules

```
module my.package

import other.module
import other.module { foo, bar }
```

## Development

```sh
cargo fmt        # format
cargo test       # run tests
```

## Architecture

The compiler pipeline is:

```
source → lexer → parser → AST → name resolution → type checking → TIR → Lua codegen
```

See [`docs/luna_compiler_architecture_v0_1.md`](docs/luna_compiler_architecture_v0_1.md) for the full design.

Planning for the next release is tracked in [`docs/v0_2_draft_plan.md`](docs/v0_2_draft_plan.md).
