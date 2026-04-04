#![allow(dead_code)]

mod ast;
mod cli;
mod codegen_lua;
mod config;
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
use config::ConfigSource;
use diagnostics::Diagnostics;
use source::SourceDb;
use span::FileId;

const DIAG_RES_IMPORT_MODULE_NOT_FOUND: &str = "CAL-RES-010";
const DIAG_RES_MODULE_READ_FAILED: &str = "CAL-RES-013";
const DIAG_RES_MODULE_DECL_MISMATCH: &str = "CAL-RES-014";
const DIAG_RES_DUPLICATE_MODULE_DEF: &str = "CAL-RES-015";

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
        Command::Check {
            input,
            config,
            module_roots,
        } => check_command(&input, config.as_deref(), &module_roots),
        Command::EmitLua {
            input,
            output,
            config,
            module_roots,
            playdate_bootstrap,
        } => emit_lua_command_with_overrides(
            &input,
            output.as_deref(),
            config.as_deref(),
            &module_roots,
            playdate_bootstrap,
        ),
        Command::Build {
            input,
            output,
            config,
            module_roots,
            playdate_bootstrap,
        } => build_command_with_overrides(
            &input,
            output.as_deref(),
            config.as_deref(),
            &module_roots,
            playdate_bootstrap,
        ),
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

#[derive(Debug, Clone)]
struct ProjectOptions {
    module_roots: Vec<PathBuf>,
    default_out_dir: PathBuf,
    config_source: ConfigSource,
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

fn check_command(
    input: &Path,
    explicit_config: Option<&Path>,
    cli_module_roots: &[PathBuf],
) -> Result<(), i32> {
    let options = resolve_project_options(input, explicit_config, cli_module_roots)?;
    let (sources, _, diagnostics, _) = compile_pipeline_with_options(input, &options)?;
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
    emit_lua_command_with_overrides(input, output, None, &[], false)
}

fn emit_lua_command_with_overrides(
    input: &Path,
    output: Option<&Path>,
    explicit_config: Option<&Path>,
    cli_module_roots: &[PathBuf],
    playdate_bootstrap: bool,
) -> Result<(), i32> {
    let options = resolve_project_options(input, explicit_config, cli_module_roots)?;
    let (sources, entry_ast, diagnostics, compiled_project) =
        compile_project_with_options(input, &options)?;
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
        if playdate_bootstrap {
            eprintln!(
                "--playdate-bootstrap requires a directory output (omit -o file.lua or pass an output directory)"
            );
            return Err(2);
        }
        let lua = codegen_lua::emit_lua_module(&entry.tir, &entry.resolved);
        let output_path = resolve_output_path(output, input, &entry_ast);
        write_lua_file(&output_path, &lua)?;
        println!("wrote {}", output_path.display());
        return Ok(());
    }

    let out_dir = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| options.default_out_dir.clone());
    for module in &project.modules {
        let lua = codegen_lua::emit_lua_module(&module.tir, &module.resolved);
        let output_path = resolve_module_output_path(&out_dir, input, module);
        write_lua_file(&output_path, &lua)?;
        println!("wrote {}", output_path.display());
    }

    if playdate_bootstrap {
        write_playdate_bootstrap(&out_dir, input, &project)?;
    }

    Ok(())
}

fn build_command(input: &Path, output: Option<&Path>) -> Result<(), i32> {
    build_command_with_overrides(input, output, None, &[], false)
}

fn build_command_with_overrides(
    input: &Path,
    output: Option<&Path>,
    explicit_config: Option<&Path>,
    cli_module_roots: &[PathBuf],
    playdate_bootstrap: bool,
) -> Result<(), i32> {
    emit_lua_command_with_overrides(
        input,
        output,
        explicit_config,
        cli_module_roots,
        playdate_bootstrap,
    )
}

