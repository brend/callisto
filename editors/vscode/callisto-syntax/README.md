# Callisto VS Code Extension (Syntax Highlighting)

This extension adds syntax highlighting for Callisto source files:

- `.cal`
- `.luna`

## Features

- Keyword highlighting for declarations and control flow (`fn`, `type`, `impl`, `if`, `match`, etc.)
- Module/import path highlighting (`module foo.bar`, `import foo.bar`)
- Type and constructor highlighting (`Int`, `Option`, `Some`, etc.)
- Local binding and assignment scopes (`let`, `var`, assignment targets)
- Parameter declaration scopes (`name: Type`)
- Function, method, and field access scopes (`foo()`, `obj.method()`, `obj.field`)
- Operator and punctuation highlighting
- String, number, boolean, wildcard (`_`), and `//` comment highlighting

## Run Grammar Regression Checks

From this directory:

```sh
npm test
```

This runs fixture-based checks in `tests/` against the TextMate regex patterns.

## Install Locally (Unpackaged)

1. Open VS Code.
2. Run `Extensions: Install from VSIX...` only if you have a `.vsix`.
3. For source-based install, use `Developer: Install Extension from Location...` and select this folder:
   - `editors/vscode/callisto-syntax`

## Package as VSIX

From this directory:

```sh
npx @vscode/vsce package
```

Then install the generated `.vsix` with `Extensions: Install from VSIX...`.
