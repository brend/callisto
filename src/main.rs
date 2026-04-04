#![allow(dead_code)]

mod ast;
mod cli;
mod codegen_lua;
mod diagnostics;
mod interner;
mod lexer;
mod parser;
mod resolve;
mod source;
mod span;
mod tir;
mod token;
mod typecheck;
mod types;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
};

use cli::{Cli, Command};
use diagnostics::Diagnostics;
use source::SourceDb;
use span::FileId;

fn main() {
    match run() {
        Ok(()) => {}
        Err(code) => std::process::exit(code),
    }
}

fn run() -> Result<(), i32> {
    let cli = match Cli::parse_from_env() {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("{}", message);
            return Err(2);
        }
    };

    match cli.command {
        Command::Parse { input } => parse_command(&input),
        Command::Check { input } => check_command(&input),
        Command::EmitLua { input, output } => emit_lua_command(&input, output.as_deref()),
        Command::Build { input, output } => build_command(&input, output.as_deref()),
    }
}

#[derive(Debug, Clone)]
struct ParsedModule {
    file_id: FileId,
    source_path: PathBuf,
    module_path: Vec<String>,
    ast: ast::Module,
    is_entry: bool,
}

#[derive(Debug, Clone)]
struct CompiledModule {
    parsed: ParsedModule,
    resolved: resolve::ResolvedModule,
    tir: tir::TirModule,
}

#[derive(Debug, Clone)]
struct CompiledProject {
    modules: Vec<CompiledModule>,
    entry_index: usize,
}

fn parse_command(input: &Path) -> Result<(), i32> {
    let (sources, ast, diagnostics, _) = compile_pipeline(input)?;
    println!("{:#?}", ast);

    if !diagnostics.is_empty() {
        eprint!("{}", diagnostics.render(&sources));
        if diagnostics.has_errors() {
            return Err(1);
        }
    }

    Ok(())
}

fn check_command(input: &Path) -> Result<(), i32> {
    let (sources, _, diagnostics, _) = compile_pipeline(input)?;
    if !diagnostics.is_empty() {
        eprint!("{}", diagnostics.render(&sources));
    }
    if diagnostics.has_errors() {
        return Err(1);
    }
    println!("ok");
    Ok(())
}

fn emit_lua_command(input: &Path, output: Option<&Path>) -> Result<(), i32> {
    let (sources, entry_ast, diagnostics, compiled_project) = compile_project(input)?;
    if !diagnostics.is_empty() {
        eprint!("{}", diagnostics.render(&sources));
    }
    if diagnostics.has_errors() {
        return Err(1);
    }

    let project = compiled_project.expect("compiled output present");
    let entry = project
        .modules
        .get(project.entry_index)
        .expect("entry module present");

    if output.is_some_and(|path| path.extension().and_then(|e| e.to_str()) == Some("lua")) {
        let lua = codegen_lua::emit_lua_module(&entry.tir, &entry.resolved);
        let output_path = resolve_output_path(output, input, &entry_ast);
        write_lua_file(&output_path, &lua)?;
        println!("wrote {}", output_path.display());
        return Ok(());
    }

    let out_dir = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("out"));
    for module in &project.modules {
        let lua = codegen_lua::emit_lua_module(&module.tir, &module.resolved);
        let output_path = resolve_module_output_path(&out_dir, input, module);
        write_lua_file(&output_path, &lua)?;
        println!("wrote {}", output_path.display());
    }

    Ok(())
}

fn build_command(input: &Path, output: Option<&Path>) -> Result<(), i32> {
    emit_lua_command(input, output)
}

