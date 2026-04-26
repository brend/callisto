# Callisto v1.0 Frozen Language Conformance Matrix

This matrix tracks the language surface frozen by `v0.9.0` for the planned `v1.0.0` release.

Legend:
- `Covered`: parser/resolver/typechecker/codegen or diagnostic coverage exists.
- `N/A`: coverage is not meaningful for that pipeline stage.
- `Post-1.0`: explicitly excluded from the stable surface.

| Area | Status | Parser Coverage | Resolver/Typechecker Coverage | Codegen Coverage | Diagnostics / Notes |
|---|---|---|---|---|---|
| Brace-delimited blocks | Covered | `parser_accepts_v0_8_brace_block_forms`, old-syntax rejection tests in `src/main.rs` | Existing expression/declaration regressions use brace syntax | Full-pipeline and sample emit tests use brace syntax | Old `do`/`then`/`end` delimiters use `CAL-PAR-001`; `elseif` uses `CAL-PAR-002`. |
| Declarations (`type`/alias/`newtype`/`fn`/`impl`) | Covered | `parser_accepts_newtype_declarations`, `parses_multiline_and_single_line_sum_declarations` | `functions_are_predeclared_for_forward_references`, `newtype_is_not_assignable_from_underlying_type`, alias assignability tests | `full_pipeline_compiles_and_emits_lua_for_feature_rich_module`, `newtype_constructor_compiles_and_codegen_is_zero_overhead` | Transparent aliases and nominal wrappers are both part of the frozen surface. |
| Records and record updates | Covered | Record initializer/update paths in full-pipeline parser tests | `record_update_reports_unknown_and_mistyped_fields`, record field validation tests | `record_update_codegen_copies_base_before_overrides` | Field typo/missing/duplicate diagnostics remain stable typechecker behavior. |
| Sums, constructors, and pattern matching | Covered | Sum and match parser regressions | Constructor validation, duplicate arm, unreachable arm, and exhaustiveness tests | `lua_golden_sum_match`, constructor-pattern codegen tests | `CAL-TYP-030`, `CAL-TYP-031`, and `CAL-TYP-032` cover match completeness/reachability diagnostics. |
| Lambdas, calls, methods, and control flow | Covered | Full-pipeline parser regressions | Generic call inference, lambda/helper tests, assignment diagnostics | Full-pipeline Lua output tests | Statement separation remains newline-based; semicolons are not part of the frozen surface. |
| Modules and imports | Covered | Import/module parser paths in compile-pipeline tests | Multi-root module loading and import item/module resolution tests | Directory output tests for imported modules | Configured module roots and attempted-path notes are stable workflow behavior. |
| Extern interop | Covered | `extern type`, `extern fn`, and `extern module` parser paths | Imported member diagnostics and extern type paths | Extern module call emission tests | `Nil` and nullable `not` remain extern-context surface only. |
| Prelude `Option[T]`, `Some`, `None` | Covered | Generic type and constructor parser paths | `prelude_option_is_available_without_local_declaration`, reserved-name tests | Option constructor Lua emission assertions | Project-local conflicts use `CAL-RES-070`. |
| Prelude `List[T]`, list literals, `length`, `map` | Covered | List literal parser paths | List literal, empty-list context, helper arity/type diagnostics | `list_literals_length_and_map_compile_to_lua_arrays` | Empty `[]` requires expected `List[T]` context. |
| v0.9 list indexing, `append`, `filter`, `fold` | Covered | Index expression parser paths | `list_index_append_filter_and_fold_compile_to_lua_arrays`, helper diagnostics, reserved-name tests | Direct Lua table indexing and helper-loop assertions | Indexing uses Lua-style 1-based indexes; helpers are free functions only. |
| String interpolation | Covered | Editor fixtures and parser/string tests | Full-pipeline string interpolation coverage | `tests/golden/lua/string_interpolation.lua` | Escaped `\${...}` markers remain supported. |
| Playdate workflow surface | Covered | N/A | `callisto.toml`, bootstrap validation, and build workflow tests | Playdate binding emit tests and bootstrap emit tests | `build-playdate`, auto-bootstrap, and shared bindings are the recommended workflow. |
| Package management | Post-1.0 | N/A | N/A | N/A | Explicitly excluded before `v1.0.0`. |
| Method-style collection helpers | Post-1.0 | N/A | N/A | N/A | Use `map(xs, f)`, `filter(xs, f)`, `fold(xs, init, f)`, and `append(xs, value)`. |
| LSP implementation | Post-1.0 | N/A | N/A | N/A | Future minimum viable LSP remains diagnostics-on-save backed by `callisto check`. |

## Release Freeze Notes

- The frozen surface is source-compatible with the v0.9 syntax and prelude described in README and the cheat sheet.
- Further syntax redesigns are deferred beyond `v1.0.0`.
- No unresolved conformance blocker is known for `v1.0.0`.
