# Callisto v0.6 Language Conformance Matrix

This matrix tracks language-surface completeness for `v0.6`.

Legend:
- `Covered`: parser/checker/codegen coverage exists and is linked.
- `Partial`: behavior exists but coverage or diagnostics depth is incomplete.
- `Planned`: targeted for completion in `v0.6`.

| Area | Status | Parser Coverage | Typechecker Coverage | Codegen Coverage | Notes |
|---|---|---|---|---|---|
| Declarations (`type`/`newtype`/`fn`/`impl`) | Covered | `src/main.rs` (`parser_accepts_newtype_declarations`, `parses_multiline_and_single_line_sum_declarations`) | `src/main.rs` (`functions_are_predeclared_for_forward_references`, `type_aliases_compile_and_codegen_like_underlying_types`) | `src/main.rs` (`full_pipeline_compiles_and_emits_lua_for_feature_rich_module`) | Core declaration forms are covered across parse/check/emit paths. |
| Expressions (call/if/loops/lambda/record update) | Covered | `src/main.rs` (`full_pipeline_compiles_and_emits_lua_for_feature_rich_module`) | `src/main.rs` (`infers_generic_function_call_type_parameters`, `record_update_reports_unknown_and_mistyped_fields`) | `src/main.rs` (`record_update_codegen_copies_base_before_overrides`, `emit_lua_expr_statement_discards_value_with_local_binding`) | Includes lambda lowering through the full-pipeline regression. |
| Type aliases (`type Alias = ...`) | Covered | `src/main.rs` (`alias_mismatch_failure_is_reported`) | `src/main.rs` (`transparent_aliases_work_for_assignability_and_control_flow`) | `src/main.rs` (`type_aliases_compile_and_codegen_like_underlying_types`) | Transparent alias behavior remains explicit and regression-backed. |
| Nominal wrappers (`newtype`) | Covered | `src/main.rs` (`parser_accepts_newtype_declarations`) | `src/main.rs` (`newtype_is_not_assignable_from_underlying_type`, `generic_newtype_infers_from_payload_but_errors_when_unconstrained`) | `src/main.rs` (`newtype_constructor_compiles_and_codegen_is_zero_overhead`) | Implemented in `M1`. |
| Sum declarations + constructors | Covered | `src/main.rs` (`parses_multiline_and_single_line_sum_declarations`) | `src/main.rs` (`constructor_arity_and_record_fields_are_validated`, `generic_sum_and_record_constructors_infer_type_arguments`) | `src/main.rs` (`lua_golden_sum_match`) | Parser, checker, and Lua lowering remain aligned for ADT constructors. |
| Pattern matching (constructor/wildcard) | Covered | `src/main.rs` (`parser_accepts_trailing_commas_and_record_field_punning`) | `src/main.rs` (`reports_duplicate_constructor_match_arm`, `reports_unreachable_match_arm_after_catch_all`, `complete_sum_match_flags_following_arm_as_unreachable_without_duplicate_cascade`) | `src/main.rs` (`lua_golden_sum_match`) | Duplicate/unreachable diagnostics completed in `M2` (`CAL-TYP-031`, `CAL-TYP-032`). |
| Bool-pattern completeness | Covered | `src/main.rs` (`parser_accepts_trailing_commas_and_record_field_punning`) | `src/main.rs` (`reports_non_exhaustive_match_for_bool_cases`, `complete_bool_match_flags_following_arm_as_unreachable`) | `src/main.rs` (`lua_golden_sum_match`) | Bool exhaustiveness and reachability tightened in `M2` under `CAL-TYP-030`/`CAL-TYP-032`. |
| Record-constructor patterns | Covered | `src/main.rs` (`record_constructor_pattern_codegen_uses_named_fields`) | `src/main.rs` (`constructor_pattern_record_payload_shape_mismatch_is_non_cascading`, `constructor_pattern_positional_payload_shape_mismatch_is_non_cascading`, `constructor_pattern_record_field_typos_have_fixit_and_code`) | `src/main.rs` (`record_constructor_pattern_codegen_uses_named_fields`) | M3 adds stable pattern diagnostics/fix-its (`CAL-TYP-023`, `CAL-TYP-024`). |
| Functions/lambdas | Covered | `src/main.rs` (`full_pipeline_compiles_and_emits_lua_for_feature_rich_module`) | `src/main.rs` (`typecheck_reports_assignment_to_immutable_parameter`) | `src/main.rs` (`full_pipeline_compiles_and_emits_lua_for_feature_rich_module`) | Parser/checker/codegen coverage includes lambda forms and local function flow. |
| Modules/import resolution | Covered | `src/main.rs` (`compile_pipeline_loads_imported_module_files`) | `src/main.rs` (`import_module_alias_and_items_resolve_in_typecheck`) | `src/main.rs` (`emit_lua_writes_imported_modules_when_output_is_directory`) | Deterministic module-root search and emitted module output are regression-covered. |
| Extern interop | Covered | `src/main.rs` (`full_pipeline_compiles_and_emits_lua_for_feature_rich_module`) | `src/main.rs` (`imported_module_missing_member_reports_clear_error`) | `src/main.rs` (`extern_module_calls_emit_lua_paths`) | Extern path resolution and emitted Lua calls are covered. |

## M0 Fill-In Tasks

- [x] Replace all `TBD` cells with concrete file/test references.
- [x] Mark each row as `Covered`, `Partial`, or `Planned` based on concrete evidence.
- [x] Link any missing-coverage rows to follow-up tasks in `M1`/`M2`/`M3`.