fn resolve_project_options(
    input: &Path,
    explicit_config: Option<&Path>,
    cli_module_roots: &[PathBuf],
) -> Result<ProjectOptions, i32> {
    let loaded = match config::load_project_config(input, explicit_config) {
        Ok(loaded) => loaded,
        Err(err) => {
            eprintln!("{}", err);
            return Err(2);
        }
    };
    let config::LoadedProjectConfig { source, config } = loaded;

    let default_module_root = input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let module_roots = if !cli_module_roots.is_empty() {
        cli_module_roots
            .iter()
            .cloned()
            .map(normalize_path)
            .collect::<Vec<_>>()
    } else if !config.module_roots.is_empty() {
        config
            .module_roots
            .into_iter()
            .map(normalize_path)
            .collect::<Vec<_>>()
    } else {
        vec![normalize_path(default_module_root)]
    };

    let default_out_dir = config
        .out_dir
        .map(normalize_path)
        .unwrap_or_else(|| PathBuf::from("out"));

    Ok(ProjectOptions {
        module_roots,
        default_out_dir,
        config_source: source,
    })
}

fn default_project_options(input: &Path) -> ProjectOptions {
    let root = input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    ProjectOptions {
        module_roots: vec![normalize_path(root)],
        default_out_dir: PathBuf::from("out"),
        config_source: ConfigSource::Default,
    }
}

fn compile_project(
    input: &Path,
) -> Result<(SourceDb, ast::Module, Diagnostics, Option<CompiledProject>), i32> {
    let options = default_project_options(input);
    compile_project_with_options(input, &options)
}

fn compile_project_with_options(
    input: &Path,
    options: &ProjectOptions,
) -> Result<(SourceDb, ast::Module, Diagnostics, Option<CompiledProject>), i32> {
    let mut sources = SourceDb::new();
    let mut diagnostics = Diagnostics::new();

    let parsed_modules = load_module_graph(input, options, &mut sources, &mut diagnostics)?;
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
    let options = default_project_options(input);
    compile_pipeline_with_options(input, &options)
}

fn compile_pipeline_with_options(
    input: &Path,
    options: &ProjectOptions,
) -> Result<
    (
        SourceDb,
        ast::Module,
        Diagnostics,
        Option<(resolve::ResolvedModule, tir::TirModule)>,
    ),
    i32,
