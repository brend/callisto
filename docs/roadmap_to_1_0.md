# Callisto Roadmap To 1.0

This document tracks Callisto's planned path from `v0.6.1` to `v1.0.0`.

`v1.0.0` means Callisto is a stable Playdate-oriented typed Lua language and workflow: dependable compiler behavior, clear diagnostics, documented project structure, maintained editor support, and samples that represent the recommended way to build Playdate projects.

## Baseline

`v0.6.1` provides the current foundation:

- A typed-to-Lua compiler pipeline with parser, resolver, typechecker, TIR, and Lua codegen coverage.
- Core language support for records, sums, generics, `impl` methods, lambdas, record updates, pattern matching, extern interop, modules, transparent aliases, and nominal `newtype` wrappers.
- A language conformance matrix covering major parser, checker, and codegen behavior.
- Project configuration through `callisto.toml`, module roots, output-directory precedence, and stable diagnostic codes.
- First-party Playdate workflows: project scaffolding, shared bindings, auto-bootstrap output, `build-playdate`, and maintained sample projects.
- Editor syntax packages for VS Code and Zed.

## Release Philosophy

- Each pre-1.0 release has one primary theme and a small number of must-ship tracks.
- Backward compatibility is preserved unless a breaking change is explicitly accepted and documented before implementation starts.
- Accepted pre-1.0 syntax breaks must land before `v0.9.0`, so `v0.9.0` can serve as the compatibility freeze for `v1.0.0`.
- Every release gate keeps `cargo test` green and updates release notes, relevant docs, and sample expectations.
- Planning documents should stay decision-oriented: enough to execute, but not so detailed that they duplicate implementation checklists before work starts.

## Non-Goals Before 1.0

- Full package or dependency management.
- Broad Lua ecosystem support beyond typed extern interop and predictable emitted Lua.
- Additional large syntax redesigns beyond the planned move to brace-delimited blocks.

## Planned 1.0 Language Additions

These language-surface additions should be included before the `v1.0.0` compatibility freeze:

- Built-in/predefined `List[T]` backed by Lua array tables, with core helpers such as `map`, `length`, and other small collection operations needed by real Playdate samples.
- Built-in/predefined `Option[T]` so optional values have one standard representation instead of every project defining its own `Option` sum type.
- Brace-delimited block syntax to replace the current keyword-delimited block style (`do`/`end` in current examples), giving functions, control flow, `impl`, `match`, and related constructs one consistent block form.

## v0.7: Core Library And Workflow Reliability

`v0.7.0` should add the small standard language surface needed by real programs while making existing project workflows feel predictable under normal and failure cases.

Must-do tracks:

- Add built-in/predefined `List[T]` as the standard typed view of Lua arrays.
- Add core list helpers, including at least `length` and `map`, with emitted Lua that uses normal array-style tables.
- Add built-in/predefined `Option[T]` and update docs/samples to prefer it over ad hoc project-local option definitions.
- Document the initial standard prelude surface and how it interacts with user modules, imports, generics, and emitted Lua.
- Harden `callisto.toml` behavior, including config discovery, module-root precedence, output-directory handling, and diagnostics for malformed or missing project inputs.
- Improve module-root and import-resolution failure messages so attempted paths, config sources, and suggested fixes are clear.
- Tighten Playdate build failure messages around `pdc`, generated output paths, bootstrap validation, and overwrite protection.
- Keep generated Playdate templates, sample projects, README snippets, and workflow docs aligned.
- Add regression coverage for prelude types/helpers, CLI/config combinations, and generated Playdate project flows.

Completion gate:

- `cargo test` passes.
- `List[T]`, `Option[T]`, and required helper functions have parser/checker/codegen or prelude-resolution coverage.
- Playdate template generation and documented sample commands are checked against current CLI behavior.
- Changelog and docs describe any workflow behavior changes.

## v0.8: Brace Syntax And Developer Feedback Loop

`v0.8.0` should complete the planned block-syntax migration and reduce the time between editing Callisto code and understanding what the compiler means.

Must-do tracks:

- Replace keyword-delimited blocks with brace-delimited blocks across functions, control flow, `impl`, `match`, extern declarations, and type/constructor forms where applicable.
- Decide whether the old `do`/`end` block style is rejected immediately or accepted with a deprecation diagnostic for one release, then document that policy.
- Update parser diagnostics so common migration mistakes point from old block delimiters to the new brace form.
- Update all samples, README examples, workflow docs, cheat sheet entries, and golden fixtures to use brace-delimited syntax.
- Improve VS Code and Zed syntax packages for the 1.0 language surface, including `newtype`, built-in `List[T]`/`Option[T]`, multiline sum declarations, record field punning, trailing commas, constructor patterns, extern declarations, and brace-delimited blocks.
- Document editor installation and local development workflows for both editor packages.
- Add or expand syntax regression coverage for modern Callisto constructs.
- Define the minimum viable language-server scope for Callisto, even if the full LSP implementation is deferred.
- Prioritize fast local feedback through `callisto check`, clearer diagnostics, and documented troubleshooting paths.

Completion gate:

- `cargo test` passes.
- Brace-delimited syntax is covered across parser, checker, codegen, samples, docs, and editor grammars.
- Editor syntax regression tests pass where available.
- README or editor-package docs explain the supported editor setup and known limits.

## v0.9: Compatibility And Stabilization

`v0.9.0` should freeze what Callisto intends to stabilize for `v1.0.0`.

Must-do tracks:

- Freeze the intended `v1.0.0` language surface and document any explicit exclusions.
- Confirm `List[T]`, `Option[T]`, and brace-delimited block syntax are part of the frozen surface.
- Close remaining conformance gaps or file them as post-1.0 work with clear rationale.
- Audit emitted Lua for readability, runtime assumptions, and Playdate compatibility.
- Harden shared Playdate bindings and sample projects against drift from the recommended workflow.
- Create the `v1.0.0` readiness checklist covering docs, tests, diagnostics, CLI behavior, compatibility guarantees, release notes, and migration notes.

Completion gate:

- `cargo test` passes.
- Language conformance docs reflect the frozen `v1.0.0` surface.
- A `v1.0.0` readiness checklist exists and identifies no unresolved release blockers.

## v1.0: Stable Playdate Language Release

`v1.0.0` should publish Callisto as a stable Playdate-first typed Lua toolchain.

Must-do tracks:

- Declare stable guarantees for the CLI, config file behavior, language surface, diagnostic-code policy, and generated Lua compatibility expectations.
- Publish final language docs, cheat sheet updates, Playdate workflow docs, and editor setup docs.
- Ensure maintained samples build with the documented workflow.
- Finalize changelog and release notes.
- Tag the release as `v1.0.0`.

Completion gate:

- `cargo test` passes.
- Release checklist is complete.
- README points new users to the stable workflow and current docs.
- `CHANGELOG.md` contains a dated `1.0.0` entry.