fn compile_project(
    input: &Path,
) -> Result<(SourceDb, ast::Module, Diagnostics, Option<CompiledProject>), i32> {
    let mut sources = SourceDb::new();
    let mut diagnostics = Diagnostics::new();

    let parsed_modules = load_module_graph(input, &mut sources, &mut diagnostics)?;
    let entry_ast = parsed_modules
        .iter()
        .find(|m| m.is_entry)
        .map(|m| m.ast.clone())
        .unwrap_or(ast::Module {
            module_decl: None,
            imports: Vec::new(),
            decls: Vec::new(),
        });

    let mut modules_by_path = HashMap::new();
    for (idx, module) in parsed_modules.iter().enumerate() {
        if module.module_path.is_empty() {
            continue;
        }
        modules_by_path.insert(module.module_path.join("."), idx);
    }

    let mut compiled_modules = Vec::new();
    for module in &parsed_modules {
        let ast_for_compile = synthesize_import_declarations(
            module,
            &parsed_modules,
            &modules_by_path,
            &mut diagnostics,
        );
        let (resolved, resolve_diags) = resolve::resolve(&ast_for_compile);
        diagnostics.extend(resolve_diags);
        let (tir, type_diags) = typecheck::typecheck_and_lower(&resolved);
        diagnostics.extend(type_diags);
        compiled_modules.push(CompiledModule {
            parsed: module.clone(),
            resolved,
            tir,
        });
    }

    let entry_index = compiled_modules
        .iter()
        .position(|m| m.parsed.is_entry)
        .unwrap_or(0);

    let compiled = if diagnostics.has_errors() {
        None
    } else {
        Some(CompiledProject {
            modules: compiled_modules,
            entry_index,
        })
    };

    Ok((sources, entry_ast, diagnostics, compiled))
}

fn compile_pipeline(
    input: &Path,
) -> Result<
    (
        SourceDb,
        ast::Module,
        Diagnostics,
        Option<(resolve::ResolvedModule, tir::TirModule)>,
    ),
    i32,
> {
    let (sources, entry_ast, diagnostics, project) = compile_project(input)?;
    let compiled = project.and_then(|project| {
        project
            .modules
            .into_iter()
            .nth(project.entry_index)
            .map(|entry| (entry.resolved, entry.tir))
    });
    Ok((sources, entry_ast, diagnostics, compiled))
}

fn load_module_graph(
    input: &Path,
    sources: &mut SourceDb,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<ParsedModule>, i32> {
    let root_dir = input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut queue: VecDeque<(PathBuf, Option<Vec<String>>, bool)> = VecDeque::new();
    queue.push_back((input.to_path_buf(), None, true));

    let mut seen_paths = HashSet::new();
    let mut module_to_path: HashMap<String, PathBuf> = HashMap::new();
    let mut modules = Vec::new();

    while let Some((path, expected_module_path, is_entry)) = queue.pop_front() {
        let path = normalize_path(path);
        if !seen_paths.insert(path.clone()) {
            continue;
        }

        let file_id = match sources.load_file(&path) {
            Ok(id) => id,
            Err(err) => {
                if is_entry {
                    eprintln!("failed to read '{}': {}", path.display(), err);
                    return Err(1);
                }
                diagnostics.error(
                    span::Span::dummy(),
                    format!("failed to read module file '{}': {}", path.display(), err),
                );
                continue;
            }
        };
        let source = sources.get(file_id).expect("source exists");
        let (tokens, lex_diags) = lexer::lex(file_id, &source.text);
        diagnostics.extend(lex_diags);
        let (ast, parse_diags) = parser::parse(tokens);
        diagnostics.extend(parse_diags);

        let module_path = match (&ast.module_decl, &expected_module_path) {
            (Some(decl), Some(expected)) => {
                if decl.path != *expected {
                    diagnostics.error(
                        decl.span,
                        format!(
                            "module declaration '{}' does not match imported path '{}'",
                            decl.path.join("."),
                            expected.join(".")
                        ),
                    );
                }
                decl.path.clone()
            }
            (Some(decl), None) => decl.path.clone(),
            (None, Some(expected)) => expected.clone(),
            (None, None) => path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| vec![s.to_string()])
                .unwrap_or_default(),
        };

        if !module_path.is_empty() {
            let key = module_path.join(".");
            if let Some(existing) = module_to_path.insert(key.clone(), path.clone()) {
                if existing != path {
                    diagnostics.error(
                        span::Span::new(file_id, 0, 0),
                        format!(
                            "module '{}' is defined by multiple files: '{}' and '{}'",
                            key,
                            existing.display(),
                            path.display()
                        ),
                    );
                }
            }
        }

        let explicit_extern_paths: HashSet<String> = ast
            .decls
            .iter()
            .filter_map(|decl| match decl {
                ast::TopDecl::ExternModule(extern_module) => Some(extern_module.path.join(".")),
                _ => None,
            })
            .collect();

        for import in &ast.imports {
            let import_key = import.path.join(".");
            if explicit_extern_paths.contains(&import_key) {
                continue;
            }
            match find_module_file(&root_dir, &import.path) {
                Some(import_path) => {
                    queue.push_back((import_path, Some(import.path.clone()), false));
                }
                None => diagnostics.error(
                    import.span,
                    format!(
                        "could not find module file for import '{}'",
                        import.path.join(".")
                    ),
                ),
            }
        }

        modules.push(ParsedModule {
            file_id,
            source_path: path,
            module_path,
            ast,
            is_entry,
        });
    }

    Ok(modules)
}

