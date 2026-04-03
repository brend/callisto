use crate::types::{FuncId, LocalId, Type, TypeId, VariantId};

#[derive(Debug, Clone)]
pub struct TirModule {
    pub types: Vec<TirTypeDecl>,
    pub funcs: Vec<TirFunc>,
}

#[derive(Debug, Clone)]
pub struct TirTypeDecl {
    pub id: TypeId,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TirFunc {
    pub id: FuncId,
    pub name: String,
    pub params: Vec<TirParam>,
    pub ret_ty: Type,
    pub body: TirBlock,
    pub kind: TirFuncKind,
}

#[derive(Debug, Clone)]
pub struct TirParam {
    pub local: LocalId,
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum TirFuncKind {
    Normal,
    Extern,
    Method { self_type: TypeId },
}

#[derive(Debug, Clone)]
pub struct TirBlock {
    pub stmts: Vec<TirStmt>,
    pub tail: Option<Box<TirExpr>>,
}

#[derive(Debug, Clone)]
pub enum TirStmt {
    Let {
        local: LocalId,
        ty: Type,
        value: TirExpr,
        mutable: bool,
    },
    Assign {
        local: LocalId,
        value: TirExpr,
    },
    Expr(TirExpr),
    Return(Option<TirExpr>),
    While {
        cond: TirExpr,
        body: TirBlock,
    },
    ForRange {
        local: LocalId,
        start: TirExpr,
        end: TirExpr,
        body: TirBlock,
    },
}

#[derive(Debug, Clone)]
pub struct TirExpr {
    pub ty: Type,
    pub kind: TirExprKind,
}

#[derive(Debug, Clone)]
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
        else_branch: Box<TirBlock>,
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
        type_id: Option<TypeId>,
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

#[derive(Debug, Clone)]
pub struct TirMatchArm {
    pub pattern: TirPattern,
    pub body: TirBlock,
}

#[derive(Debug, Clone)]
pub enum TirPattern {
    Wildcard,
    Bind(LocalId),
    Int(i64),
    Bool(bool),
    String(String),
    Variant {
        variant_id: Option<VariantId>,
        payload: TirPatternVariantPayload,
    },
}

#[derive(Debug, Clone)]
pub enum TirPatternVariantPayload {
    None,
    Positional(Vec<TirPattern>),
    Record(Vec<(String, TirPattern)>),
}

#[derive(Debug, Clone)]
pub enum TirVariantPayload {
    None,
    Positional(Vec<TirExpr>),
    Record(Vec<(String, TirExpr)>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TirBinaryOp {
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
pub enum TirUnaryOp {
    Neg,
    Not,
}
