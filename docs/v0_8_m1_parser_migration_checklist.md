# Callisto v0.8 M1 Parser Migration + Diagnostics Checklist

## Parser

- [x] Parse function bodies with `{ ... }`.
- [x] Parse `impl` and `extern module` bodies with `{ ... }`.
- [x] Parse `while` and `for` bodies with `{ ... }`.
- [x] Parse `if ... { ... } else if ... { ... } else { ... }`.
- [x] Parse `match value { case Pattern => expr }`.
- [x] Keep record, constructor, update, import-list, interpolation, list, and type-parameter braces intact.

## Diagnostics

- [x] Reject `do`, `then`, and `end` block delimiters with `CAL-PAR-001`.
- [x] Reject `elseif` with `CAL-PAR-002`.
- [x] Include replacement guidance in migration diagnostics.
- [x] Include `RBrace`, `else`, `else if`, match arms, and commas in recovery boundaries.

## Coverage

- [x] Parser accepts every v0.8 brace block form.
- [x] Parser rejects old function, control-flow, `impl`, `extern module`, and `match` block forms.
- [x] Parser rejects `elseif` with the migration code.
- [x] `cargo test` passes.
