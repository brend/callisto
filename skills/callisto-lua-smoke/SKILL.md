# Skill: Callisto Lua Smoke Validation

Use this skill after semantic or codegen changes to quickly validate emitted Lua.

## Input
A `.cal` source file (`.luna` input is legacy/deprecated naming).

## Steps
1. Run semantic check:
   - `cargo run -- check <file>`
2. Emit Lua:
   - `cargo run -- emit-lua <file> -o /tmp/callisto_out.lua`
3. Inspect output:
   - ensure module table exists (`local M = {}` / `return M`)
   - ensure public functions are exported on `M`
   - ensure records and record-pattern fields emit as Lua table field access
   - ensure match lowering checks constructor tags (for example `__scrutinee.tag == "Variant"`)
   - if extern paths are used, ensure calls emit fully-qualified Lua paths (for example `foo.bar.baz()`)
4. If output regressed, identify whether issue is in:
   - lowering (`src/typecheck.rs`)
   - TIR shape (`src/tir.rs`)
   - emitter (`src/codegen_lua.rs`)
5. Recommended smoke files:
   - `samples/imports_extern_interop.cal`
   - any temporary `.cal` program that exercises the changed feature
   - if reusing an older `.luna` sample, rename/copy it to `.cal` first

## Output Expectations
A short report with:
- command results
- output path
- one or two key Lua lines confirming expected lowering
