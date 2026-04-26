# Callisto v0.8 M2 Docs, Samples + Templates Checklist

## Source Migration

- [x] `samples/` use brace-delimited syntax.
- [x] `playdate_auto_bootstrap/` source uses brace-delimited syntax.
- [x] `playdate_bouncing_ball/` source uses brace-delimited syntax.
- [x] `playdate_bindings/` source uses brace-delimited syntax.
- [x] Generated Playdate template source in `src/main.rs` uses brace-delimited syntax.

## Docs

- [x] README examples use brace-delimited syntax.
- [x] Cheat sheet documents brace blocks, `else if`, and brace-delimited `match`.
- [x] Playdate workflow examples use brace-delimited syntax.
- [x] Docs no longer describe optional `match do` syntax.

## Coverage

- [x] `target/debug/callisto check samples/imports_extern_interop.cal`
- [x] `target/debug/callisto emit-lua samples/imports_extern_interop.cal -o /tmp/callisto_v0_8_imports.lua`
- [x] `target/debug/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
- [x] `target/debug/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o /tmp/callisto_v0_8_smoke --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`
