# Callisto

Callisto is a statically-typed programming language that compiles to Lua. It brings type safety, algebraic data types, and pattern matching to the Lua ecosystem while emitting clean, readable Lua code.

## Features

- **Static type system** — primitives (`Int`, `Float`, `Bool`, `String`), record types, sum types, generics, and nominal `newtype` wrappers
- **Pattern matching** — exhaustive `match`/`case` on sum types, literals, and constructors
- **Algebraic data types** — record types and sum types with positional or named payloads
- **Immutable/mutable bindings** — `let` for immutable, `var` for mutable
- **Method syntax** — `impl` blocks for attaching methods to types
- **Lambda expressions** — first-class functions with explicit types
- **Standard prelude** — built-in `Option[T]`, `List[T]`, `length`, and `map`
- **Record update syntax** — non-destructive field updates with `with`
- **Parser ergonomics** — trailing commas in multiline lists and record field punning (`Point { x }`)
- **Extern interop** — typed bindings to existing Lua APIs via `extern`
- **Module system** — `module` declarations and `import` statements
- **Readable output** — emits idiomatic, human-readable Lua

## Installation

Requires [Rust](https://rustup.rs/) (edition 2024).

```sh
git clone <repo>
cd callisto
cargo install --path . --locked
# binary: ~/.cargo/bin/callisto
```

## Usage

```
callisto parse    <file.cal>                  # Parse and dump the AST
callisto init     --template playdate <dir> [--workflow auto-bootstrap|manual-shim] [--starter-assets]
callisto check    <file.cal> [--config path] [--module-root path]...
callisto emit-lua <file.cal> [-o out.lua|dir] [--config path] [--module-root path]... [--playdate-bootstrap] [--playdate-bootstrap-target lua.path] [--playdate-bootstrap-preload module/path|lua.path=module/path]...
callisto build    <file.cal> [-o out.lua|dir] [--config path] [--module-root path]... [--playdate-bootstrap] [--playdate-bootstrap-target lua.path] [--playdate-bootstrap-preload module/path|lua.path=module/path]...
callisto build-playdate <file.cal> [--source-dir dir] [--pdx bundle.pdx] [--pdc exe] [--run] [--config path] [--module-root path]... [--playdate-bootstrap-target lua.path] [--playdate-bootstrap-preload module/path|lua.path=module/path]...
```

Default output precedence:
- `-o` flag overrides everything.
- If `-o` is not provided and config has `out_dir`, config `out_dir` is used.
- Otherwise output defaults to `out/`.
- `--playdate-bootstrap` (directory output only) writes `main.lua` that imports the entry module, initializes state, then calls `update(state)` and `render(state)` every frame.
- `--playdate-bootstrap-target` changes the assigned update function path (default: `playdate.update`).
- `--playdate-bootstrap-preload` adds preload imports before entry import; use `lua.path=module/path` to assign returned modules.

Playdate-first workflow:
- `callisto init --template playdate <dir>` scaffolds a project with `callisto.toml`, `src/game.cal`, and a Makefile.
- `--workflow auto-bootstrap` (default) emits a template Makefile wired for `--playdate-bootstrap`.
- `--workflow manual-shim` adds `Source/main.lua` and a manual-shim Makefile flow.
- `--starter-assets` adds starter directories: `Source/images`, `Source/sounds`, and `Source/fonts`.
- `callisto build-playdate <entry.cal>` runs Lua emission with bootstrap + `pdc` in one command.

## Configuration (v0.2)

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

## Diagnostics (v0.2)

Compiler diagnostics include stable error codes for key config, resolve, and typecheck paths.

Example format:

```
path/to/file.cal:line:col: error[CAL-TYP-010]: cannot call imported module 'foo.bar' as a function
```

Use these codes to search issues/docs and to keep troubleshooting stable when wording evolves.

Match-analysis diagnostics include:
- `CAL-TYP-030`: non-exhaustive `match` (sum variants and `Bool` cases list missing coverage).
- `CAL-TYP-031`: duplicate constructor arm (with a note pointing to the earlier covered arm).
- `CAL-TYP-032`: unreachable arm after prior coverage is complete (catch-all, complete `Bool`, or complete constructor coverage).
- `CAL-TYP-023`: constructor-pattern payload shape errors (wrong payload form or constructor-pattern arity mismatch).
- `CAL-TYP-024`: constructor-pattern record-field diagnostics (unknown/duplicate/missing fields with fix-it guidance).

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
fn unwrap_or(value: Option[Int], fallback: Int) -> Int do
  match value do
    case Some(v) => v
    case None    => fallback
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

`Option[T]`, `Some(T)`, and `None` are provided by the standard prelude.

### Lists

```
pub fn doubled_count() -> Int do
  let xs: List[Int] = [1, 2, 3]
  let doubled = map(xs, fn (x: Int) -> Int => x * 2)
  length(doubled)
end
```

`List[T]` is backed by Lua array tables. Empty list literals such as `[]` require an expected `List[T]` type from an annotation, return type, field, or argument context.

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
| `Option[T]` | Built-in/prelude optional value type with `Some(T)` and `None` |
| `List[T]` | Built-in/prelude Lua-array-backed list type |
| `type Point { x: Int, y: Int }` | Record type |
| `type Shape = \| Circle(Float) \| Rect { w: Float, h: Float }` | Sum type |
| `newtype UserId = Int` | Nominal wrapper over an underlying representation type |

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

Multiline parameter/argument/payload/match-arm lists may include trailing commas.

### Record field punning

```
type Point { x: Int, y: Int }

fn make(x: Int) -> Point do
  Point { x, y = 0 }
end
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

## Cheat Sheet

For a compact CLI + syntax reference, see [`docs/callisto_cheat_sheet.md`](docs/callisto_cheat_sheet.md).

## VS Code

Syntax highlighting extension source lives in:

- `editors/vscode/callisto-syntax`

It currently provides TextMate-based highlighting for `.cal` and `.luna` files.

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

Recent release completion, active `v0.6` planning, and the path to `v1.0` are tracked in:
- [`docs/roadmap_to_1_0.md`](docs/roadmap_to_1_0.md)
- [`docs/v0_3_draft_plan.md`](docs/v0_3_draft_plan.md)
- [`docs/v0_3_m4_release_checklist.md`](docs/v0_3_m4_release_checklist.md)
- [`docs/v0_4_draft_plan.md`](docs/v0_4_draft_plan.md)
- [`docs/v0_4_m0_scope_freeze_checklist.md`](docs/v0_4_m0_scope_freeze_checklist.md)
- [`docs/v0_4_m1_sample_depth_execution_checklist.md`](docs/v0_4_m1_sample_depth_execution_checklist.md)
- [`docs/v0_4_m2_parser_ergonomics_execution_checklist.md`](docs/v0_4_m2_parser_ergonomics_execution_checklist.md)
- [`docs/v0_4_m3_type_inference_execution_checklist.md`](docs/v0_4_m3_type_inference_execution_checklist.md)
- [`docs/v0_4_m4_release_checklist.md`](docs/v0_4_m4_release_checklist.md)
- [`docs/v0_6_draft_plan.md`](docs/v0_6_draft_plan.md)
- [`docs/v0_6_language_conformance_matrix.md`](docs/v0_6_language_conformance_matrix.md)
- [`docs/v0_6_m0_scope_freeze_checklist.md`](docs/v0_6_m0_scope_freeze_checklist.md)
- [`docs/v0_6_m1_nominal_types_execution_checklist.md`](docs/v0_6_m1_nominal_types_execution_checklist.md)
- [`docs/v0_6_m2_match_analysis_execution_checklist.md`](docs/v0_6_m2_match_analysis_execution_checklist.md)
- [`docs/v0_6_m3_pattern_conformance_execution_checklist.md`](docs/v0_6_m3_pattern_conformance_execution_checklist.md)
- [`docs/v0_6_m4_release_checklist.md`](docs/v0_6_m4_release_checklist.md)
