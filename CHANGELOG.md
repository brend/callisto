# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-04

Initial `v0.1` release.

### Added
- End-to-end compiler pipeline: lexer -> parser -> resolver -> typechecker -> TIR -> Lua codegen.
- CLI commands: `parse`, `check`, `emit-lua`, and `build` (alias of `emit-lua`).
- Recursive multi-file module loading from an entry file.
- Core language support for records, sum types, pattern matching, generics, methods (`impl`), lambdas, and record updates.
- Extern interop via `extern module` and `extern fn`.

### Changed
- README `v0.1` scope is now explicit about supported features, exclusions, and expected CLI behavior.
- `v0.1` completion checklist updated to reflect completed release-quality tasks.

### Fixed
- Improved diagnostics for import/module misuse, including calls on imported module aliases.
- Added targeted diagnostic notes for constructor and record payload/field mismatches.
- Expanded negative-path regression coverage for:
  - generic ADT inference failures
  - alias mismatch failures
  - import module/item misuse
  - non-exhaustive generic `match`
