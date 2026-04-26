# Callisto v0.7 M3 Workflow Reliability Execution Checklist

## Implementation

- [x] Improve malformed/missing `callisto.toml` messages with config source context.
- [x] Improve module-root/import failures with attempted paths and suggested fixes.
- [x] Tighten output-directory handling and overwrite diagnostics.
- [x] Improve Playdate `pdc` execution failures with executable, source, output, and fix guidance.
- [x] Improve bootstrap validation messages around missing or mismatched `init`, `update`, and `render`.

## Tests

- [x] CLI/config regression tests for malformed config, missing explicit config, missing inputs, root precedence, and output directory selection.
- [x] Import-resolution tests assert attempted-path notes.
- [x] Fake-`pdc` tests cover generated source emission and `pdc` invocation.
- [x] Generated Playdate template flow is smoke-tested.

## Docs

- [x] Playdate workflow docs describe current failure modes and fixes.
- [x] Generated template README matches current CLI behavior.
