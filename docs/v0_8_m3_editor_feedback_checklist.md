# Callisto v0.8 M3 Editor Feedback Checklist

## VS Code

- [x] TextMate grammar recognizes brace-delimited v0.8 syntax.
- [x] Grammar covers `newtype`, `List[T]`, `Option[T]`, constructor patterns, record punning, and string interpolation.
- [x] Fixture uses modern Callisto constructs.
- [x] README documents local install and `npm test`.
- [x] `npm test` passes in `editors/vscode/callisto-syntax`.

## Zed

- [x] Tree-sitter grammar recognizes brace-delimited v0.8 syntax.
- [x] Highlight queries cover `newtype`, brace syntax, prelude types, constructor patterns, record punning, and string interpolation.
- [x] Fixture uses modern Callisto constructs.
- [x] README documents dev-extension install and `npm test`.
- [x] `npm test` passes in `editors/zed/callisto-syntax`.

## Future LSP Scope

- [x] Minimum viable LSP scope is documented as future work, not implemented in v0.8.
