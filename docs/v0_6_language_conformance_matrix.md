# Callisto v0.6 Language Conformance Matrix

This matrix tracks language-surface completeness for `v0.6`.

Legend:
- `Covered`: parser/checker/codegen coverage exists and is linked.
- `Partial`: behavior exists but coverage or diagnostics depth is incomplete.
- `Planned`: targeted for completion in `v0.6`.

| Area | Status | Parser Coverage | Typechecker Coverage | Codegen Coverage | Notes |
|---|---|---|---|---|---|
| Type aliases (`type Alias = ...`) | Covered | TBD | TBD | TBD | Validate links during `M0`. |
| Nominal wrappers (`newtype`) | Planned | TBD | TBD | TBD | Implement in `M1`. |
| Sum declarations + constructors | Covered | TBD | TBD | TBD | Keep multiline declaration regressions intact. |
| Pattern matching (constructor/wildcard) | Partial | TBD | TBD | TBD | Complete duplicate/unreachable diagnostics in `M2`. |
| Bool-pattern completeness | Partial | TBD | TBD | TBD | Tighten exhaustiveness/reachability behavior in `M2`. |
| Record-constructor patterns | Partial | TBD | TBD | TBD | Harden diagnostics and mismatch reporting in `M3`. |
| Functions/lambdas | Covered | TBD | TBD | TBD | Confirm explicit annotation paths remain stable. |
| Modules/imports/extern interop | Covered | TBD | TBD | TBD | Regression-check against sample projects. |

## M0 Fill-In Tasks

- [ ] Replace all `TBD` cells with concrete file/test references.
- [ ] Mark each row as `Covered`, `Partial`, or `Planned` based on concrete evidence.
- [ ] Link any missing-coverage rows to follow-up tasks in `M1`/`M2`/`M3`.
