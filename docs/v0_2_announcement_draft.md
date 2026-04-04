# Callisto v0.2.0 Announcement (Draft)

Callisto `v0.2.0` focuses on project-scale usability and release stability.

## Highlights

- Project config via `callisto.toml`:
  - `module_roots`
  - `out_dir`
  - optional `package`
- CLI/config integration:
  - `--config <path>`
  - repeatable `--module-root <path>`
  - deterministic precedence with `-o` override behavior
- Resolver diagnostics now include attempted-path notes for unresolved imports.
- Stable diagnostic error codes across config/resolve/typecheck hot paths.
- Golden fixtures for both diagnostics and emitted Lua.

## Reliability

- Full test suite passing on the release candidate (`cargo test`).
- Sample-project smoke checks passing for `check` and `emit-lua`.
- Release binary build + smoke tests passing (`cargo build --release` and `target/release/callisto`).

## Upgrade Notes

- Existing `v0.1` projects remain compatible by default.
- To use shared libraries across directories, add `module_roots` in `callisto.toml` or pass `--module-root` flags explicitly.
- Diagnostic output now includes stable codes (for example `CAL-RES-*`, `CAL-TYP-*`, `CAL-CFG-*`) for easier troubleshooting.
