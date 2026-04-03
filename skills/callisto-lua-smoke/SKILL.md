# Skill: Callisto Lua Smoke Validation

Use this skill after semantic or codegen changes to quickly validate emitted Lua.

## Input
A `.luna` source file (or small temporary test program).

## Steps
1. Run semantic check:
   - `cargo run -- check <file.luna>`
2. Emit Lua:
   - `cargo run -- emit-lua <file.luna> -o /tmp/callisto_out.lua`
3. Inspect output:
   - ensure module table exists (`local M = {}` / `return M`)
   - ensure public functions are exported on `M`
   - ensure records emit as Lua tables
   - ensure variants emit `{ tag = ... }`-style tables
4. If output regressed, identify whether issue is in:
   - lowering (`src/typecheck.rs`)
   - TIR shape (`src/tir.rs`)
   - emitter (`src/codegen_lua.rs`)

## Output Expectations
A short report with:
- command results
- output path
- one or two key Lua lines confirming expected lowering
