use std::collections::HashMap;

use crate::{
    ast::{
        ExternFuncDecl, FuncDecl, ImplDecl, Module, SumVariantPayload as AstSumVariantPayload,
        TopDecl, TypeDeclBody, TypeExpr, TypeExprKind,
    },
    diagnostics::Diagnostics,
    types::{
        FieldInfo, FuncId, FuncInfo, FuncKind, Type, TypeId, TypeInfo, TypeKind, TypeParamId,
        VariantId, VariantInfo, VariantPayload,
    },
};

#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub type_infos: Vec<TypeInfo>,
    pub func_infos: Vec<FuncInfo>,
    pub type_names: HashMap<String, TypeId>,
    pub func_names: HashMap<String, FuncId>,
    pub variant_names: HashMap<String, VariantId>,
    pub variant_to_type: HashMap<VariantId, TypeId>,
    pub method_names: HashMap<(TypeId, String), FuncId>,
    pub import_modules: HashMap<String, Vec<String>>,
    pub import_items: HashMap<String, String>,
    pub bodies: Vec<ResolvedBody>,
}

#[derive(Debug, Clone)]
pub struct ResolvedBody {
    pub func_id: FuncId,
    pub decl: FuncDecl,
}

#[derive(Default)]
struct Ctx {
    diagnostics: Diagnostics,
    type_infos: Vec<TypeInfo>,
    func_infos: Vec<FuncInfo>,
    type_names: HashMap<String, TypeId>,
    func_names: HashMap<String, FuncId>,
    variant_names: HashMap<String, VariantId>,
    variant_to_type: HashMap<VariantId, TypeId>,
    method_names: HashMap<(TypeId, String), FuncId>,
    import_modules: HashMap<String, Vec<String>>,
    import_items: HashMap<String, String>,
    next_type_id: u32,
    next_func_id: u32,
    next_variant_id: u32,
    next_type_param_id: u32,
    pending_types: Vec<(TypeId, crate::ast::TypeDecl)>,
    pending_funcs: Vec<(FuncId, FuncDecl, bool)>, // bool = extern context for type parsing
    pending_extern_funcs: Vec<(FuncId, ExternFuncDecl, bool)>,
    pending_impls: Vec<ImplDecl>,
    bodies: Vec<ResolvedBody>,
}

pub fn resolve(module: &Module) -> (ResolvedModule, Diagnostics) {
    let mut ctx = Ctx::default();
    ctx.collect_declarations(module);
    ctx.resolve_type_decls();
    ctx.resolve_functions();

    let resolved = ResolvedModule {
        type_infos: ctx.type_infos,
        func_infos: ctx.func_infos,
        type_names: ctx.type_names,
        func_names: ctx.func_names,
        variant_names: ctx.variant_names,
        variant_to_type: ctx.variant_to_type,
        method_names: ctx.method_names,
        import_modules: ctx.import_modules,
        import_items: ctx.import_items,
        bodies: ctx.bodies,
    };

    (resolved, ctx.diagnostics)
}

