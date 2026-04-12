# Callisto Syntax Cheat Sheet

Compiler-accurate quick reference for `.cal` / `.luna` source syntax.

## File layout

Top-level order is:

1. Optional `module ...`
2. Zero or more `import ...`
3. Declarations (`type`, `fn`, `extern ...`, `impl ...`)

```cal
module game.main
import math.vec2
import math.vec2.{length}

pub type Vec2 { x: Float, y: Float }
```

## Lexical basics

- Line comments: `// like this`
- Strings: `"text"`
- String interpolation: `"Hello ${name}"`
- Escape interpolation marker: `"\\${literal}"` (renders `${literal}`)
- Booleans: `true`, `false`
- Unit value/type: `()` / `Unit`
- Statements are newline-separated (no semicolons)

## Imports

```cal
import foo.bar
import foo.bar.{baz, qux}
```

- `import foo.bar` binds module alias `bar`.
- `import foo.bar.{baz}` brings `baz` into scope directly.
- There is no `as` alias syntax.

## Declarations

### Functions

```cal
fn add(a: Int, b: Int) -> Int do
  a + b
end

fn add_multiline(
  a: Int,
  b: Int,
) -> Int do
  a + b
end

pub fn log(msg: String) do
  ()
end
```

- Params are always typed.
- Return type is optional; omitted means `Unit`.
- Multiline lists allow trailing commas.

### Types

```cal
type Distance = Int

type Vec2 { x: Float, y: Float }

type Option[T] =
  | None
  | Some(T)

type Shape =
  | Circle { radius: Int }
  | Rect { w: Int, h: Int }
```

- Generic params use `[T, U]`.
- Sum variants can be:
- No payload: `None`
- Positional payload: `Some(T)`
- Record payload: `Circle { radius: Int }`

### Externs

```cal
extern type PDImage

extern fn now_ms() -> Int

extern module playdate.graphics do
  extern fn clear() -> Unit
  extern fn drawText(text: String, x: Int, y: Int) -> Unit
end
```

- `extern module` uses `do ... end`.
- Members inside must be declared as `extern fn`.

### Impl blocks

```cal
impl Vec2 do
  fn moved(self: Vec2, dx: Int, dy: Int) -> Vec2 do
    self with { x = self.x + dx, y = self.y + dy }
  end
end
```

## Type expressions

```cal
Int
Option[Int]
Int -> Bool
(Int)
Int not   // nullable, extern context only
Nil       // extern context only
```

- Function type arrows are right-associative (`A -> B -> C`).

## Statements and blocks

```cal
let x: Int = 1
var total = 0
total = total + x

return total
return

while total < 10 do
  total = total + 1
end

for i in 0..10 do
  total = total + i
end
```

- Assignment target must be a local name (not `obj.field = ...`).
- Blocks are expression-oriented: last expression becomes block value.

## Expressions

```cal
123
3.14
"hi"
true
()

foo(1, 2)
obj.field
obj.method(1)

Vec2 { x = 1, y = 2 }      // record init
Vec2 { x, y = 2 }          // record field punning (`x = x`)
Some(1)                    // positional constructor
None                       // nullary constructor
p with { x = p.x + 1 }     // record update

if cond then
  1
elseif other then
  2
else
  3
end

match value do
  case Some(v) => v
  case None => 0,
end

let inc = fn (x: Int) -> Int => x + 1
```

- `if` is an expression and requires `else`.
- `match` supports optional `do` (`match value do ... end` or `match value ... end`).
- Trailing commas are accepted in multiline params/args/payloads/match arms.

## Patterns (`match case`)

```cal
case _ => 0
case n => n
case 0 => 1
case true => 1
case "ok" => 1
case Some(v) => v
case Circle { radius } => radius
case Circle { radius = r } => r
```

## Operators (lowest to highest)

1. `or`
2. `and`
3. `==`, `!=`
4. `<`, `<=`, `>`, `>=`
5. `+`, `-`
6. `*`, `/`, `%`
7. `with` (record update)
8. Postfix: call `()`, field `.x`, method `.f(...)`

Unary operators: `-x`, `not x`.

## Practical gotchas

- Constructors are parsed from uppercase identifiers (`Some`, `None`, `Circle`).
- Record/constructor field initializers use `=` (not `:`): `{ x = 1 }`.
- Record initializers support punning: `{ x }` is equivalent to `{ x = x }`.
- Type fields use `:` (not `=`): `{ x: Int }`.
- `Int not` and `Nil` are only valid in extern type positions.
- `import` declarations must appear before top-level declarations.
