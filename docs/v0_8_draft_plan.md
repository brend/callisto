# Callisto v0.8 Draft Plan

This document records the execution plan for `v0.8.0`.

`v0.8.0` completes the planned block-syntax break before the `v0.9.0` compatibility freeze and improves fast local feedback through parser diagnostics, `callisto check`, docs, and editor syntax packages.

Status: implementation validation complete as of 2026-04-26; awaiting maintainer sign-off/tagging.

## Why v0.8

- The language needs one stable block syntax before compatibility freeze.
- Old `do`/`then`/`end` blocks should not survive into `v0.9.0`.
- Users should get direct migration diagnostics from `callisto check`.
- Editor highlighting should track the modern Callisto surface.

## Release Guardrails

- Brace-delimited blocks are the only accepted source block syntax.
- Old `do`, `then`, and `end` delimiters are rejected with `CAL-PAR-001`.
- `elseif` is rejected with `CAL-PAR-002`; use `else if`.
- Keep statement separation newline-based; do not add semicolons.
- Do not implement an LSP server in this release.
- Preserve existing AST, resolver, typechecker, TIR, and Lua codegen semantics where possible.

## Scope (Must-Do)

1. Parse brace-delimited blocks for functions, `impl`, `extern module`, `if`, `match`, `while`, and `for`.
2. Reject old block delimiters with actionable migration diagnostics.
3. Update samples, generated templates, bindings, README examples, cheat sheet, Playdate workflow docs, and golden fixtures.
4. Update VS Code and Zed syntax packages for brace blocks and the modern language surface.
5. Document editor setup, local regression commands, and minimum viable future LSP scope.
6. Run compiler, CLI smoke, and editor regression checks before release.

## Out of Scope (v0.8)

- Language-server implementation.
- Semicolon-based statement separation.
- Package management or dependency resolution changes.
- Additional standard-prelude helpers beyond the existing v0.7 surface.
- Broad Lua backend redesign.

## Minimum Viable Future LSP

The first language-server slice should be diagnostics-on-save backed by `callisto check`. It should understand the active file, project `callisto.toml`, module roots, parser diagnostics, resolver diagnostics, and typechecker diagnostics. Completion, hover, formatting, rename, and code actions are deferred until after that diagnostic loop is reliable.

## Milestones

1. `M0: Scope Freeze + Syntax Policy`
- Lock brace-only syntax and immediate old-syntax rejection.
- Checklist: [`docs/v0_8_m0_scope_freeze_checklist.md`](docs/v0_8_m0_scope_freeze_checklist.md)

2. `M1: Parser Migration + Diagnostics`
- Implement brace block parsing and migration diagnostics.
- Checklist: [`docs/v0_8_m1_parser_migration_checklist.md`](docs/v0_8_m1_parser_migration_checklist.md)

3. `M2: Docs, Samples + Templates`
- Migrate source examples, generated templates, and user-facing docs.
- Checklist: [`docs/v0_8_m2_docs_samples_templates_checklist.md`](docs/v0_8_m2_docs_samples_templates_checklist.md)

4. `M3: Editor Feedback`
- Update VS Code and Zed syntax support and document future LSP scope.
- Checklist: [`docs/v0_8_m3_editor_feedback_checklist.md`](docs/v0_8_m3_editor_feedback_checklist.md)

5. `M4: Release Readiness`
- Final validation, changelog, version metadata, and release checks.
- Checklist: [`docs/v0_8_m4_release_checklist.md`](docs/v0_8_m4_release_checklist.md)

## Milestone Status

- `M0`: complete.
- `M1`: complete.
- `M2`: complete.
- `M3`: complete.
- `M4`: in progress.