fn synthesize_import_declarations(
    module: &ParsedModule,
    all_modules: &[ParsedModule],
    modules_by_path: &HashMap<String, usize>,
    diagnostics: &mut Diagnostics,
) -> ast::Module {
    let mut ast = module.ast.clone();
    let explicit_extern_paths: HashSet<String> = ast
        .decls
        .iter()
        .filter_map(|decl| match decl {
            ast::TopDecl::ExternModule(extern_module) => Some(extern_module.path.join(".")),
            _ => None,
        })
        .collect();
    let mut known_type_names: HashSet<String> = ast
        .decls
        .iter()
        .filter_map(|decl| match decl {
            ast::TopDecl::Type(type_decl) => Some(type_decl.name.clone()),
            ast::TopDecl::ExternType(type_decl) => Some(type_decl.name.clone()),
            _ => None,
        })
        .collect();

    for import in &module.ast.imports {
        let key = import.path.join(".");
        let Some(imported_idx) = modules_by_path.get(&key).copied() else {
            continue;
        };
        let imported = &all_modules[imported_idx];

        if !explicit_extern_paths.contains(&key) {
            let mut funcs = Vec::new();
            for decl in &imported.ast.decls {
                match decl {
                    ast::TopDecl::Func(func_decl)
                        if matches!(func_decl.vis, ast::Visibility::Public) =>
                    {
                        funcs.push(ast::ExternFuncDecl {
                            span: func_decl.span,
                            vis: ast::Visibility::Private,
                            name: func_decl.name.clone(),
                            params: func_decl.params.clone(),
                            ret_ty: func_decl.ret_ty.clone(),
                        });
                    }
                    ast::TopDecl::ExternFunc(func_decl)
                        if matches!(func_decl.vis, ast::Visibility::Public) =>
                    {
                        let mut extern_func = func_decl.clone();
                        extern_func.vis = ast::Visibility::Private;
                        funcs.push(extern_func);
                    }
                    _ => {}
                }
            }
            if !funcs.is_empty() {
                ast.decls
                    .push(ast::TopDecl::ExternModule(ast::ExternModuleDecl {
                        span: import.span,
                        vis: ast::Visibility::Private,
                        path: import.path.clone(),
                        funcs,
                    }));
            }
        }

        for decl in &imported.ast.decls {
            let extern_type = match decl {
                ast::TopDecl::Type(type_decl)
                    if matches!(type_decl.vis, ast::Visibility::Public) =>
                {
                    Some(ast::ExternTypeDecl {
                        span: type_decl.span,
                        vis: ast::Visibility::Private,
                        name: type_decl.name.clone(),
                        type_params: type_decl.type_params.clone(),
                    })
                }
                ast::TopDecl::ExternType(type_decl)
                    if matches!(type_decl.vis, ast::Visibility::Public) =>
                {
                    let mut ty = type_decl.clone();
                    ty.vis = ast::Visibility::Private;
                    Some(ty)
                }
                _ => None,
            };
            if let Some(extern_type) = extern_type {
                if known_type_names.insert(extern_type.name.clone()) {
                    ast.decls.push(ast::TopDecl::ExternType(extern_type));
                }
            }
        }

        if !imported.is_entry && imported.ast.module_decl.is_none() {
            diagnostics.warning(
                span::Span::new(module.file_id, import.span.start, import.span.end),
                format!(
                    "imported module '{}' has no explicit 'module' declaration",
                    key
                ),
            );
        }
    }

    ast
}

