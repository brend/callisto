# Callisto v0.7 M3 Workflow Reliability Execution Checklist

## Implementation

- [ ] Improve malformed/missing `callisto.toml` messages with config source context.
- [ ] Improve module-root/import failures with attempted paths and suggested fixes.
- [ ] Tighten output-directory handling and overwrite diagnostics.
- [x] Improve Playdate `pdc` execution failures with executable, source, output, and fix guidance.
- [ ] Improve bootstrap validation messages around missing or mismatched `init`, `update`, and `render`.

## Tests

- [ ] CLI/config regression tests for malformed config, missing explicit config, missing inputs, root precedence, and output directory selection.
- [ ] Import-resolution tests assert attempted-path notes.
- [ ] Fake-`pdc` tests cover missing executable and non-zero exit failure messages.
- [ ] Generated Playdate template flow is smoke-tested.

## Docs

- [ ] Playdate workflow docs describe current failure modes and fixes.
- [ ] Generated template README matches current CLI behavior.