> {
    let (sources, entry_ast, diagnostics, project) = compile_project_with_options(input, options)?;
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
    options: &ProjectOptions,
    sources: &mut SourceDb,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<ParsedModule>, i32> {
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
                diagnostics.error_code(
                    span::Span::dummy(),
                    DIAG_RES_MODULE_READ_FAILED,
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
                    diagnostics.error_code(
                        decl.span,
                        DIAG_RES_MODULE_DECL_MISMATCH,
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
                    diagnostics.error_code(
                        span::Span::new(file_id, 0, 0),
                        DIAG_RES_DUPLICATE_MODULE_DEF,
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
            let lookup = find_module_file(&options.module_roots, &import.path);
            match lookup.path {
                Some(import_path) => {
                    queue.push_back((import_path, Some(import.path.clone()), false));
                }
                None => {
                    let note = if lookup.attempted.is_empty() {
                        "attempted paths: <none>".to_string()
                    } else {
                        let attempted = lookup
                            .attempted
                            .iter()
                            .map(|p| p.display().to_string())
                            .collect::<Vec<_>>()
                            .join("\n  - ");
                        format!("attempted paths:\n  - {}", attempted)
                    };
                    diagnostics.error_with_note_code(
                        import.span,
                        DIAG_RES_IMPORT_MODULE_NOT_FOUND,
                        format!(
                            "could not find module file for import '{}'",
                            import.path.join(".")
                        ),
                        import.span,
                        note,
                    );
                }
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

fn write_playdate_bootstrap(
    out_dir: &Path,
    input: &Path,
    project: &CompiledProject,
) -> Result<(), i32> {
    let entry = project
        .modules
        .get(project.entry_index)
        .expect("entry module present");
    let main_path = out_dir.join("main.lua");

    if project
        .modules
        .iter()
        .any(|module| resolve_module_output_path(out_dir, input, module) == main_path)
    {
        eprintln!(
            "--playdate-bootstrap would overwrite '{}' emitted from a module; rename the module or disable bootstrap",
            main_path.display()
        );
        return Err(1);
    }

    let has_update = entry.resolved.func_infos.iter().any(|info| {
        matches!(info.vis, ast::Visibility::Public)
            && matches!(info.kind, types::FuncKind::Normal)
            && info.name == "update"
            && info.params.is_empty()
            && matches!(info.ret, types::Type::Unit)
    });
    if !has_update {
        let module_name = if entry.parsed.module_path.is_empty() {
            "<entry>".to_string()
        } else {
            entry.parsed.module_path.join(".")
        };
        eprintln!(
            "--playdate-bootstrap requires entry module '{}' to export `pub fn update() -> Unit`",
            module_name
        );
        return Err(1);
    }

    let entry_out = resolve_module_output_path(out_dir, input, entry);
    let rel = entry_out
        .strip_prefix(out_dir)
        .expect("entry output should be inside out_dir");
    let mut import_rel = rel.to_path_buf();
    import_rel.set_extension("");
    let import_path = import_rel
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string();

    let lua = format!(
        "local game = import \"{}\"\n\nfunction playdate.update()\n    game.update()\nend\n",
        import_path
    );
    write_lua_file(&main_path, &lua)?;
    println!("wrote {}", main_path.display());
    Ok(())
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

struct ModuleLookup {
    path: Option<PathBuf>,
    attempted: Vec<PathBuf>,
}

fn find_module_file(root_dirs: &[PathBuf], module_path: &[String]) -> ModuleLookup {
    if module_path.is_empty() {
        return ModuleLookup {
            path: None,
            attempted: Vec::new(),
        };
    }

    let relative = module_path.iter().fold(PathBuf::new(), |mut acc, segment| {
        acc.push(segment);
        acc
    });

    let mut attempted = Vec::new();
    for root_dir in root_dirs {
        let file_candidates = ["luna", "cal"].into_iter().map(|ext| {
            let mut path = root_dir.join(&relative);
            path.set_extension(ext);
            path
        });
        for candidate in file_candidates {
            attempted.push(candidate.clone());
            if candidate.is_file() {
                return ModuleLookup {
                    path: Some(candidate),
                    attempted,
                };
            }
        }
        let mod_candidates = ["mod.luna", "mod.cal"]
            .into_iter()
            .map(|name| root_dir.join(&relative).join(name));
        for candidate in mod_candidates {
            attempted.push(candidate.clone());
            if candidate.is_file() {
                return ModuleLookup {
                    path: Some(candidate),
                    attempted,
                };
            }
        }
    }

    ModuleLookup {
        path: None,
        attempted,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{
        codegen_lua, config::ConfigSource, diagnostics::Diagnostics, lexer, parser, resolve,
        source::SourceDb, typecheck,
    };

    use super::{
        ProjectOptions, check_command, compile_pipeline, compile_pipeline_with_options,
        emit_lua_command, emit_lua_command_with_overrides, resolve_output_path,
        resolve_project_options,
    };

    fn render_diagnostics_for_source(file_name: &str, source: &str) -> String {
        let mut db = SourceDb::new();
        let file_id = db.add_file(PathBuf::from(file_name), source.to_string());

        let (tokens, lex_diags) = lexer::lex(file_id, source);
        let mut diagnostics = Diagnostics::new();
        diagnostics.extend(lex_diags);

        let (ast, parse_diags) = parser::parse(tokens);
        diagnostics.extend(parse_diags);

        let (resolved, resolve_diags) = resolve::resolve(&ast);
        diagnostics.extend(resolve_diags);

        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        diagnostics.extend(type_diags);

        diagnostics.render(&db)
    }

    fn assert_diagnostics_golden(name: &str, file_name: &str, source: &str) {
        let actual = render_diagnostics_for_source(file_name, source);
        let golden_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("golden")
            .join("diagnostics")
            .join(format!("{name}.txt"));

        if std::env::var("UPDATE_GOLDENS").ok().as_deref() == Some("1") {
            if let Some(parent) = golden_path.parent() {
                std::fs::create_dir_all(parent).expect("failed to create golden dir");
            }
            std::fs::write(&golden_path, &actual).expect("failed to write golden file");
        }

        let expected = std::fs::read_to_string(&golden_path).unwrap_or_else(|_| {
            panic!(
                "missing diagnostics golden '{}'; run with UPDATE_GOLDENS=1",
                golden_path.display()
            )
        });

        assert_eq!(
            actual,
            expected,
            "diagnostics golden '{}' mismatch",
            golden_path.display()
        );
    }

    fn emit_lua_for_source(file_name: &str, source: &str) -> String {
        let mut db = SourceDb::new();
        let file_id = db.add_file(PathBuf::from(file_name), source.to_string());

        let (tokens, lex_diags) = lexer::lex(file_id, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);

        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);

        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);

        let (tir, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);

        codegen_lua::emit_lua_module(&tir, &resolved)
    }

    fn assert_lua_golden(name: &str, file_name: &str, source: &str) {
        let actual = emit_lua_for_source(file_name, source);
        let golden_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("golden")
            .join("lua")
            .join(format!("{name}.lua"));

        if std::env::var("UPDATE_GOLDENS").ok().as_deref() == Some("1") {
            if let Some(parent) = golden_path.parent() {
                std::fs::create_dir_all(parent).expect("failed to create golden dir");
            }
            std::fs::write(&golden_path, &actual).expect("failed to write golden file");
        }

        let expected = std::fs::read_to_string(&golden_path).unwrap_or_else(|_| {
            panic!(
                "missing lua golden '{}'; run with UPDATE_GOLDENS=1",
                golden_path.display()
            )
        });

        assert_eq!(
            actual,
            expected,
            "lua golden '{}' mismatch",
            golden_path.display()
        );
    }

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
    fn check_command_fails_with_missing_explicit_config_file() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_check_missing_cfg_{}", nonce));
        std::fs::create_dir_all(&root).expect("failed to create temp dir");
        let entry = root.join("main.luna");
        std::fs::write(&entry, "fn main() -> Int do\n0\nend\n").expect("failed to write entry");
        let missing = root.join("missing.toml");

        assert_eq!(check_command(&entry, Some(&missing), &[]).unwrap_err(), 2);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn check_command_fails_with_invalid_discovered_config_toml() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_check_bad_toml_{}", nonce));
        std::fs::create_dir_all(&root).expect("failed to create temp dir");
        let entry = root.join("main.luna");
        std::fs::write(&entry, "fn main() -> Int do\n0\nend\n").expect("failed to write entry");
        std::fs::write(root.join("callisto.toml"), "module_roots = [\n")
            .expect("failed to write config");

        assert_eq!(check_command(&entry, None, &[]).unwrap_err(), 2);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn check_command_fails_with_invalid_discovered_config_field_values() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_check_bad_cfg_value_{}", nonce));
        std::fs::create_dir_all(&root).expect("failed to create temp dir");
        let entry = root.join("main.luna");
        std::fs::write(&entry, "fn main() -> Int do\n0\nend\n").expect("failed to write entry");
        std::fs::write(root.join("callisto.toml"), "module_roots = [\"\"]\n")
            .expect("failed to write config");

        assert_eq!(check_command(&entry, None, &[]).unwrap_err(), 2);

        let _ = std::fs::remove_dir_all(root);
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
    fn module_resolution_prefers_first_matching_root_in_order() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_multiroot_order_{}", nonce));
        let entry_dir = root.join("entry");
        let root_a = root.join("roots").join("a");
        let root_b = root.join("roots").join("b");
        std::fs::create_dir_all(entry_dir.as_path()).expect("failed to create entry dir");
        std::fs::create_dir_all(root_a.join("foo")).expect("failed to create root_a");
        std::fs::create_dir_all(root_b.join("foo")).expect("failed to create root_b");

        let module_path_a = root_a.join("foo").join("bar.luna");
        std::fs::write(
            &module_path_a,
            r#"
module foo.bar

pub fn value() -> Int do
  true
end
"#,
        )
        .expect("failed to write root_a module");
        std::fs::write(
            root_b.join("foo").join("bar.luna"),
            r#"
module foo.bar

pub fn value() -> Int do
  1
end
"#,
        )
        .expect("failed to write root_b module");

        let entry = entry_dir.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app

import foo.bar

fn main() -> Int do
  bar.value()
end
"#,
        )
        .expect("failed to write entry module");

        let options = ProjectOptions {
            module_roots: vec![root_a.clone(), root_b.clone()],
            default_out_dir: PathBuf::from("out"),
            config_source: ConfigSource::Default,
        };

        let (sources, _, diagnostics, compiled) =
            compile_pipeline_with_options(&entry, &options).expect("pipeline failed");
        assert!(diagnostics.has_errors());
        assert!(compiled.is_none());
        let rendered = diagnostics.render(&sources);
        assert!(rendered.contains(module_path_a.to_string_lossy().as_ref()));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn check_command_uses_config_module_root_order_deterministically() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_cfg_root_order_{}", nonce));
        let entry_dir = root.join("entry");
        let first_root = root.join("first");
        let second_root = root.join("second");
        std::fs::create_dir_all(first_root.join("lib")).expect("failed to create first root");
        std::fs::create_dir_all(second_root.join("lib")).expect("failed to create second root");
        std::fs::create_dir_all(&entry_dir).expect("failed to create entry dir");

        std::fs::write(
            first_root.join("lib").join("math.luna"),
            r#"
module lib.math

pub fn add(a: Int, b: Int) -> Int do
  true
end
"#,
        )
        .expect("failed to write first root module");
        std::fs::write(
            second_root.join("lib").join("math.luna"),
            r#"
module lib.math

pub fn add(a: Int, b: Int) -> Int do
  a + b
end
"#,
        )
        .expect("failed to write second root module");

        let entry = entry_dir.join("main.luna");
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
        .expect("failed to write entry");

        std::fs::write(
            entry_dir.join("callisto.toml"),
            "module_roots = [\"../first\", \"../second\"]\n",
        )
        .expect("failed to write config");
        assert_eq!(check_command(&entry, None, &[]).unwrap_err(), 1);

        std::fs::write(
            entry_dir.join("callisto.toml"),
            "module_roots = [\"../second\", \"../first\"]\n",
        )
        .expect("failed to rewrite config");
        assert!(check_command(&entry, None, &[]).is_ok());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn resolver_defaults_to_entry_directory_root_when_no_config_or_cli_roots() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_multiroot_default_{}", nonce));
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

        let options = resolve_project_options(&entry, None, &[]).expect("resolve options");
        assert_eq!(
            options.module_roots,
            vec![entry.parent().expect("entry parent").to_path_buf()]
        );

        let (_, _, diagnostics, compiled) =
            compile_pipeline_with_options(&entry, &options).expect("pipeline failed");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.items);
        assert!(compiled.is_some());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn unresolved_import_reports_attempted_paths_note() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_multiroot_note_{}", nonce));
        let entry_dir = root.join("entry");
        let root_a = root.join("roots").join("a");
        let root_b = root.join("roots").join("b");
        std::fs::create_dir_all(entry_dir.as_path()).expect("failed to create entry dir");
        std::fs::create_dir_all(&root_a).expect("failed to create root_a");
        std::fs::create_dir_all(&root_b).expect("failed to create root_b");

        let entry = entry_dir.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app

import missing.mod

fn main() -> Int do
  0
end
"#,
        )
        .expect("failed to write entry module");

        let options = ProjectOptions {
            module_roots: vec![root_a.clone(), root_b.clone()],
            default_out_dir: PathBuf::from("out"),
            config_source: ConfigSource::Default,
        };

        let (_, _, diagnostics, compiled) =
            compile_pipeline_with_options(&entry, &options).expect("pipeline failed");
        assert!(diagnostics.has_errors());
        assert!(compiled.is_none());

        let import_diag = diagnostics
            .items
            .iter()
            .find(|d| {
                d.message
                    .contains("could not find module file for import 'missing.mod'")
            })
            .expect("missing import diagnostic");
        assert!(!import_diag.notes.is_empty());
        let note = &import_diag.notes[0].1;
        assert!(note.contains("attempted paths:"));
        assert!(
            note.contains(
                root_a
                    .join("missing")
                    .join("mod.luna")
                    .to_string_lossy()
                    .as_ref()
            )
        );
        assert!(
            note.contains(
                root_b
                    .join("missing")
                    .join("mod.luna")
                    .to_string_lossy()
                    .as_ref()
            )
        );

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
    fn emit_lua_uses_config_out_dir_when_o_not_provided() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_emit_cfg_out_{}", nonce));
        let src_dir = root.join("src");
        let shared_dir = root.join("shared");
        std::fs::create_dir_all(shared_dir.join("lib")).expect("failed to create shared dirs");
        std::fs::create_dir_all(&src_dir).expect("failed to create src dir");

        std::fs::write(
            shared_dir.join("lib").join("math.luna"),
            r#"
module lib.math

pub fn add(a: Int, b: Int) -> Int do
  a + b
end
"#,
        )
        .expect("failed to write shared module");
        let entry = src_dir.join("main.luna");
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
        .expect("failed to write entry");
        std::fs::write(
            src_dir.join("callisto.toml"),
            "module_roots = [\"../shared\"]\nout_dir = \"cfg_build\"\n",
        )
        .expect("failed to write config");

        emit_lua_command(&entry, None).expect("emit failed");
        assert!(src_dir.join("cfg_build").join("app.lua").is_file());
        assert!(
            src_dir
                .join("cfg_build")
                .join("lib")
                .join("math.lua")
                .is_file()
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_o_flag_overrides_config_out_dir() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_emit_o_override_{}", nonce));
        let src_dir = root.join("src");
        let shared_dir = root.join("shared");
        let explicit_out = root.join("explicit_out");
        std::fs::create_dir_all(shared_dir.join("lib")).expect("failed to create shared dirs");
        std::fs::create_dir_all(&src_dir).expect("failed to create src dir");

        std::fs::write(
            shared_dir.join("lib").join("math.luna"),
            r#"
module lib.math

pub fn add(a: Int, b: Int) -> Int do
  a + b
end
"#,
        )
        .expect("failed to write shared module");
        let entry = src_dir.join("main.luna");
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
        .expect("failed to write entry");
        std::fs::write(
            src_dir.join("callisto.toml"),
            "module_roots = [\"../shared\"]\nout_dir = \"cfg_build\"\n",
        )
        .expect("failed to write config");

        emit_lua_command(&entry, Some(explicit_out.as_path())).expect("emit failed");
        assert!(explicit_out.join("app.lua").is_file());
        assert!(explicit_out.join("lib").join("math.lua").is_file());
        assert!(!src_dir.join("cfg_build").join("app.lua").is_file());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_playdate_bootstrap_writes_main_shim() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_playdate_bootstrap_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("game.cal");
        std::fs::write(
            &entry,
            r#"
module app.game

pub fn update() -> Unit do
  ()
end
"#,
        )
        .expect("failed to write entry");

        emit_lua_command_with_overrides(&entry, Some(out_dir.as_path()), None, &[], true)
            .expect("emit failed");

        let module_lua = out_dir.join("app").join("game.lua");
        assert!(module_lua.is_file(), "missing module output");
        let shim = out_dir.join("main.lua");
        assert!(shim.is_file(), "missing playdate shim");
        let shim_text = std::fs::read_to_string(&shim).expect("read shim");
        assert!(
            shim_text.contains("local game = import \"app/game\""),
            "{shim_text}"
        );
        assert!(shim_text.contains("game.update()"), "{shim_text}");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_playdate_bootstrap_requires_public_zero_arg_update() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("callisto_playdate_bootstrap_missing_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("game.cal");
        std::fs::write(
            &entry,
            r#"
module app.game

pub fn tick() -> Unit do
  ()
end
"#,
        )
        .expect("failed to write entry");

        let result =
            emit_lua_command_with_overrides(&entry, Some(out_dir.as_path()), None, &[], true);
        assert_eq!(result.unwrap_err(), 1);
        assert!(!out_dir.join("main.lua").is_file());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_playdate_input_binding_emits_button_paths() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_playdate_input_emit_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("main.cal");
        std::fs::write(
            &entry,
            r#"
module app

import playdate.input

pub fn poll() -> Bool do
  input.a_pressed()
end
"#,
        )
        .expect("failed to write entry");

        let bindings_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("playdate_bindings")
            .join("src");
        emit_lua_command_with_overrides(
            &entry,
            Some(out_dir.as_path()),
            None,
            std::slice::from_ref(&bindings_root),
            false,
        )
        .expect("emit failed");

        let input_lua = out_dir.join("playdate").join("input.lua");
        let input_text = std::fs::read_to_string(&input_lua).expect("read input lua");
        assert!(input_text.contains("playdate.buttonIsPressed"), "{input_text}");
        assert!(input_text.contains("M.a_pressed = a_pressed"), "{input_text}");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_playdate_audio_binding_emits_playnote_paths() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_playdate_audio_emit_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("main.cal");
        std::fs::write(
            &entry,
            r#"
module app

import playdate.audio

pub fn cue() -> Unit do
  audio.bounce_blip()
end
"#,
        )
        .expect("failed to write entry");

        let bindings_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("playdate_bindings")
            .join("src");
        emit_lua_command_with_overrides(
            &entry,
            Some(out_dir.as_path()),
            None,
            std::slice::from_ref(&bindings_root),
            false,
        )
        .expect("emit failed");

        let audio_lua = out_dir.join("playdate").join("audio.lua");
        let audio_text = std::fs::read_to_string(&audio_lua).expect("read audio lua");
        assert!(audio_text.contains("playdate.sound.playNote"), "{audio_text}");
        assert!(audio_text.contains("M.bounce_blip = bounce_blip"), "{audio_text}");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_playdate_system_binding_emits_crank_paths() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_playdate_system_emit_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("main.cal");
        std::fs::write(
            &entry,
            r#"
module app

import playdate.system

pub fn right_half() -> Bool do
  system.crank_is_right_half()
end
"#,
        )
        .expect("failed to write entry");

        let bindings_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("playdate_bindings")
            .join("src");
        emit_lua_command_with_overrides(
            &entry,
            Some(out_dir.as_path()),
            None,
            std::slice::from_ref(&bindings_root),
            false,
        )
        .expect("emit failed");

        let system_lua = out_dir.join("playdate").join("system.lua");
        let system_text = std::fs::read_to_string(&system_lua).expect("read system lua");
        assert!(system_text.contains("playdate.getCrankPosition"), "{system_text}");
        assert!(
            system_text.contains("M.crank_is_right_half = crank_is_right_half"),
            "{system_text}"
        );

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

    #[test]
    fn diagnostics_golden_constructor_payload_note() {
        let source = r#"
type MaybeInt = | None | Some(Int)

fn main() -> Int do
  let x = None(1)
  0
end
"#;
        assert_diagnostics_golden(
            "constructor_payload_note",
            "golden_constructor_payload.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_imported_module_member_missing() {
        let source = r#"
import foo.bar

extern module foo.bar do
  extern fn baz() -> Int
end

fn main() -> Int do
  bar.qux()
end
"#;
        assert_diagnostics_golden(
            "imported_module_member_missing",
            "golden_import_member.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_unresolved_name() {
        let source = r#"
fn main() -> Int do
  missing_name
end
"#;
        assert_diagnostics_golden("unresolved_name", "golden_unresolved_name.luna", source);
    }

    #[test]
    fn diagnostics_golden_non_exhaustive_match() {
        let source = r#"
type MaybeInt = | None | Some(Int)

fn unwrap(m: MaybeInt) -> Int do
  match m do
    case Some(v) => v
  end
end
"#;
        assert_diagnostics_golden(
            "non_exhaustive_match",
            "golden_non_exhaustive_match.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_duplicate_import_alias() {
        let source = r#"
import foo.bar
import baz.bar

fn main() -> Int do
  0
end
"#;
        assert_diagnostics_golden(
            "duplicate_import_alias",
            "golden_duplicate_import_alias.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_imported_item_missing_declaration() {
        let source = r#"
import foo.bar.{qux}

fn main() -> Int do
  qux(1)
end
"#;
        assert_diagnostics_golden(
            "imported_item_missing_declaration",
            "golden_import_item_missing.luna",
            source,
        );
    }

    #[test]
    fn lua_golden_record_update() {
        let source = r#"
type Point { x: Int, y: Int }

fn bump(p: Point) -> Point do
  p with { x = p.x + 1 }
end
"#;
        assert_lua_golden("record_update", "golden_record_update.luna", source);
    }

    #[test]
    fn lua_golden_sum_match() {
        let source = r#"
type MaybeInt = | None | Some(Int)

fn pick(m: MaybeInt) -> Int do
  match m do
    case Some(v) => v
    case None => 0
  end
end
"#;
        assert_lua_golden("sum_match", "golden_sum_match.luna", source);
    }
}
