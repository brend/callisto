# Callisto v1.0.0 Readiness Checklist

This checklist tracks the remaining work before tagging `v1.0.0`.

## Compatibility Guarantees

- [x] Source syntax surface is frozen by `v0.9.0`.
- [x] Prelude names and helper forms are frozen by `v0.9.0`.
- [x] CLI/config behavior expected for stable Playdate workflows is documented.
- [x] Diagnostic-code policy remains based on stable `CAL-*` codes.
- [x] Generated Lua compatibility expectation is documented as readable, dependency-light Lua suitable for Playdate workflows.

## Docs

- [x] README describes current language and workflow surface.
- [x] Cheat sheet describes brace-only syntax, prelude helpers, and list indexing.
- [x] Playdate workflow docs describe shared bindings, auto-bootstrap, and `build-playdate`.
- [x] VS Code and Zed editor docs state highlighting-only scope and future LSP expectations.
- [x] v1.0 conformance matrix exists.

## Tests And Validation

- [x] Compiler regression suite passes for the frozen surface.
- [x] CLI smoke checks are listed in the v0.9 release gate.
- [x] Editor syntax regression checks are listed in the v0.9 release gate.
- [x] Playdate sample check/emit workflows are listed in the v0.9 release gate.

## Release Notes And Migration

- [x] `CHANGELOG.md` contains `0.9.0` stabilization notes.
- [x] Old keyword-delimited block syntax migration is already documented under `0.8.0`.
- [x] v0.9 notes identify the frozen prelude and list-helper additions.
- [x] Known exclusions are recorded: package management, broad Lua ecosystem support, method-style collection helpers, full LSP implementation, and further syntax redesigns.

## Final v1.0 Gate

- [ ] Update `Cargo.toml` to `1.0.0`.
- [ ] Add a dated `1.0.0` changelog entry.
- [ ] Confirm `cargo test` passes.
- [ ] Confirm CLI smoke checks pass.
- [ ] Confirm VS Code and Zed syntax regression checks pass.
- [ ] Confirm maintained Playdate samples still follow the documented workflow.
- [ ] Tag `v1.0.0`.

No unresolved `v1.0.0` blocker is known as of the `v0.9.0` compatibility freeze.
