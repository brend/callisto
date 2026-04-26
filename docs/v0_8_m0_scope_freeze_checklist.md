# Callisto v0.8 M0 Scope Freeze + Syntax Policy Checklist

Use this as the planning gate before implementation milestones start.

## Scope

- [x] `v0.8.0` scope is limited to brace syntax migration and developer feedback.
- [x] Old `do`/`then`/`end` block syntax is rejected immediately, not deprecated.
- [x] `elseif` is replaced by `else if`.
- [x] Statement separation remains newline-based.
- [x] LSP implementation is deferred.
- [x] Out-of-scope items are documented in `docs/v0_8_draft_plan.md`.

## Validation Baseline

- [x] `cargo test`
- [x] Existing samples and Playdate projects identified for syntax migration.
- [x] VS Code and Zed syntax packages identified for regression updates.

## Docs

- [x] `docs/v0_8_draft_plan.md` links all milestone checklists.
- [x] README, cheat sheet, and Playdate workflow docs are identified for M2 updates.
- [x] Editor package READMEs are identified for M3 updates.
