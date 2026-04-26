# Callisto v0.9 M0 Scope Freeze + Compatibility Policy Checklist

Use this as the planning gate before stabilization milestones start.

## Scope

- [x] `v0.9.0` scope is limited to compatibility freeze, conformance, emitted Lua, Playdate workflow, docs, and release readiness.
- [x] List indexing plus `append`, `filter`, and `fold` are included in the release.
- [x] No additional syntax breaks are accepted in `v0.9.0`.
- [x] Package management, method-style collection helpers, broad Lua backend redesign, and LSP implementation are deferred.
- [x] Out-of-scope items are documented in `docs/v0_9_draft_plan.md`.

## Compatibility Policy

- [x] Frozen source syntax remains brace-delimited only.
- [x] Frozen prelude names are documented.
- [x] List indexing is documented as Lua-style 1-based indexing.
- [x] Post-1.0 exclusions are documented.

## Validation Baseline

- [x] Existing compiler regression coverage for list indexing and expanded list helpers identified.
- [x] Existing Playdate sample projects and shared bindings identified for stabilization checks.
- [x] VS Code and Zed syntax packages identified for fixture/doc updates.