fn resolve_output_path(output: Option<&Path>, input: &Path, ast: &ast::Module) -> PathBuf {
    let stem = ast
        .module_decl
        .as_ref()
        .and_then(|m| m.path.last())
        .cloned()
        .or_else(|| {
            input
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "main".to_string());

    match output {
        Some(path) if path.extension().and_then(|e| e.to_str()) == Some("lua") => {
            path.to_path_buf()
        }
        Some(path) => path.join(format!("{}.lua", stem)),
        None => PathBuf::from("out").join(format!("{}.lua", stem)),
    }
}

fn resolve_module_output_path(out_dir: &Path, input: &Path, module: &CompiledModule) -> PathBuf {
    if !module.parsed.module_path.is_empty() {
        let mut path = out_dir.to_path_buf();
        if module.parsed.module_path.len() > 1 {
            for segment in &module.parsed.module_path[..module.parsed.module_path.len() - 1] {
                path.push(segment);
            }
        }
        let file_name = format!(
            "{}.lua",
            module
                .parsed
                .module_path
                .last()
                .cloned()
                .unwrap_or_default()
        );
        path.push(file_name);
        return path;
    }

    let stem = module
        .parsed
        .ast
        .module_decl
        .as_ref()
        .and_then(|m| m.path.last())
        .cloned()
        .or_else(|| {
            module
                .parsed
                .source_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            input
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "main".to_string());
    out_dir.join(format!("{}.lua", stem))
}

fn write_lua_file(path: &Path, lua: &str) -> Result<(), i32> {
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            eprintln!(
                "failed to create output directory '{}': {}",
                parent.display(),
                err
            );
            return Err(1);
        }
    }
    if let Err(err) = std::fs::write(path, lua) {
        eprintln!("failed to write '{}': {}", path.display(), err);
        return Err(1);
    }
    Ok(())
}

