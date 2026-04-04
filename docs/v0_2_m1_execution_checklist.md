# Callisto v0.2 M1 Execution Checklist

Execution checklist for `M1: Config + Resolver Foundation`.

Use this as the implementation board. Keep tasks checked only when code and tests are done.

## Scope Freeze

- [ ] `callisto.toml` is the config filename for `v0.2`.
- [ ] Config discovery rule is explicit and documented.
- [ ] CLI override precedence is explicit and documented.

## M1 Decisions (Implement As Written)

1. Config discovery
- Try explicit CLI `--config <path>` first.
- If no explicit config path is provided, look for `callisto.toml` in the entry file directory only.
- If no file exists, continue with defaults (not an error).

2. Config schema (`callisto.toml`)
- `module_roots = ["relative/or/absolute/path", ...]`
- `out_dir = "out"` (optional)
- `package = "my.module"` (optional)

3. Path semantics
- Relative paths in config are resolved against the config file directory.
- Module resolution search order is deterministic: CLI `--module-root` entries (in provided order), else config `module_roots` (in file order), else `[entry_dir]`.

4. Output directory precedence
- `emit-lua/build -o` wins over everything.
- If `-o` is not provided and config has `out_dir`, use config `out_dir`.
- Else use `out`.

## Implementation Tasks

### A) Config Model + Loader

- [ ] Add dependency support for config parsing in [Cargo.toml](/Users/waldrumpus/code/callisto/Cargo.toml).
- [ ] Add `ProjectConfig` and config parsing/validation in `src/config.rs`.
- [ ] Add `load_project_config(entry_input, explicit_config_path)` API in `src/config.rs`.
- [ ] Validate `module_roots` for empty/duplicate entries in `src/config.rs`.
- [ ] Resolve relative config paths against config file directory in `src/config.rs`.
- [ ] Wire module in [src/main.rs](/Users/waldrumpus/code/callisto/src/main.rs) (`mod config;`).

### B) CLI Surface

- [ ] Add `--config <path>` in [src/cli.rs](/Users/waldrumpus/code/callisto/src/cli.rs) for `check`, `emit-lua`, and `build`.
- [ ] Add repeatable `--module-root <path>` in [src/cli.rs](/Users/waldrumpus/code/callisto/src/cli.rs) for `check`, `emit-lua`, and `build`.
- [ ] Update CLI usage text to include new flags and precedence summary.
- [ ] Add CLI parser tests for valid and invalid flag combinations.

### C) Compiler Pipeline Plumbing

- [ ] Add a compile options struct in [src/main.rs](/Users/waldrumpus/code/callisto/src/main.rs) that carries resolved module roots, output default, and config-source metadata.
- [ ] Pass options through `check`, `emit-lua`, `build`, `compile_project`, and `load_module_graph`.

### D) Resolver Root Search

- [ ] Replace single-root lookup with multi-root search in [src/main.rs](/Users/waldrumpus/code/callisto/src/main.rs), evolving/replacing `find_module_file`.
- [ ] Preserve existing `.luna|.cal` and `mod.luna|mod.cal` candidate rules in new lookup logic.
- [ ] On failure, report attempted candidate paths in diagnostics notes.
- [ ] Preserve backward compatibility when no config/flags are present.

### E) Tests + Fixtures

- [ ] Add unit tests for config loading and validation.
- [ ] Add resolver tests for root search order.
- [ ] Add resolver tests for fallback to entry-dir defaults.
- [ ] Add resolver tests for unresolved-import diagnostics that include attempted paths.
- [ ] Add one integration-style multi-module fixture using two roots.

## Definition of Done (M1)

- [ ] `cargo test` passes.
- [ ] `cargo run -- check <entry.cal>` works with no config.
- [ ] `cargo run -- check <entry.cal>` works with config-only roots.
- [ ] `cargo run -- check <entry.cal>` works with CLI-only roots.
- [ ] `cargo run -- check <entry.cal>` works with CLI overriding config roots.
- [ ] `cargo run -- emit-lua <entry.cal>` default output behavior follows precedence rules.
- [ ] README and `docs/v0_2_draft_plan.md` reflect final M1 behavior.

## Suggested Task Order

1. `A) Config Model + Loader`
2. `B) CLI Surface`
3. `C) Compiler Pipeline Plumbing`
4. `D) Resolver Root Search`
5. `E) Tests + Fixtures`
