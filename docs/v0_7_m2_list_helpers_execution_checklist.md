# Callisto v0.7 M2 List Literals + Helpers Execution Checklist

## Implementation

- [x] Add `List[T]` as a predefined generic type backed by Lua array tables.
- [x] Parse list literals (`[a, b, c]`, `[]`).
- [x] Infer element type for non-empty list literals.
- [x] Require expected `List[T]` context for empty list literals.
- [x] Typecheck `length(xs)` as `Int`.
- [x] Typecheck `map(xs, fn(x: A) -> B => body)` as `List[B]`.
- [x] Emit list literals as Lua array tables.
- [x] Emit `length(xs)` as `#xs`.
- [x] Emit `map(xs, f)` as an inline loop expression returning a new array.

## Tests

- [x] Parser tests for list literals and trailing commas.
- [x] Checker tests for homogeneous lists, compatible numeric lists, mixed invalid lists, and empty-list context.
- [x] Codegen tests for list literals, `length`, and `map`.
- [x] Regression coverage for helper arity and argument diagnostics.

## Docs

- [x] README and cheat sheet document `List[T]`, literals, `length`, and `map`.
- [ ] Samples use lists where they improve clarity without broad rewrites.
