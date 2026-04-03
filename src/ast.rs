use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatternId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeExprId(pub u32);

#[derive(Debug, Clone)]
pub struct Module {
    pub module_decl: Option<ModuleDecl>,
    pub imports: Vec<ImportDecl>,
    pub decls: Vec<TopDecl>,
}

#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub span: Span,
    pub path: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub span: Span,
    pub path: Vec<String>,
    pub items: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum TopDecl {
    Type(TypeDecl),
    Func(FuncDecl),
    ExternType(ExternTypeDecl),
    ExternFunc(ExternFuncDecl),
    ExternModule(ExternModuleDecl),
    Impl(ImplDecl),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Public,
}

#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub span: Span,
    pub vis: Visibility,
    pub name: String,
    pub type_params: Vec<String>,
    pub body: TypeDeclBody,
}

#[derive(Debug, Clone)]
pub enum TypeDeclBody {
    Alias(TypeExpr),
    Record(Vec<RecordFieldType>),
    Sum(Vec<SumVariantDecl>),
}

#[derive(Debug, Clone)]
pub struct RecordFieldType {
    pub span: Span,
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct SumVariantDecl {
    pub span: Span,
    pub name: String,
    pub payload: SumVariantPayload,
}

#[derive(Debug, Clone)]
pub enum SumVariantPayload {
    None,
    Positional(Vec<TypeExpr>),
    Record(Vec<RecordFieldType>),
}

#[derive(Debug, Clone)]
pub struct TypeExpr {
    pub id: TypeExprId,
    pub span: Span,
    pub kind: TypeExprKind,
}

#[derive(Debug, Clone)]
pub enum TypeExprKind {
    Named {
        name: String,
        args: Vec<TypeExpr>,
    },
    Func {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },
    Nullable {
        inner: Box<TypeExpr>,
    },
    Nil,
    Unit,
}

#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub span: Span,
    pub vis: Visibility,
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub ret_ty: TypeExpr,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub span: Span,
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct ExternFuncDecl {
    pub span: Span,
    pub vis: Visibility,
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct ExternTypeDecl {
    pub span: Span,
    pub vis: Visibility,
    pub name: String,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExternModuleDecl {
    pub span: Span,
    pub vis: Visibility,
    pub path: Vec<String>,
    pub funcs: Vec<ExternFuncDecl>,
}

#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub span: Span,
    pub target: String,
    pub methods: Vec<FuncDecl>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub span: Span,
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(LetStmt),
    Var(VarStmt),
    Assign(AssignStmt),
    Expr(ExprStmt),
    Return(ReturnStmt),
    While(WhileStmt),
    For(ForStmt),
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub span: Span,
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct VarStmt {
    pub span: Span,
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub span: Span,
    pub target: String,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub span: Span,
    pub expr: Expr,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub span: Span,
    pub value: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub span: Span,
    pub cond: Expr,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub span: Span,
    pub name: String,
    pub start: Expr,
    pub end: Expr,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub id: ExprId,
    pub span: Span,
    pub kind: ExprKind,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Unit,

    Var(String),
    Path(Vec<String>),

    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Field {
        receiver: Box<Expr>,
        name: String,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },

    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },

    If {
        branches: Vec<(Expr, Block)>,
        else_branch: Box<Block>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },

    RecordInit {
        type_name: String,
        fields: Vec<RecordFieldInit>,
    },
    RecordUpdate {
        base: Box<Expr>,
        fields: Vec<RecordFieldInit>,
    },
    Constructor {
        name: String,
        payload: ConstructorPayload,
    },

    Lambda {
        params: Vec<Param>,
        ret_ty: TypeExpr,
        body: Box<Expr>,
    },

    Paren(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct MatchExpr {
    pub span: Span,
    pub scrutinee: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub span: Span,
    pub pattern: Pattern,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct RecordFieldInit {
    pub span: Span,
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub enum ConstructorPayload {
    None,
    Positional(Vec<Expr>),
    Record(Vec<RecordFieldInit>),
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub id: PatternId,
    pub span: Span,
    pub kind: PatternKind,
}

#[derive(Debug, Clone)]
pub enum PatternKind {
    Wildcard,
    Bind {
        name: String,
    },
    Int {
        value: i64,
    },
    Bool {
        value: bool,
    },
    String {
        value: String,
    },
    Constructor {
        name: String,
        args: Vec<Pattern>,
    },
    RecordConstructor {
        name: String,
        fields: Vec<RecordPatternField>,
    },
}

#[derive(Debug, Clone)]
pub struct RecordPatternField {
    pub span: Span,
    pub name: String,
    pub pattern: Option<Pattern>,
}
