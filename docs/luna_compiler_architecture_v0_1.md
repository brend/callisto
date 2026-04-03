# Luna Compiler Architecture v0.1

Below is a practical **Rust-oriented compiler architecture** for Luna v0.1.

The aim is not academic purity. The aim is a codebase you can actually build and evolve.

---

# 1. Overall pipeline

I recommend this pipeline:

```txt
source text
-> lexer
-> parser
-> AST
-> name resolution
-> type checking
-> typed IR
-> Lua code generation
-> .lua output
```

For v0.1, that is enough.

I would **not** introduce a separate optimization IR yet. The first internal split should be:

- **AST**: close to syntax
- **typed IR**: semantically resolved, simpler for codegen

That keeps the compiler understandable.

---

# 2. Crate / module structure

You can start in a single Rust crate with modules, then split later if needed.

Initial layout:

```txt
luna/
  Cargo.toml
  src/
    main.rs
    cli.rs
    source.rs
    span.rs
    diagnostics.rs
    lexer.rs
    token.rs
    parser.rs
    ast.rs
    resolve.rs
    types.rs
    typecheck.rs
    tir.rs
    codegen_lua.rs
    interner.rs
```

That is enough for a serious v0.1.

If it grows, split into crates later:

- `luna_syntax`
- `luna_semantics`
- `luna_codegen_lua`
- `luna_cli`

Do not start with multiple crates unless you already feel the need.

---

# 3. High-level design principle

Use **IDs instead of deep copying semantic structures**.

In practice:

- AST nodes own syntax data
- type declarations get stable IDs
- functions get stable IDs
- local variables get stable IDs
- resolved names point to IDs
- types in the checker are represented by compact enums and IDs

This keeps later phases simpler.

---

# 4. Core infrastructure

## 4.1 Source files

You need a representation for source text and file identity.

```rust
pub type FileId = u32;

pub struct SourceFile {
    pub id: FileId,
    pub path: std::path::PathBuf,
    pub text: String,
}
```

Then a small database:

```rust
pub struct SourceDb {
    files: Vec<SourceFile>,
}
```

This is enough for v0.1.

---

## 4.2 Spans

Every token and every syntax node should carry a span.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub file_id: FileId,
    pub start: u32,
    pub end: u32,
}
```

Do this from day one. Do not postpone spans.

A compiler without consistent spans becomes painful immediately.

---

## 4.3 Diagnostics

Start with a simple structure:

```rust
pub enum DiagnosticLevel {
    Error,
    Warning,
}

pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub primary_span: Span,
    pub notes: Vec<(Span, String)>,
}
```

And an accumulator:

```rust
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}
```

You can later add prettier rendering.
For now, correctness of spans matters more than beautiful formatting.

---

# 5. Lexing

## 5.1 Token design

Use a token kind enum plus span.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    IntLit,
    FloatLit,
    StringLit,

    KwModule,
    KwImport,
    KwPub,
    KwExtern,
    KwType,
    KwFn,
    KwImpl,
    KwLet,
    KwVar,
    KwIf,
    KwThen,
    KwElseIf,
    KwElse,
    KwMatch,
    KwCase,
    KwWhile,
    KwDo,
    KwFor,
    KwIn,
    KwReturn,
    KwEnd,
    KwTrue,
    KwFalse,
    KwAnd,
    KwOr,
    KwNot,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Colon,
    Arrow,      // ->
    FatArrow,   // =>
    Eq,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Pipe,
    DotDot,     // ..

    Newline,
    Eof,
}
```

You may choose to skip `Newline` tokens if your parser does not need them, but for this language they can help separate constructs cleanly.

## 5.2 Token structure

```rust
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub lexeme: String, // optional; you can optimize later
}
```

For a first version, storing the lexeme is fine.

---

# 6. Parser

I recommend:

- **handwritten recursive descent**
- **Pratt parser** for expressions

That is the best fit here.

Why:
- grammar is small
- precedence matters
- you want control over diagnostics
- parser generators are unnecessary overhead right now

## 6.1 Parser responsibilities

The parser should produce only syntax structure.
It should **not** resolve names or types.