impl Ctx {
    fn collect_declarations(&mut self, module: &Module) {
        self.collect_imports(module);
        for decl in &module.decls {
            match decl {
                TopDecl::Type(type_decl) => {
                    let type_id = self.alloc_type_id();
                    if self
                        .type_names
                        .insert(type_decl.name.clone(), type_id)
                        .is_some()
                    {
                        self.diagnostics.error(
                            type_decl.span,
                            format!("duplicate type declaration '{}'", type_decl.name),
                        );
                    }
                    self.type_infos.push(TypeInfo {
                        name: type_decl.name.clone(),
                        vis: type_decl.vis,
                        params: Vec::new(),
                        kind: TypeKind::Alias(Type::Error),
                    });
                    self.pending_types.push((type_id, type_decl.clone()));
                }
                TopDecl::ExternType(extern_type) => {
                    let type_id = self.alloc_type_id();
                    if self
                        .type_names
                        .insert(extern_type.name.clone(), type_id)
                        .is_some()
                    {
                        self.diagnostics.error(
                            extern_type.span,
                            format!("duplicate type declaration '{}'", extern_type.name),
                        );
                    }
                    self.type_infos.push(TypeInfo {
                        name: extern_type.name.clone(),
                        vis: extern_type.vis,
                        params: Vec::new(),
                        kind: TypeKind::ExternOpaque,
                    });
                }
                TopDecl::Func(func_decl) => {
                    let func_id = self.alloc_func_id();
                    if self
                        .func_names
                        .insert(func_decl.name.clone(), func_id)
                        .is_some()
                    {
                        self.diagnostics.error(
                            func_decl.span,
                            format!("duplicate function declaration '{}'", func_decl.name),
                        );
                    }
                    self.func_infos.push(FuncInfo {
                        name: func_decl.name.clone(),
                        vis: func_decl.vis,
                        type_params: Vec::new(),
                        params: Vec::new(),
                        ret: Type::Error,
                        kind: FuncKind::Normal,
                    });
                    self.pending_funcs.push((func_id, func_decl.clone(), false));
                }
                TopDecl::ExternFunc(extern_func) => {
                    let func_id = self.alloc_func_id();
                    if self
                        .func_names
                        .insert(extern_func.name.clone(), func_id)
                        .is_some()
                    {
                        self.diagnostics.error(
                            extern_func.span,
                            format!("duplicate function declaration '{}'", extern_func.name),
                        );
                    }
                    self.func_infos.push(FuncInfo {
                        name: extern_func.name.clone(),
                        vis: extern_func.vis,
                        type_params: Vec::new(),
                        params: Vec::new(),
                        ret: Type::Error,
                        kind: FuncKind::Extern,
                    });
                    self.pending_extern_funcs
                        .push((func_id, extern_func.clone(), true));
                }
                TopDecl::ExternModule(extern_module) => {
                    for func in &extern_module.funcs {
                        let full_name = format!("{}.{}", extern_module.path.join("."), func.name);
                        let func_id = self.alloc_func_id();
                        if self.func_names.insert(full_name.clone(), func_id).is_some() {
                            self.diagnostics.error(
                                func.span,
                                format!("duplicate extern function declaration '{}'", full_name),
                            );
                        }
                        self.func_infos.push(FuncInfo {
                            name: full_name,
                            vis: func.vis,
                            type_params: Vec::new(),
                            params: Vec::new(),
                            ret: Type::Error,
                            kind: FuncKind::Extern,
                        });
                        self.pending_extern_funcs
                            .push((func_id, func.clone(), true));
                    }
                }
                TopDecl::Impl(impl_decl) => {
                    self.pending_impls.push(impl_decl.clone());
                }
            }
        }

        self.collect_impl_methods();
    }

    fn collect_imports(&mut self, module: &Module) {
        for import in &module.imports {
            let Some(alias) = import.path.last().cloned() else {
                continue;
            };

            if let Some(existing) = self
                .import_modules
                .insert(alias.clone(), import.path.clone())
            {
                if existing != import.path {
                    self.diagnostics
                        .error(import.span, format!("duplicate import alias '{}'", alias));
                }
            }

            if let Some(items) = &import.items {
                for item in items {
                    let qualified = format!("{}.{}", import.path.join("."), item);
                    if let Some(existing) =
                        self.import_items.insert(item.clone(), qualified.clone())
                    {
                        if existing != qualified {
                            self.diagnostics
                                .error(import.span, format!("duplicate imported item '{}'", item));
                        }
                    }
                }
            }
        }
    }

