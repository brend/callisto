# Callisto v0.7 M1 Option Prelude Execution Checklist

## Implementation

- [x] Add built-in/predefined `Option[T]`.
- [x] Add `Some(T)` and `None` constructors.
- [x] Ensure generic constructor inference works for `Some` and contextual `None`.
- [x] Reject project declarations that conflict with reserved prelude names.
- [x] Keep emitted Lua compatible with existing sum constructor representation.

## Tests

- [x] Typecheck `Option[Int]`, `Some(1)`, and contextual `None` without a local type declaration.
- [x] Match on built-in `Option[T]`.
- [x] Reject unconstrained `None` where no expected `Option[T]` exists.
- [x] Reject user declarations for `Option`, `Some`, `None`, `List`, `length`, and `map`.
- [x] Update affected diagnostics/golden tests.

## Docs

- [x] README examples use built-in `Option[T]`.
- [x] Cheat sheet documents built-in `Option[T]`.
- [x] Migration notes explain removing local `Option` declarations.