## 6.2 Parser output

One file parses to:

```rust
pub struct Module {
    pub module_decl: Option<ModuleDecl>,
    pub imports: Vec<ImportDecl>,
    pub decls: Vec<TopDecl>,
}
```

---

# 7. AST design

The AST should stay close to source syntax.

## 7.1 Top-level AST

```rust
pub struct ModuleDecl {
    pub span: Span,
    pub path: Vec<String>,
}

pub struct ImportDecl {
    pub span: Span,
    pub path: Vec<String>,
    pub items: Option<Vec<String>>,
}
```

Top-level declarations:

```rust
pub enum TopDecl {
    Type(TypeDecl),
    Func(FuncDecl),
    ExternType(ExternTypeDecl),
    ExternFunc(ExternFuncDecl),
    ExternModule(ExternModuleDecl),
    Impl(ImplDecl),
}
```

Each declaration should include visibility.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Public,
}
```

---

## 7.2 Type declarations AST

```rust
pub struct TypeDecl {
    pub span: Span,
    pub vis: Visibility,
    pub name: String,
    pub type_params: Vec<String>,
    pub body: TypeDeclBody,
}

pub enum TypeDeclBody {
    Alias(TypeExpr),
    Record(Vec<RecordFieldType>),
    Sum(Vec<SumVariantDecl>),
}

pub struct RecordFieldType {
    pub span: Span,
    pub name: String,
    pub ty: TypeExpr,
}
```

Sum variants:

```rust
pub struct SumVariantDecl {
    pub span: Span,
    pub name: String,
    pub payload: SumVariantPayload,
}

pub enum SumVariantPayload {
    None,
    Positional(Vec<TypeExpr>),
    Record(Vec<RecordFieldType>),
}
```

---

## 7.3 Type expressions AST

Because v0.1 is nominal, this can stay fairly small.

```rust
pub enum TypeExpr {
    Named {
        span: Span,
        name: String,
        args: Vec<TypeExpr>,
    },
    Func {
        span: Span,
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },
    Nullable {
        span: Span,
        inner: Box<TypeExpr>, // extern-only
    },
    Nil {
        span: Span, // extern-only
    },
    Unit {
        span: Span,
    },
}
```

If you adopt “named records only”, then there is no anonymous record type expression node. I recommend that.

---

## 7.4 Functions AST

```rust
pub struct FuncDecl {
    pub span: Span,
    pub vis: Visibility,
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub ret_ty: TypeExpr,
    pub body: Block,
}

pub struct Param {
    pub span: Span,
    pub name: String,
    pub ty: TypeExpr,
}
```

Extern function:

```rust
pub struct ExternFuncDecl {
    pub span: Span,
    pub vis: Visibility,
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: TypeExpr,
}
```

Extern type:

```rust
pub struct ExternTypeDecl {
    pub span: Span,
    pub vis: Visibility,
    pub name: String,
    pub type_params: Vec<String>,
}
```

Extern module:

```rust
pub struct ExternModuleDecl {
    pub span: Span,
    pub vis: Visibility,
    pub path: Vec<String>,
    pub funcs: Vec<ExternFuncDecl>,
}
```

---

## 7.5 Impl AST

```rust
pub struct ImplDecl {
    pub span: Span,
    pub target: String,
    pub methods: Vec<FuncDecl>,
}
```

You may later resolve `target` to a `TypeId`.

---

## 7.6 Statements and blocks AST

```rust
pub struct Block {
    pub span: Span,
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>,
}
```

This is a very good design for an expression-oriented language with explicit `return`.

Statements:

```rust
pub enum Stmt {
    Let(LetStmt),
    Var(VarStmt),
    Assign(AssignStmt),
    Expr(ExprStmt),
    Return(ReturnStmt),
    While(WhileStmt),
    For(ForStmt),
}
```

Bindings:

```rust
pub struct LetStmt {
    pub span: Span,
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
}

