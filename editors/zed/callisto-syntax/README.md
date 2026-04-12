# Callisto Zed Extension

Syntax highlighting support for Callisto (`.cal`, `.luna`) in the Zed editor.

## Included

- `extension.toml` manifest
- `languages/callisto/config.toml` language metadata
- `languages/callisto/highlights.scm` Tree-sitter highlight queries
- Bundled `tree-sitter-callisto` grammar source

This extension includes interpolation highlighting for strings, including `${expr}` segments and escaped interpolation markers like `\${literal}`.

## Install as a Dev Extension

1. Open Zed.
2. Run `zed: install dev extension`.
3. Select this directory: `editors/zed/callisto-syntax`.

## Run Regression Checks

From this directory:

```sh
npm test
```

This regenerates the bundled Tree-sitter parser and verifies parsing + highlight captures against `tests/fixtures/highlighting.cal`.

## Notes

- The grammar source currently lives in this repository under `tree-sitter-callisto`.
- `extension.toml` currently points at a local `file://` grammar path suitable for this workspace. If you move the project, update `[grammars.callisto].repository` accordingly.
