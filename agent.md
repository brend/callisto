# Callisto Agent Guide

This repository contains `callisto`, a Callisto-to-Lua compiler implemented in Rust.

## Mission
- Keep implementation aligned with `docs/luna_compiler_architecture_v0_1.md`.
- Preserve the phase split:
  - lexer
  - parser + AST
  - resolution
  - typecheck + TIR lowering
  - Lua codegen
- Prefer small, phase-local changes over cross-phase shortcuts.

## Project Layout
- `src/span.rs`, `src/source.rs`, `src/diagnostics.rs`: shared compiler infra.
- `src/interner.rs`: string interning utility and symbol IDs.
- `src/token.rs`, `src/lexer.rs`: lexical layer.
- `src/ast.rs`, `src/parser.rs`: syntax layer.
- `src/types.rs`, `src/resolve.rs`, `src/typecheck.rs`, `src/tir.rs`: semantic layer.
- `src/codegen_lua.rs`: backend.
- `src/cli.rs`, `src/main.rs`: CLI and pipeline wiring.
- `samples/`: runnable language examples for parse/check/emit-lua smoke tests.
- `zed-extension-callisto/`: editor grammar/highlighting package.

## Developer Commands
- Format: `cargo fmt`
- Tests: `cargo test`
- Parse only: `cargo run -- parse <file.cal>`
- Full semantic check: `cargo run -- check <file.cal>`
- Emit Lua: `cargo run -- emit-lua <file.cal> [-o out.lua|out_dir]`
- Build alias: `cargo run -- build <file.cal> [-o out.lua|out_dir]`
- Canonical extension: `.cal`.
- `.luna` is deprecated and should be treated as legacy input naming.
- Emit default output when `-o` is omitted: `out/<module_or_file_stem>.lua`.

## Change Rules
- Keep spans and diagnostics intact for new syntax/semantics.
- Add parser constructs in AST first, then resolve/typecheck/codegen.
- Do not bypass TIR by generating Lua directly from AST.
- Prefer ID-based references (`TypeId`, `FuncId`, `VariantId`, `LocalId`, `TypeParamId`) for semantic wiring.
- Keep extern/import behavior consistent with current syntax:
  - `import foo.bar` and `import foo.bar.{item}`
  - `extern module foo.bar do ... end`

## Done Criteria
For feature work, expect all of the following:
1. `cargo fmt` clean.
2. `cargo test` passing.
3. At least one manual `check` and `emit-lua` run on a representative sample from `samples/`.
   - example: `cargo run -- check samples/imports_extern_interop.cal`
   - example: `cargo run -- emit-lua samples/imports_extern_interop.cal -o /tmp/callisto_imports_extern.lua`
4. Diagnostics are actionable and include correct spans.