fn normalize_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn find_module_file(root_dir: &Path, module_path: &[String]) -> Option<PathBuf> {
    if module_path.is_empty() {
        return None;
    }

    let relative = module_path.iter().fold(PathBuf::new(), |mut acc, segment| {
        acc.push(segment);
        acc
    });

    let file_candidates = ["luna", "cal"].into_iter().map(|ext| {
        let mut path = root_dir.join(&relative);
        path.set_extension(ext);
        path
    });
    for candidate in file_candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let mod_candidates = ["mod.luna", "mod.cal"]
        .into_iter()
        .map(|name| root_dir.join(&relative).join(name));
    for candidate in mod_candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{codegen_lua, lexer, parser, resolve, typecheck};

    use super::{compile_pipeline, emit_lua_command, resolve_output_path};

    #[test]
    fn full_pipeline_compiles_and_emits_lua_for_feature_rich_module() {
        let source = r#"
type Point { x: Int, y: Int }
type MaybeInt = | None | Some(Int)

impl MaybeInt do
fn unwrap_or(self: MaybeInt, fallback: Int) -> Int do
match self do
case Some(v) => v
case None => fallback
end
end
end

fn add(a: Int, b: Int) -> Int do
a + b
end

pub fn main() -> Int do
let p = Point { x = 1, y = 2 }
var total: Int = add(p.x, p.y)
if true then
total = total + 1
else
total = total + 2
end
for i in 0..1 do
total = total + i
end
let inc = fn (x: Int) -> Int => x + 1
let m = Some(inc(total))
m.unwrap_or(0)
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);

        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);

        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);

        let (tir, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);

        let lua = codegen_lua::emit_lua_module(&tir, &resolved);
        assert!(lua.contains("local main"), "{lua}");
        assert!(lua.contains("main = function("), "{lua}");
        assert!(lua.contains("M.main = main"), "{lua}");
        assert!(lua.contains("MaybeInt_unwrap_or"), "{lua}");
    }

    #[test]
    fn typecheck_reports_assignment_to_immutable_parameter() {
        let source = r#"
fn bad(x: Int) -> Int do
x = 2
x
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors());

        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors());

        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors());

        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(
            type_diags
                .items
                .iter()
                .any(|d| d.message.contains("cannot assign to immutable local 'x'"))
        );
    }

    #[test]
    fn compile_pipeline_loads_file_and_missing_file_errors() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("callisto_compile_pipeline_{}.luna", nonce));
        std::fs::write(&path, "fn ok() -> Int do\n1\nend\n").expect("failed to write temp file");

        let (_, _, diagnostics, compiled) = compile_pipeline(&path).expect("pipeline failed");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.items);
        assert!(compiled.is_some());

        let missing = path.with_extension("missing.luna");
        assert_eq!(compile_pipeline(&missing).unwrap_err(), 1);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn compile_pipeline_omits_compiled_output_when_diagnostics_have_errors() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("callisto_compile_errors_{}.luna", nonce));
        std::fs::write(&path, "fn main() -> Int do\ntrue\nend\n")
            .expect("failed to write temp file");

        let (_, _, diagnostics, compiled) = compile_pipeline(&path).expect("pipeline failed");
        assert!(diagnostics.has_errors(), "{:?}", diagnostics.items);
        assert!(compiled.is_none());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn resolve_output_path_uses_lua_file_dir_and_defaults() {
        let (tokens, lex_diags) = lexer::lex(0, "module alpha.beta\n");
        assert!(!lex_diags.has_errors());
        let (ast_with_module, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors());

        let input = Path::new("src/input.luna");
        assert_eq!(
            resolve_output_path(Some(Path::new("build/custom.lua")), input, &ast_with_module),
            PathBuf::from("build/custom.lua")
        );
        assert_eq!(
            resolve_output_path(Some(Path::new("build")), input, &ast_with_module),
            PathBuf::from("build").join("beta.lua")
        );
        assert_eq!(
            resolve_output_path(None, input, &ast_with_module),
            PathBuf::from("out").join("beta.lua")
        );

        let (tokens, lex_diags) = lexer::lex(0, "");
        assert!(!lex_diags.has_errors());
        let (ast_without_module, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors());
        assert_eq!(
            resolve_output_path(None, Path::new("src/fallback.luna"), &ast_without_module),
            PathBuf::from("out").join("fallback.lua")
        );
    }

    #[test]
    fn constructor_arity_and_record_fields_are_validated() {
        let source = r#"
type Point { x: Int, y: Int }
type MaybeInt = | None | Some(Int)

fn main() -> Int do
let p = Point { z = 1 }
let m = Some(1, 2)
p.x
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors());
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors());
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(type_diags.items.iter().any(|d| {
            d.message
                .contains("unknown field 'z' in record initializer")
        }));
        assert!(
            type_diags
                .items
                .iter()
                .any(|d| d.message.contains("constructor argument count mismatch"))
        );
    }

    #[test]
    fn reports_non_exhaustive_match_for_sum_types() {
        let source = r#"
type MaybeInt = | None | Some(Int)

fn unwrap(m: MaybeInt) -> Int do
match m do
case Some(v) => v
end
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors());
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors());
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(
            type_diags
                .items
                .iter()
                .any(|d| d.message.contains("non-exhaustive match"))
        );
    }

    #[test]
    fn infers_generic_function_call_type_parameters() {
        let source = r#"
fn id[T](x: T) -> T do
x
end

fn main() -> Int do
id(1)
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors());
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors());
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);
    }

    #[test]
    fn import_module_alias_and_items_resolve_in_typecheck() {
        let source = r#"
import foo.bar
import foo.bar.{qux}

extern module foo.bar do
extern fn baz() -> Int
extern fn qux(x: Int) -> Int
end

fn main() -> Int do
bar.baz() + qux(1)
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors());
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);
    }

    #[test]
    fn extern_module_calls_emit_lua_paths() {
        let source = r#"
import foo.bar
import foo.bar.{qux}

extern module foo.bar do
extern fn baz() -> Int
extern fn qux(x: Int) -> Int
end

fn main() -> Int do
bar.baz() + qux(1)
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors());
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (tir, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);

        let lua = codegen_lua::emit_lua_module(&tir, &resolved);
        assert!(lua.contains("foo.bar.baz()"), "{lua}");
        assert!(lua.contains("foo.bar.qux(1)"), "{lua}");
    }

    #[test]
    fn imported_item_without_matching_declaration_reports_clear_error() {
        let source = r#"
import foo.bar.{qux}

fn main() -> Int do
qux(1)
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(type_diags.items.iter().any(|d| {
            d.message.contains(
                "imported item 'qux' resolves to 'foo.bar.qux' but no matching function/extern declaration exists",
            )
        }));
    }

    #[test]
    fn imported_module_missing_member_reports_clear_error() {
        let source = r#"
import foo.bar

extern module foo.bar do
extern fn baz() -> Int
end

fn main() -> Int do
bar.qux()
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(type_diags.items.iter().any(|d| {
            d.message.contains(
                "unknown imported module function 'foo.bar.qux'; add a matching extern declaration",
            )
        }));
    }

    #[test]
    fn compile_pipeline_loads_imported_module_files() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_multifile_{}", nonce));
        let lib_dir = root.join("lib");
        std::fs::create_dir_all(&lib_dir).expect("failed to create temp dirs");

        let lib = lib_dir.join("math.luna");
        std::fs::write(
            &lib,
            r#"
module lib.math

pub fn add(a: Int, b: Int) -> Int do
  a + b
end
"#,
        )
        .expect("failed to write lib module");

        let entry = root.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app

