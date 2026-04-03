use crate::ast::Visibility;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VariantId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeParamId(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Unit,

    Named(TypeId, Vec<Type>),
    Func(Vec<Type>, Box<Type>),

    TypeParam(TypeParamId),

    ForeignNil,
    ForeignNullable(Box<Type>),
    Error,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub vis: Visibility,
    pub params: Vec<TypeParamId>,
    pub kind: TypeKind,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Alias(Type),
    Record(Vec<FieldInfo>),
    Sum(Vec<VariantInfo>),
    ExternOpaque,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct VariantInfo {
    pub id: VariantId,
    pub name: String,
    pub payload: VariantPayload,
}

#[derive(Debug, Clone)]
pub enum VariantPayload {
    None,
    Positional(Vec<Type>),
    Record(Vec<FieldInfo>),
}

#[derive(Debug, Clone)]
pub struct FuncInfo {
    pub name: String,
    pub vis: Visibility,
    pub type_params: Vec<TypeParamId>,
    pub params: Vec<Type>,
    pub ret: Type,
    pub kind: FuncKind,
}

#[derive(Debug, Clone)]
pub enum FuncKind {
    Normal,
    Extern,
    Method { self_type: TypeId },
}

impl Type {
    pub fn is_assignable_from(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Error, _) | (_, Type::Error) => true,
            (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool)
            | (Type::String, Type::String)
            | (Type::Unit, Type::Unit)
            | (Type::ForeignNil, Type::ForeignNil) => true,
            (Type::Named(lhs_id, lhs_args), Type::Named(rhs_id, rhs_args)) => {
                lhs_id == rhs_id
                    && lhs_args.len() == rhs_args.len()
                    && lhs_args
                        .iter()
                        .zip(rhs_args)
                        .all(|(lhs, rhs)| lhs.is_assignable_from(rhs))
            }
            (Type::Func(lhs_p, lhs_r), Type::Func(rhs_p, rhs_r)) => {
                lhs_p.len() == rhs_p.len()
                    && lhs_p
                        .iter()
                        .zip(rhs_p)
                        .all(|(lhs, rhs)| lhs.is_assignable_from(rhs))
                    && lhs_r.is_assignable_from(rhs_r)
            }
            (Type::TypeParam(a), Type::TypeParam(b)) => a == b,
            (Type::ForeignNullable(_), Type::ForeignNil) => true,
            (Type::ForeignNullable(lhs), Type::ForeignNullable(rhs)) => lhs.is_assignable_from(rhs),
            _ => false,
        }
    }
}