pub struct VarStmt {
    pub span: Span,
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
}
```

Assignment, in v0.1, only to locals:

```rust
pub struct AssignStmt {
    pub span: Span,
    pub target: String,
    pub value: Expr,
}
```

That deliberately excludes arbitrary lvalues.

---

## 7.7 Expressions AST

This is the most important piece.

```rust
pub enum Expr {
    Int(IntExpr),
    Float(FloatExpr),
    String(StringExpr),
    Bool(BoolExpr),
    Unit(UnitExpr),

    Var(VarExpr),
    Path(PathExpr),

    Call(CallExpr),
    Field(FieldExpr),
    MethodCall(MethodCallExpr),

    Binary(BinaryExpr),
    Unary(UnaryExpr),

    If(IfExpr),
    Match(MatchExpr),

    RecordInit(RecordInitExpr),
    RecordUpdate(RecordUpdateExpr),
    Constructor(ConstructorExpr),

    Lambda(LambdaExpr),

    Paren(ParenExpr),
}
```

Notes:

- `PathExpr` is useful for `playdate.graphics.clear` or imported module references.
- `ConstructorExpr` covers sum constructors.
- `RecordInitExpr` covers `Vec2 { x = 1.0, y = 2.0 }`.
- `MethodCallExpr` can be lowered later to normal call form.

Examples of a few structs:

```rust
pub struct VarExpr {
    pub span: Span,
    pub name: String,
}

pub struct PathExpr {
    pub span: Span,
    pub segments: Vec<String>,
}
```

Call:

```rust
pub struct CallExpr {
    pub span: Span,
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
}
```

Field:

```rust
pub struct FieldExpr {
    pub span: Span,
    pub receiver: Box<Expr>,
    pub name: String,
}
```

Method call:

```rust
pub struct MethodCallExpr {
    pub span: Span,
    pub receiver: Box<Expr>,
    pub method: String,
    pub args: Vec<Expr>,
}
```

If expression:

```rust
pub struct IfExpr {
    pub span: Span,
    pub branches: Vec<(Expr, Block)>, // cond, body
    pub else_branch: Box<Block>,
}
```

I recommend using `Block` for branches, even when they contain a single expression. This is more uniform.

Match:

```rust
pub struct MatchExpr {
    pub span: Span,
    pub scrutinee: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

pub struct MatchArm {
    pub span: Span,
    pub pattern: Pattern,
    pub body: Block,
}
```

Record init:

```rust
pub struct RecordInitExpr {
    pub span: Span,
    pub type_name: String,
    pub fields: Vec<RecordFieldInit>,
}

pub struct RecordFieldInit {
    pub span: Span,
    pub name: String,
    pub value: Expr,
}
```

Record update:

```rust
pub struct RecordUpdateExpr {
    pub span: Span,
    pub base: Box<Expr>,
    pub fields: Vec<RecordFieldInit>,
}
```

Constructor:

```rust
pub struct ConstructorExpr {
    pub span: Span,
    pub name: String,
    pub payload: ConstructorPayload,
}

pub enum ConstructorPayload {
    None,
    Positional(Vec<Expr>),
    Record(Vec<RecordFieldInit>),
}
```

Lambda:

```rust
pub struct LambdaExpr {
    pub span: Span,
    pub params: Vec<Param>,
    pub ret_ty: TypeExpr,
    pub body: Box<Expr>, // v0.1 lambda is expr-bodied only
}
```

---

# 8. Patterns AST

v0.1 patterns are small, which is good.

```rust
pub enum Pattern {
    Wildcard { span: Span },
    Bind { span: Span, name: String },
    Int { span: Span, value: i64 },
    Bool { span: Span, value: bool },
    String { span: Span, value: String },

    Constructor {
        span: Span,
        name: String,
        args: Vec<Pattern>,
    },