import lib.math

fn main() -> Int do
  math.add(1, 2)
end
"#,
        )
        .expect("failed to write entry module");

        let (_, _, diagnostics, compiled) = compile_pipeline(&entry).expect("pipeline failed");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.items);
        assert!(compiled.is_some());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_writes_imported_modules_when_output_is_directory() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_multifile_emit_{}", nonce));
        let lib_dir = root.join("lib");
        let out_dir = root.join("out");
        std::fs::create_dir_all(&lib_dir).expect("failed to create temp dirs");

        let lib = lib_dir.join("math.luna");
        std::fs::write(
            &lib,
            r#"
module lib.math

pub fn add(a: Int, b: Int) -> Int do
  a + b
end
"#,
        )
        .expect("failed to write lib module");

        let entry = root.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app

import lib.math

pub fn main() -> Int do
  math.add(1, 2)
end
"#,
        )
        .expect("failed to write entry module");

        emit_lua_command(&entry, Some(out_dir.as_path())).expect("emit failed");
        assert!(out_dir.join("app.lua").is_file());
        assert!(out_dir.join("lib").join("math.lua").is_file());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn nullary_constructor_pattern_is_not_lowered_as_bind() {
        let source = r#"
type MaybeInt = | None | Some(Int)

fn pick(m: MaybeInt) -> Int do
match m do
case None => 0
case Some(v) => v
end
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors());
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (tir, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);

        let lua = codegen_lua::emit_lua_module(&tir, &resolved);
        assert!(lua.contains("__scrutinee.tag == \"None\""), "{lua}");
    }

    #[test]
    fn functions_are_predeclared_for_forward_references() {
        let source = r#"
fn main() -> Int do
helper()
end

fn helper() -> Int do
1
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (tir, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);

        let lua = codegen_lua::emit_lua_module(&tir, &resolved);
        assert!(lua.contains("local main"), "{lua}");
        assert!(lua.contains("local helper"), "{lua}");
        assert!(lua.contains("main = function()"), "{lua}");
        assert!(lua.contains("return helper()"), "{lua}");
    }

    #[test]
    fn record_constructor_pattern_codegen_uses_named_fields() {
        let source = r#"
type Shape = | Circle { radius: Int }

fn area(s: Shape) -> Int do
match s do
case Circle { radius } => radius
end
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors());
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (tir, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);

        let lua = codegen_lua::emit_lua_module(&tir, &resolved);
        assert!(lua.contains("__scrutinee.radius"), "{lua}");
        assert!(!lua.contains("__scrutinee._1"), "{lua}");
    }

    #[test]
    fn resolve_reports_duplicate_import_aliases_and_items() {
        let source = r#"
import foo.bar
import baz.bar
import foo.one.{zap}
import foo.two.{zap}

fn main() -> Int do
0
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (_, resolve_diags) = resolve::resolve(&ast);
        assert!(resolve_diags.has_errors());
        assert!(
            resolve_diags
                .items
                .iter()
                .any(|d| d.message.contains("duplicate import alias 'bar'"))
        );
        assert!(
            resolve_diags
                .items
                .iter()
                .any(|d| d.message.contains("duplicate imported item 'zap'"))
        );
    }

    #[test]
    fn resolve_rejects_nullable_and_nil_types_outside_extern_context() {
        let source = r#"
fn main(x: Int not, y: Nil) -> Int do
1
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (_, resolve_diags) = resolve::resolve(&ast);
        assert!(resolve_diags.has_errors());
        assert!(resolve_diags.items.iter().any(|d| {
            d.message
                .contains("nullable types are only allowed in extern contexts")
        }));
        assert!(resolve_diags.items.iter().any(|d| {
            d.message
                .contains("nil type is only allowed in extern contexts")
        }));
    }

    #[test]
    fn constructor_payload_shape_mismatch_is_reported() {
        let source = r#"
type MaybeInt = | None | Some(Int)

fn main() -> Int do
let x = None(1)
0
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(
            type_diags
                .items
                .iter()
                .any(|d| d.message.contains("constructor does not accept a payload"))
        );
        assert!(type_diags.items.iter().any(|d| {
            d.message.contains("constructor does not accept a payload")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("remove the payload"))
        }));
    }

    #[test]
    fn record_update_reports_unknown_and_mistyped_fields() {
        let source = r#"
type Point { x: Int, y: Int }

fn main() -> Int do
let p = Point { x = 1, y = 2 } with { x = true, z = 3 }
p.x
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(
            type_diags
                .items
                .iter()
                .any(|d| d.message.contains("field 'x' expects Int but got Bool"))
        );
        assert!(
            type_diags
                .items
                .iter()
                .any(|d| d.message.contains("unknown field 'z' in record update"))
        );
        assert!(type_diags.items.iter().any(|d| {
            d.message.contains("unknown field 'z' in record update")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("expected fields: x, y"))
        }));
    }

    #[test]
    fn record_update_codegen_copies_base_before_overrides() {
        let source = r#"
type Point { x: Int, y: Int }

fn bump(p: Point) -> Point do
p with { x = p.x + 1 }
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (tir, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);

        let lua = codegen_lua::emit_lua_module(&tir, &resolved);
        assert!(
            lua.contains("for k, v in pairs(__base) do __tmp[k] = v end"),
            "{lua}"
        );
        assert!(lua.contains("__tmp.x = (p.x + 1)"), "{lua}");
    }

    #[test]
    fn generic_sum_and_record_constructors_infer_type_arguments() {
        let source = r#"
type Option[T] = | None | Some(T)
type Box[T] { value: T }

fn opt() -> Option[Int] do
Some(1)
end

fn boxify() -> Box[Int] do
Box { value = 1 }
end

fn unbox(b: Box[Int]) -> Int do
b.value
end

fn main() -> Int do
let b = boxify()
let b2 = b with { value = 2 }
match opt() do
case Some(v) => unbox(b2) + v
case None => 0
end
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (tir, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);

        let lua = codegen_lua::emit_lua_module(&tir, &resolved);
        assert!(lua.contains("tag = \"Some\""), "{lua}");
        assert!(lua.contains("_1 = 1"), "{lua}");
    }

    #[test]
    fn nullary_generic_constructor_uses_expected_context() {
        let source = r#"
type Option[T] = | None | Some(T)

fn takes(v: Option[Int]) -> Int do
  match v do
    case Some(x) => x
    case None => 0
  end
end

fn main() -> Int do
  let a: Option[Int] = None
  takes(None) + takes(a)
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);
    }

    #[test]
    fn unconstrained_nullary_generic_constructor_reports_error() {
        let source = r#"
type Option[T] = | None | Some(T)

fn main() -> Unit do
  let x = None
  ()
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(type_diags.items.iter().any(|d| {
            d.message.contains(
                "cannot infer generic type arguments for constructor 'None' without context",
            )
        }));
    }

    #[test]
    fn transparent_aliases_work_for_assignability_and_control_flow() {
        let source = r#"
type Distance = Int
type Flag = Bool
type Id[T] = T
type Option[T] = | None | Some(T)
type IntOpt = Option[Int]

fn choose(flag: Flag) -> Id[Int] do
  if flag then
    1
  else
    2
  end
end

fn pick(flag: Flag) -> IntOpt do
  if flag then
    Some(choose(flag))
  else
    None
  end
end

fn len(d: Distance) -> Distance do
  d + 1
end

fn main(flag: Flag) -> Distance do
  let base: Distance = 41
  let out: Id[Int] = choose(flag)
  let chosen: IntOpt = pick(flag)
  match chosen do
    case Some(v) => len(base + v)
    case None => len(base + out)
  end
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);
    }

    #[test]
    fn generic_record_constructor_without_type_context_reports_inference_failure() {
        let source = r#"
type Phantom[T] { value: Int }

fn main() -> Unit do
  let p = Phantom { value = 1 }
  ()
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(type_diags.items.iter().any(|d| {
            d.message.contains("could not infer generic type parameter")
                && d.message.contains("record initializer 'Phantom'")
        }));
    }

    #[test]
    fn alias_mismatch_failure_is_reported() {
        let source = r#"
type Id[T] = T

fn takes_id(x: Id[Int]) -> Int do
  x
end

fn main() -> Int do
  takes_id(true)
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(type_diags.items.iter().any(|d| {
            d.message.contains("argument 1 expects")
                && d.message.contains("Named(TypeId(")
                && d.message.contains("Bool")
        }));
    }

    #[test]
    fn calling_imported_module_alias_as_function_reports_clear_error() {
        let source = r#"
import foo.bar

extern module foo.bar do
  extern fn baz(x: Int) -> Int
end

fn main() -> Int do
  bar(1)
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(type_diags.items.iter().any(|d| {
            d.message
                .contains("cannot call imported module 'foo.bar' as a function")
                && d.notes.iter().any(|(_, note)| note.contains("module path"))
        }));
    }

    #[test]
    fn reports_non_exhaustive_match_for_generic_sum_types() {
        let source = r#"
type Option[T] = | None | Some(T)

fn unwrap(v: Option[Int]) -> Int do
  match v do
    case Some(x) => x
  end
end
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(type_diags.items.iter().any(|d| {
            d.message
                .contains("non-exhaustive match, missing variants: None")
        }));
    }
}