    fn collect_impl_methods(&mut self) {
        let pending_impls = self.pending_impls.clone();
        for impl_decl in pending_impls {
            let Some(type_id) = self.type_names.get(&impl_decl.target).copied() else {
                self.diagnostics.error(
                    impl_decl.span,
                    format!("unknown impl target type '{}'", impl_decl.target),
                );
                continue;
            };

            for method in impl_decl.methods {
                let func_id = self.alloc_func_id();
                let lowered_name = format!("{}_{}", impl_decl.target, method.name);
                if self
                    .func_names
                    .insert(lowered_name.clone(), func_id)
                    .is_some()
                {
                    self.diagnostics
                        .error(method.span, format!("duplicate method '{}'", lowered_name));
                }
                if self
                    .method_names
                    .insert((type_id, method.name.clone()), func_id)
                    .is_some()
                {
                    self.diagnostics.error(
                        method.span,
                        format!(
                            "duplicate method '{}' for type '{}'",
                            method.name, impl_decl.target
                        ),
                    );
                }
                self.func_infos.push(FuncInfo {
                    name: lowered_name,
                    vis: method.vis,
                    type_params: Vec::new(),
                    params: Vec::new(),
                    ret: Type::Error,
                    kind: FuncKind::Method { self_type: type_id },
                });
                self.pending_funcs.push((func_id, method, false));
            }
        }
    }

    fn resolve_type_decls(&mut self) {
        let pending_types = self.pending_types.clone();
        for (type_id, type_decl) in pending_types {
            let mut local_type_params = HashMap::new();
            let mut params = Vec::new();
            for param in &type_decl.type_params {
                let id = self.alloc_type_param_id();
                local_type_params.insert(param.clone(), id);
                params.push(id);
            }

            let kind = match &type_decl.body {
                TypeDeclBody::Alias(expr) => {
                    TypeKind::Alias(self.resolve_type_expr(expr, &local_type_params, false))
                }
                TypeDeclBody::Record(fields) => TypeKind::Record(
                    fields
                        .iter()
                        .map(|field| FieldInfo {
                            name: field.name.clone(),
                            ty: self.resolve_type_expr(&field.ty, &local_type_params, false),
                        })
                        .collect(),
                ),
                TypeDeclBody::Sum(variants) => {
                    let mut out = Vec::new();
                    for variant in variants {
                        let variant_id = self.alloc_variant_id();
                        if self
                            .variant_names
                            .insert(variant.name.clone(), variant_id)
                            .is_some()
                        {
                            self.diagnostics.error(
                                variant.span,
                                format!("duplicate variant '{}'", variant.name),
                            );
                        }
                        self.variant_to_type.insert(variant_id, type_id);
                        let payload = match &variant.payload {
                            AstSumVariantPayload::None => VariantPayload::None,
                            AstSumVariantPayload::Positional(tys) => VariantPayload::Positional(
                                tys.iter()
                                    .map(|ty| self.resolve_type_expr(ty, &local_type_params, false))
                                    .collect(),
                            ),
                            AstSumVariantPayload::Record(fields) => VariantPayload::Record(
                                fields
                                    .iter()
                                    .map(|field| FieldInfo {
                                        name: field.name.clone(),
                                        ty: self.resolve_type_expr(
                                            &field.ty,
                                            &local_type_params,
                                            false,
                                        ),
                                    })
                                    .collect(),
                            ),
                        };
                        out.push(VariantInfo {
                            id: variant_id,
                            name: variant.name.clone(),
                            payload,
                        });
                    }
                    TypeKind::Sum(out)
                }
            };

            if let Some(info) = self.type_infos.get_mut(type_id.0 as usize) {
                info.params = params;
                info.kind = kind;
            }
        }
    }

