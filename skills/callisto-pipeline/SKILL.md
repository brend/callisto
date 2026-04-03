# Skill: Callisto Pipeline Work

Use this skill when implementing or modifying compiler behavior across phases.

## Goal
Maintain a clean compiler pipeline and avoid phase leakage.

## Steps
1. Confirm the target feature against `docs/luna_compiler_architecture_v0_1.md`.
2. If syntax changes:
   - update `src/token.rs` and `src/lexer.rs` if tokens change
   - update `src/ast.rs` and `src/parser.rs`
3. Update semantics:
   - name/type IDs and registries in `src/types.rs` and `src/resolve.rs`
   - type rules and lowering in `src/typecheck.rs`
   - TIR shape in `src/tir.rs` if needed
4. Update Lua emission in `src/codegen_lua.rs` from TIR only.
5. Validate:
   - `cargo fmt`
   - `cargo test`
   - `cargo run -- check <sample.luna>`

## Output Expectations
- Compiler still runs end-to-end.
- No direct AST-to-Lua shortcuts.
- New behavior is diagnosed correctly on invalid input.
