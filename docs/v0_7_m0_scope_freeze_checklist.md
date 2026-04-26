# Callisto v0.7 M0 Scope Freeze + Prelude Design Checklist

Use this as the planning gate before implementation milestones start.

## Scope

- [x] `v0.7.0` scope is limited to the initial prelude and workflow reliability.
- [x] Prelude names are implicit globals, not imported modules.
- [x] Reserved prelude names are fixed: `Option`, `Some`, `None`, `List`, `length`, `map`.
- [x] `map` uses helper form only: `map(list, fn)`.
- [x] Empty list literals require expected `List[T]` context.
- [x] Out-of-scope items are documented in `docs/v0_7_draft_plan.md`.

## Validation Baseline

- [x] `cargo test`
- [x] `target/debug/callisto check playdate_auto_bootstrap/src/game.cal --config playdate_auto_bootstrap/callisto.toml`
- [x] `target/debug/callisto emit-lua playdate_auto_bootstrap/src/game.cal -o /tmp/callisto_v0_7_smoke --config playdate_auto_bootstrap/callisto.toml --playdate-bootstrap`

## Docs

- [x] `docs/v0_7_draft_plan.md` links all milestone checklists.
- [x] README and cheat sheet prelude docs are identified for M1/M2 updates.
- [x] Playdate workflow/sample updates are identified for M3/M4.