    fn resolve_functions(&mut self) {
        let pending_funcs = self.pending_funcs.clone();
        for (func_id, func_decl, extern_ctx) in pending_funcs {
            let mut local_type_params = HashMap::new();
            let mut type_params = Vec::new();
            for name in &func_decl.type_params {
                let id = self.alloc_type_param_id();
                local_type_params.insert(name.clone(), id);
                type_params.push(id);
            }

            let params: Vec<Type> = func_decl
                .params
                .iter()
                .map(|p| self.resolve_type_expr(&p.ty, &local_type_params, extern_ctx))
                .collect();
            let ret = self.resolve_type_expr(&func_decl.ret_ty, &local_type_params, extern_ctx);

            if let Some(info) = self.func_infos.get_mut(func_id.0 as usize) {
                info.type_params = type_params;
                info.params = params;
                info.ret = ret;
            }

            match self.func_infos.get(func_id.0 as usize).map(|f| &f.kind) {
                Some(FuncKind::Extern) => {}
                _ => {
                    self.bodies.push(ResolvedBody {
                        func_id,
                        decl: func_decl,
                    });
                }
            }
        }

        let pending_extern_funcs = self.pending_extern_funcs.clone();
        for (func_id, extern_func, extern_ctx) in pending_extern_funcs {
            let local_type_params = HashMap::new();
            let params: Vec<Type> = extern_func
                .params
                .iter()
                .map(|p| self.resolve_type_expr(&p.ty, &local_type_params, extern_ctx))
                .collect();
            let ret = self.resolve_type_expr(&extern_func.ret_ty, &local_type_params, extern_ctx);

            if let Some(info) = self.func_infos.get_mut(func_id.0 as usize) {
                info.params = params;
                info.ret = ret;
            }
        }
    }

    fn resolve_type_expr(
        &mut self,
        expr: &TypeExpr,
        type_params: &HashMap<String, TypeParamId>,
        extern_ctx: bool,
    ) -> Type {
        match &expr.kind {
            TypeExprKind::Named { name, args } => {
                if let Some(param) = type_params.get(name) {
                    return Type::TypeParam(*param);
                }
                if let Some(ty) = builtin_type(name) {
                    if !args.is_empty() {
                        self.diagnostics.error(
                            expr.span,
                            format!("builtin type '{}' does not take type arguments", name),
                        );
                    }
                    return ty;
                }
                let Some(type_id) = self.type_names.get(name).copied() else {
                    self.diagnostics
                        .error(expr.span, format!("unknown type '{}'", name));
                    return Type::Error;
                };
                let args = args
                    .iter()
                    .map(|arg| self.resolve_type_expr(arg, type_params, extern_ctx))
                    .collect();
                Type::Named(type_id, args)
            }
            TypeExprKind::Func { params, ret } => {
                let params = params
                    .iter()
                    .map(|p| self.resolve_type_expr(p, type_params, extern_ctx))
                    .collect();
                let ret = self.resolve_type_expr(ret, type_params, extern_ctx);
                Type::Func(params, Box::new(ret))
            }
            TypeExprKind::Nullable { inner } => {
                if !extern_ctx {
                    self.diagnostics.error(
                        expr.span,
                        "nullable types are only allowed in extern contexts",
                    );
                }
                let inner = self.resolve_type_expr(inner, type_params, true);
                Type::ForeignNullable(Box::new(inner))
            }
            TypeExprKind::Nil => {
                if !extern_ctx {
                    self.diagnostics
                        .error(expr.span, "nil type is only allowed in extern contexts");
                }
                Type::ForeignNil
            }
            TypeExprKind::Unit => Type::Unit,
        }
    }

    fn alloc_type_id(&mut self) -> TypeId {
        let id = TypeId(self.next_type_id);
        self.next_type_id += 1;
        id
    }

    fn alloc_func_id(&mut self) -> FuncId {
        let id = FuncId(self.next_func_id);
        self.next_func_id += 1;
        id
    }

    fn alloc_variant_id(&mut self) -> VariantId {
        let id = VariantId(self.next_variant_id);
        self.next_variant_id += 1;
        id
    }

    fn alloc_type_param_id(&mut self) -> TypeParamId {
        let id = TypeParamId(self.next_type_param_id);
        self.next_type_param_id += 1;
        id
    }
}

fn builtin_type(name: &str) -> Option<Type> {
    Some(match name {
        "Int" => Type::Int,
        "Float" => Type::Float,
        "Bool" => Type::Bool,
        "String" => Type::String,
        "Unit" => Type::Unit,
        _ => return None,
    })
}