    RecordConstructor {
        span: Span,
        name: String,
        fields: Vec<RecordPatternField>,
    },
}
```

Fields:

```rust
pub struct RecordPatternField {
    pub span: Span,
    pub name: String,
    pub pattern: Option<Pattern>, // `radius` sugar means bind same name
}
```

That gives you room for both:

```txt
case Circle { radius }
case Pair { left = x, right = y }
```

even if you initially support only the shorthand case.

---

# 9. Name resolution phase

After parsing, perform a separate **name resolution** pass.

This phase should answer:

- what type does a type name refer to?
- what constructor does a constructor name refer to?
- what function does a function name refer to?
- what local variable does an identifier refer to?
- what method belongs to which impl target?
- what imported name is in scope?

Do not mix this into parsing.

## 9.1 IDs

Use stable IDs:

```rust
pub struct TypeId(u32);
pub struct FuncId(u32);
pub struct VariantId(u32);
pub struct LocalId(u32);
pub struct ModuleId(u32);
```

Resolved names should point to IDs, not strings.

## 9.2 Symbol tables

You need:

- module/global scope
- local lexical scopes
- type parameter scopes

A simple stack of hash maps for locals is enough.

Global resolution can use registries like:

```rust
HashMap<String, TypeId>
HashMap<String, FuncId>
HashMap<String, VariantId>
```

Later this becomes module-qualified.

## 9.3 Output of resolution

You can either:
- annotate AST with resolution info, or
- build a separate resolved map keyed by node IDs

For v0.1, the simplest path is to add **node IDs** to AST nodes and keep side tables.

Example:

```rust
pub struct ExprId(u32);
pub struct PatternId(u32);
```

Then maps like:

```rust
HashMap<ExprId, ValueRes>
HashMap<TypeExprId, TypeRes>
```

This avoids rewriting the AST repeatedly.

---

# 10. Type system representation

You need two different notions of type:

- syntax-level `TypeExpr`
- semantic-level `Type`

Semantic type:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Unit,

    Named(TypeId, Vec<Type>),
    Func(Vec<Type>, Box<Type>),

    TypeParam(TypeParamId),

    ForeignNil,          // extern-only
    ForeignNullable(Box<Type>), // extern-only
    Error,
}
```

`Error` is important so typechecking can continue after a failure.

I would avoid representing records and sums structurally here; use `Named(TypeId, args)` and let the type definition table describe what that named type is.

---

# 11. Semantic registries

You need central tables describing declarations.

## 11.1 Type table

```rust
pub struct TypeInfo {
    pub name: String,
    pub vis: Visibility,
    pub params: Vec<TypeParamId>,
    pub kind: TypeKind,
}

pub enum TypeKind {
    Alias(Type),
    Record(Vec<FieldInfo>),
    Sum(Vec<VariantInfo>),
    ExternOpaque,
}
```

Fields and variants:

```rust
pub struct FieldInfo {
    pub name: String,
    pub ty: Type,
}

pub struct VariantInfo {
    pub id: VariantId,
    pub name: String,
    pub payload: VariantPayload,
}

pub enum VariantPayload {
    None,
    Positional(Vec<Type>),
    Record(Vec<FieldInfo>),
}
```

## 11.2 Function table

```rust
pub struct FuncInfo {
    pub name: String,
    pub vis: Visibility,
    pub type_params: Vec<TypeParamId>,
    pub params: Vec<Type>,
    pub ret: Type,
    pub kind: FuncKind,
}
```

Kinds:

```rust
pub enum FuncKind {
    Normal,
    Extern,
    Method { self_type: TypeId },
}
```

---

# 12. Typechecking

This should be a separate pass over resolved AST.

Responsibilities:

- infer local binding types when omitted
- validate explicit annotations
- typecheck function bodies
- typecheck `if` and `match`
- enforce `return` type consistency
- ensure `nil` / nullable types appear only in extern contexts
- ensure record updates are valid
- ensure pattern matches are type-correct and exhaustive where required

## 12.1 Local environment

For each function body:

```rust
pub struct LocalInfo {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
}
```

Store by `LocalId`.

## 12.2 Expression typing output

Each expression should get a semantic type:

```rust
HashMap<ExprId, Type>
```

Or better, produce typed IR directly.

I recommend eventually producing typed IR rather than attaching types only to AST.

---

# 13. Typed IR (TIR)

This is the representation codegen should consume.

It should be simpler than AST:

- names resolved
- method calls desugared
- constructor usage explicit
- every expression has a known type
- only semantically valid constructs remain

## 13.1 Why TIR matters

Without TIR, your Lua emitter becomes entangled with:
- name lookup
- method desugaring
- ad hoc semantic decisions

That gets ugly fast.

## 13.2 TIR sketch

```rust
pub struct TirModule {
    pub types: Vec<TirTypeDecl>,
    pub funcs: Vec<TirFunc>,
}
```

Functions:

```rust
pub struct TirFunc {
    pub id: FuncId,
    pub name: String,
    pub params: Vec<TirParam>,
    pub ret_ty: Type,
    pub body: TirBlock,
    pub kind: TirFuncKind,
}
```

Blocks:

```rust
pub struct TirBlock {
    pub stmts: Vec<TirStmt>,
    pub tail: Option<TirExpr>,
}
```

Statements:

```rust
pub enum TirStmt {
    Let { local: LocalId, ty: Type, value: TirExpr, mutable: bool },
    Assign { local: LocalId, value: TirExpr },
    Expr(TirExpr),
    Return(Option<TirExpr>),
    While { cond: TirExpr, body: TirBlock },
    ForRange { local: LocalId, start: TirExpr, end: TirExpr, body: TirBlock },
}
```

Expressions:

```rust
pub struct TirExpr {
    pub ty: Type,
    pub kind: TirExprKind,
}
```

Kinds:

```rust
pub enum TirExprKind {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Unit,

    Local(LocalId),
    Func(FuncId),
    ExternPath(Vec<String>),

    Call {
        callee: Box<TirExpr>,
        args: Vec<TirExpr>,
    },

    Field {
        base: Box<TirExpr>,
        field: String,
    },

    Binary {
        op: TirBinaryOp,
        left: Box<TirExpr>,
        right: Box<TirExpr>,
    },

    Unary {
        op: TirUnaryOp,
        expr: Box<TirExpr>,
    },

    If {
        branches: Vec<(TirExpr, TirBlock)>,
        else_branch: TirBlock,
    },

    Match {
        scrutinee: Box<TirExpr>,
        arms: Vec<TirMatchArm>,
    },

    RecordInit {
        type_id: TypeId,
        fields: Vec<(String, TirExpr)>,
    },

    RecordUpdate {
        base: Box<TirExpr>,
        type_id: TypeId,
        fields: Vec<(String, TirExpr)>,
    },

    VariantInit {
        variant_id: VariantId,
        payload: TirVariantPayload,
    },

    Lambda {
        params: Vec<(LocalId, Type)>,
        body: Box<TirExpr>,
    },
}
```

This is more than enough for v0.1.

---

# 14. Lua code generation strategy

The emitter should consume TIR only.

## 14.1 Codegen principles

Emit simple, readable Lua.
Do not try to be clever in v0.1.

## 14.2 Records

Luna:

```txt
Vec2 { x = 1.0, y = 2.0 }
```

Lua:

```lua
{ x = 1.0, y = 2.0 }
```

## 14.3 Record update

Luna:

```txt
player with { hp = player.hp - 1 }
```

Lua:

```lua
(function(__base)
    local __tmp = {}
    for k, v in pairs(__base) do
        __tmp[k] = v
    end
    __tmp.hp = __base.hp - 1
    return __tmp
end)(player)
```

Not elegant, but correct for v0.1.

Later you can optimize based on known record shapes.

## 14.4 Sum constructors

Luna:

```txt
Some(42)
None
Circle { radius = 10.0 }
```

Lua:

```lua
{ tag = "Some", _1 = 42 }
{ tag = "None" }
{ tag = "Circle", radius = 10.0 }
```

This is the obvious v0.1 representation.

## 14.5 Match

Generate chained tests on `.tag`, literals, or booleans.
Pattern matching can compile to nested `if` blocks with temporary locals.

That is fine for a first version.

## 14.6 Methods

Lower methods to ordinary Lua functions:

```lua
local function Vec2_add(self, other)
    ...
end
```

Then `a.add(b)` in Luna should already have been lowered in TIR to a function call equivalent, or codegen can emit direct helper calls.

I recommend lowering method calls before codegen.

## 14.7 Modules

Emit one Lua file per Luna module.
Each file returns a table of public bindings.

Example:

```lua
local M = {}

function M.is_alive(e)
    return e.hp > 0
end

return M
```

That aligns naturally with Lua module style.

---

# 15. CLI architecture

Start with a simple CLI:

```txt
luna build src/main.luna
luna check src/main.luna
luna emit-lua src/main.luna -o out/
```

Main entry:

```rust
fn main() {
    let cli = cli::parse();
    match cli.command {
        ...
    }
}
```

The build flow should be:

1. load source
2. lex
3. parse
4. resolve
5. typecheck
6. lower to TIR
7. emit Lua
8. print diagnostics and exit nonzero on errors

---

# 16. Recommended implementation order

This is the order I would actually follow.

## Phase 1: syntax only
Implement:

- source files
- spans
- diagnostics
- lexer
- parser
- AST dump / pretty debug output

Target:
- parse a file and print the AST

## Phase 2: declarations and resolution
Implement:

- type/function registries
- top-level symbol collection
- import/module path handling
- local variable scopes
- constructor resolution

Target:
- detect duplicate names and unresolved names

## Phase 3: minimal typechecker
Implement:

- primitives
- `Unit`
- nominal types
- functions
- records
- constructors
- `if`
- local inference
- explicit returns

Target:
- typecheck a small file with records and functions

## Phase 4: sum types and match
Implement:

- sum variants
- constructor typing
- pattern typing
- exhaustiveness checks

Target:
- typecheck `Option[T]`-style code

## Phase 5: typed IR + Lua codegen
Implement:

- lowering AST/resolved info to TIR
- basic Lua emission for records/functions/modules
- constructors and match emission

Target:
- emit runnable Lua for a few sample programs

## Phase 6: extern interop
Implement:

- `extern type`
- `extern fn`
- `extern module`
- foreign nullable restrictions

Target:
- typed bindings to Lua APIs

That is the right order. Do not start with codegen.

---

# 17. Rust-specific recommendations

## 17.1 Use small newtype IDs

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);
```

Do this everywhere. It pays off.

## 17.2 Avoid storing references into source text initially

Own strings at first.
You can optimize with interning later.

## 17.3 Add node IDs only when needed
You do not need every AST node to have an ID on day one.
But expressions and patterns will likely benefit from IDs once resolution/typechecking begin.

## 17.4 Keep parser recovery modest
Do basic error recovery:
- skip to newline
- skip to `end`
- skip to next top-level keyword

Do not overinvest in recovery too early.

## 17.5 Write snapshot tests
For compiler projects, snapshot tests are extremely effective.

Test:
- token streams
- parsed AST
- diagnostics
- emitted Lua

Rust crates like `insta` are useful here.

---

# 18. A lean first Rust type layout

If you want the shortest credible path, these are the most important types to define first:

```rust
Span
Diagnostic
TokenKind
Token
Module
TopDecl
TypeDecl
FuncDecl
Stmt
Expr
Pattern
Type
TypeId / FuncId / VariantId / LocalId
```

Once those are stable, the project becomes much easier.

---

# 19. What I would avoid in the implementation

Avoid these early mistakes:

- building codegen directly from raw AST
- mixing parsing and typechecking
- using strings everywhere instead of IDs after resolution starts
- representing locals only by names
- skipping spans
- trying to support multi-file import semantics too early in full generality
- adding fancy borrow-heavy arenas before the design stabilizes

Keep ownership boring at first.

---

# 20. Concrete recommendation

If you want the cleanest first milestone, implement these files first:

```txt
span.rs
diagnostics.rs
token.rs
lexer.rs
ast.rs
parser.rs
main.rs
```

and get this command working:

```txt
cargo run -- parse examples/test.luna
```

where it prints either:

- a readable AST, or
- syntax errors with spans

That is the correct first foothold.

After that, define:

```txt
types.rs
resolve.rs
typecheck.rs
tir.rs
codegen_lua.rs
```

in that order.

---

# 21. My recommended minimal AST/TIR boundary

One final design point: do not overcomplicate the AST/TIR split.

Use:

- **AST** for parsed syntax
- **TIR** for resolved + typed + desugared code

That is enough.

You do not need:
- CST
- AST
- HIR
- MIR
- LIR

for v0.1.

Two layers is the right size.
