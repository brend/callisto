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
    process::Command as ProcessCommand,
};

use cli::{Cli, Command, PlaydateTemplateWorkflow};
use config::ConfigSource;
use diagnostics::Diagnostics;
use source::SourceDb;
use span::FileId;

const DIAG_RES_IMPORT_MODULE_NOT_FOUND: &str = "CAL-RES-010";
const DIAG_RES_MODULE_READ_FAILED: &str = "CAL-RES-013";
const DIAG_RES_MODULE_DECL_MISMATCH: &str = "CAL-RES-014";
const DIAG_RES_DUPLICATE_MODULE_DEF: &str = "CAL-RES-015";
const PLAYDATE_TEMPLATE_BINDINGS: &[(&str, &str)] = &[
    (
        "bindings/math.cal",
        include_str!("../playdate_bindings/src/math.cal"),
    ),
    (
        "bindings/playdate.cal",
        include_str!("../playdate_bindings/src/playdate.cal"),
    ),
    (
        "bindings/playdate/graphics.cal",
        include_str!("../playdate_bindings/src/playdate/graphics.cal"),
    ),
    (
        "bindings/playdate/graphics/sprite.cal",
        include_str!("../playdate_bindings/src/playdate/graphics/sprite.cal"),
    ),
    (
        "bindings/playdate/input.cal",
        include_str!("../playdate_bindings/src/playdate/input.cal"),
    ),
    (
        "bindings/playdate/audio.cal",
        include_str!("../playdate_bindings/src/playdate/audio.cal"),
    ),
    (
        "bindings/playdate/system.cal",
        include_str!("../playdate_bindings/src/playdate/system.cal"),
    ),
    (
        "bindings/playdate/timer.cal",
        include_str!("../playdate_bindings/src/playdate/timer.cal"),
    ),
];

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
        Command::InitPlaydate {
            dir,
            workflow,
            starter_assets,
        } => init_playdate_template_command(&dir, workflow, starter_assets),
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
            playdate_bootstrap_target,
            playdate_bootstrap_preloads,
        } => {
            let bootstrap = if playdate_bootstrap {
                Some(parse_playdate_bootstrap_options(
                    playdate_bootstrap_target.as_deref(),
                    &playdate_bootstrap_preloads,
                )?)
            } else {
                None
            };
            emit_lua_command_with_bootstrap_options(
                &input,
                output.as_deref(),
                config.as_deref(),
                &module_roots,
                bootstrap.as_ref(),
            )
        }
        Command::Build {
            input,
            output,
            config,
            module_roots,
            playdate_bootstrap,
            playdate_bootstrap_target,
            playdate_bootstrap_preloads,
        } => {
            let bootstrap = if playdate_bootstrap {
                Some(parse_playdate_bootstrap_options(
                    playdate_bootstrap_target.as_deref(),
                    &playdate_bootstrap_preloads,
                )?)
            } else {
                None
            };
            build_command_with_bootstrap_options(
                &input,
                output.as_deref(),
                config.as_deref(),
                &module_roots,
                bootstrap.as_ref(),
            )
        }
        Command::BuildPlaydate {
            input,
            source_dir,
            pdx,
            pdc,
            run,
            config,
            module_roots,
            playdate_bootstrap_target,
            playdate_bootstrap_preloads,
        } => {
            let bootstrap = parse_playdate_bootstrap_options(
                playdate_bootstrap_target.as_deref(),
                &playdate_bootstrap_preloads,
            )?;
            build_playdate_command_with_bootstrap_options(
                &input,
                source_dir.as_deref(),
                pdx.as_deref(),
                pdc.as_deref(),
                run,
                config.as_deref(),
                &module_roots,
                &bootstrap,
            )
        }
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

type EntryCompilation = Option<(resolve::ResolvedModule, tir::TirModule)>;
type CompilePipelineResult = (SourceDb, ast::Module, Diagnostics, EntryCompilation);

#[derive(Debug, Clone)]
struct ProjectOptions {
    module_roots: Vec<PathBuf>,
    default_out_dir: PathBuf,
    config_source: ConfigSource,
}

#[derive(Debug, Clone, Default)]
struct PlaydateBootstrapOptions {
    update_target: Option<String>,
    preloads: Vec<PlaydateBootstrapPreload>,
}

#[derive(Debug, Clone)]
struct PlaydateBootstrapPreload {
    assign_target: Option<String>,
    import_path: String,
}

fn init_playdate_template_command(
    dir: &Path,
    workflow: PlaydateTemplateWorkflow,
    starter_assets: bool,
) -> Result<(), i32> {
    let root = normalize_path(dir.to_path_buf());
    if root.exists() {
        if !root.is_dir() {
            eprintln!(
                "cannot initialize project at '{}': path is not a directory",
                root.display()
            );
            return Err(1);
        }
        let mut entries = match std::fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(err) => {
                eprintln!("failed to read '{}': {}", root.display(), err);
                return Err(1);
            }
        };
        if entries.next().is_some() {
            eprintln!(
                "cannot initialize playdate template in non-empty directory '{}'",
                root.display()
            );
            return Err(1);
        }
    } else if let Err(err) = std::fs::create_dir_all(&root) {
        eprintln!(
            "failed to create project directory '{}': {}",
            root.display(),
            err
        );
        return Err(1);
    }

    let src_dir = root.join("src");
    if let Err(err) = std::fs::create_dir_all(&src_dir) {
        eprintln!("failed to create '{}': {}", src_dir.display(), err);
        return Err(1);
    }
    let source_dir = root.join("Source");
    if let Err(err) = std::fs::create_dir_all(&source_dir) {
        eprintln!("failed to create '{}': {}", source_dir.display(), err);
        return Err(1);
    }

    let package_suffix = root
        .file_name()
        .and_then(|n| n.to_str())
        .map(sanitize_package_segment)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "game".to_string());
    let package_name = format!("playdate.{}", package_suffix);

    let callisto_toml = format!(
        "out_dir = \"Source\"\nmodule_roots = [\"src\", \"bindings\"]\npackage = \"{}\"\n",
        package_name
    );
    let game_cal = r#"module game

import playdate.graphics

type State {
  frame: Int
}

pub fn init() -> State {
  State { frame = 0 }
}

pub fn update(state: State) -> State {
  state with { frame = state.frame + 1 }
}

pub fn render(state: State) -> Unit {
  graphics.clear()
  graphics.drawText("Hello from Callisto", 20.0, 40.0)
  ()
}
"#;
    let readme = playdate_template_readme(&package_name, workflow, starter_assets);
    let makefile = playdate_template_makefile(workflow);

    write_project_file(root.join("callisto.toml"), &callisto_toml)?;
    write_project_file(src_dir.join("game.cal"), game_cal)?;
    write_project_file(root.join("README.md"), &readme)?;
    write_project_file(root.join("Makefile"), makefile)?;
    if workflow == PlaydateTemplateWorkflow::ManualShim {
        write_project_file(
            source_dir.join("main.lua"),
            "local game = import \"game\"\nlocal state = game.init()\n\nfunction playdate.update()\n    state = game.update(state)\n    game.render(state)\nend\n",
        )?;
    }
    if starter_assets {
        write_project_file(source_dir.join("images").join(".keep"), "")?;
        write_project_file(source_dir.join("sounds").join(".keep"), "")?;
        write_project_file(source_dir.join("fonts").join(".keep"), "")?;
    }
    for (rel_path, contents) in PLAYDATE_TEMPLATE_BINDINGS {
        write_project_file(root.join(rel_path), contents)?;
    }

    println!("initialized playdate template at {}", root.display());
    Ok(())
}

fn playdate_template_readme(
    package_name: &str,
    workflow: PlaydateTemplateWorkflow,
    starter_assets: bool,
) -> String {
    let workflow_lines = match workflow {
        PlaydateTemplateWorkflow::AutoBootstrap => {
            "Workflow: auto bootstrap (`--playdate-bootstrap`).\n"
        }
        PlaydateTemplateWorkflow::ManualShim => {
            "Workflow: manual shim (`Source/main.lua` owns frame-to-frame state).\n"
        }
    };
    let optional_bootstrap = match workflow {
        PlaydateTemplateWorkflow::AutoBootstrap => {
            "\n## Optional Bootstrap Customization\n\n```sh\ncallisto build src/game.cal -o Source --config callisto.toml --playdate-bootstrap \\\n  --playdate-bootstrap-target playdate.gameUpdate \\\n  --playdate-bootstrap-preload playdate.input=playdate/input\n```\n"
        }
        PlaydateTemplateWorkflow::ManualShim => "",
    };
    let starter_assets_note = if starter_assets {
        "\n## Starter Assets\n\nTemplate includes starter asset folders:\n- `Source/images/`\n- `Source/sounds/`\n- `Source/fonts/`\n"
    } else {
        ""
    };
    format!(
        "# {}\n\nPlaydate project scaffold generated by `callisto init --template playdate`.\n\n{}## Build Lua\n\n```sh\nmake build-lua\n```\n\n## Build .pdx\n\n```sh\nmake build\n```\n\n## Run\n\n```sh\nmake run\n```\n{}{}",
        package_name, workflow_lines, optional_bootstrap, starter_assets_note
    )
}

fn playdate_template_makefile(workflow: PlaydateTemplateWorkflow) -> &'static str {
    match workflow {
        PlaydateTemplateWorkflow::AutoBootstrap => {
            "CALLISTO ?= callisto\nPDC ?= pdc\nPDX ?= Game.pdx\n\nbuild-lua:\n\t$(CALLISTO) build src/game.cal --config callisto.toml -o Source/ --playdate-bootstrap\n\nbuild: build-lua\n\t$(PDC) Source/ $(PDX)\n\nrun: build\n\topen $(PDX)\n\nbuild-playdate:\n\t$(CALLISTO) build-playdate src/game.cal --config callisto.toml --pdx $(PDX)\n\nrun-playdate:\n\t$(CALLISTO) build-playdate src/game.cal --config callisto.toml --pdx $(PDX) --run\n"
        }
        PlaydateTemplateWorkflow::ManualShim => {
            "CALLISTO ?= callisto\nPDC ?= pdc\nPDX ?= Game.pdx\n\nbuild-lua:\n\t$(CALLISTO) build src/game.cal --config callisto.toml -o Source/\n\nbuild: build-lua\n\t$(PDC) Source/ $(PDX)\n\nrun: build\n\topen $(PDX)\n"
        }
    }
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
    let bootstrap = playdate_bootstrap.then_some(PlaydateBootstrapOptions::default());
    emit_lua_command_with_bootstrap_options(
        input,
        output,
        explicit_config,
        cli_module_roots,
        bootstrap.as_ref(),
    )
}

fn emit_lua_command_with_bootstrap_options(
    input: &Path,
    output: Option<&Path>,
    explicit_config: Option<&Path>,
    cli_module_roots: &[PathBuf],
    playdate_bootstrap: Option<&PlaydateBootstrapOptions>,
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
        if playdate_bootstrap.is_some() {
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

    if let Some(bootstrap) = playdate_bootstrap {
        write_playdate_bootstrap(&out_dir, input, &project, bootstrap)?;
    }

    Ok(())
}

fn build_command(input: &Path, output: Option<&Path>) -> Result<(), i32> {
    build_command_with_overrides(input, output, None, &[], false)
}

fn build_playdate_command_with_overrides(
    input: &Path,
    source_dir: Option<&Path>,
    pdx: Option<&Path>,
    pdc: Option<&str>,
    run_after_build: bool,
    explicit_config: Option<&Path>,
    cli_module_roots: &[PathBuf],
) -> Result<(), i32> {
    build_playdate_command_with_bootstrap_options(
        input,
        source_dir,
        pdx,
        pdc,
        run_after_build,
        explicit_config,
        cli_module_roots,
        &PlaydateBootstrapOptions::default(),
    )
}

fn build_playdate_command_with_bootstrap_options(
    input: &Path,
    source_dir: Option<&Path>,
    pdx: Option<&Path>,
    pdc: Option<&str>,
    run_after_build: bool,
    explicit_config: Option<&Path>,
    cli_module_roots: &[PathBuf],
    playdate_bootstrap: &PlaydateBootstrapOptions,
) -> Result<(), i32> {
    let out_dir = if let Some(source_dir) = source_dir {
        source_dir.to_path_buf()
    } else {
        let options = resolve_project_options(input, explicit_config, cli_module_roots)?;
        if options.default_out_dir == Path::new("out") {
            PathBuf::from("Source")
        } else {
            options.default_out_dir
        }
    };

    emit_lua_command_with_bootstrap_options(
        input,
        Some(out_dir.as_path()),
        explicit_config,
        cli_module_roots,
        Some(playdate_bootstrap),
    )?;

    let default_stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Game");
    let pdx_path = pdx.map(Path::to_path_buf).unwrap_or_else(|| {
        let root = out_dir.parent().unwrap_or(Path::new("."));
        root.join(format!("{}.pdx", default_stem))
    });

    let pdc_exe = pdc.unwrap_or("pdc");
    let status = match ProcessCommand::new(pdc_exe)
        .arg(&out_dir)
        .arg(&pdx_path)
        .status()
    {
        Ok(status) => status,
        Err(err) => {
            eprintln!("failed to execute '{}': {}", pdc_exe, err);
            eprintln!(
                "build-playdate source directory: {}; output bundle: {}",
                out_dir.display(),
                pdx_path.display()
            );
            eprintln!("install the Playdate SDK or pass --pdc <path-to-pdc> for build-playdate");
            return Err(2);
        }
    };
    if !status.success() {
        eprintln!(
            "Playdate build failed: '{}' returned exit code {:?} while compiling '{}' to '{}'",
            pdc_exe,
            status.code(),
            out_dir.display(),
            pdx_path.display()
        );
        return Err(1);
    }
    println!("built {}", pdx_path.display());

    if run_after_build {
        let run_status = match ProcessCommand::new("open").arg(&pdx_path).status() {
            Ok(status) => status,
            Err(err) => {
                eprintln!("failed to launch simulator with 'open': {}", err);
                return Err(2);
            }
        };
        if !run_status.success() {
            eprintln!("failed to open '{}'", pdx_path.display());
            return Err(1);
        }
        println!("opened {}", pdx_path.display());
    }

    Ok(())
}

fn build_command_with_overrides(
    input: &Path,
    output: Option<&Path>,
    explicit_config: Option<&Path>,
    cli_module_roots: &[PathBuf],
    playdate_bootstrap: bool,
) -> Result<(), i32> {
    let bootstrap = playdate_bootstrap.then_some(PlaydateBootstrapOptions::default());
    build_command_with_bootstrap_options(
        input,
        output,
        explicit_config,
        cli_module_roots,
        bootstrap.as_ref(),
    )
}

fn build_command_with_bootstrap_options(
    input: &Path,
    output: Option<&Path>,
    explicit_config: Option<&Path>,
    cli_module_roots: &[PathBuf],
    playdate_bootstrap: Option<&PlaydateBootstrapOptions>,
) -> Result<(), i32> {
    emit_lua_command_with_bootstrap_options(
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

fn compile_pipeline(input: &Path) -> Result<CompilePipelineResult, i32> {
    let options = default_project_options(input);
    compile_pipeline_with_options(input, &options)
}

fn compile_pipeline_with_options(
    input: &Path,
    options: &ProjectOptions,
) -> Result<CompilePipelineResult, i32> {
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
            if let Some(existing) = module_to_path.insert(key.clone(), path.clone())
                && existing != path
            {
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

        let public_concrete_type_decls: HashMap<String, ast::TypeDecl> = imported
            .ast
            .decls
            .iter()
            .filter_map(|decl| match decl {
                ast::TopDecl::Type(type_decl)
                    if matches!(type_decl.vis, ast::Visibility::Public) =>
                {
                    Some((type_decl.name.clone(), type_decl.clone()))
                }
                _ => None,
            })
            .collect();
        let public_extern_type_names: HashSet<String> = imported
            .ast
            .decls
            .iter()
            .filter_map(|decl| match decl {
                ast::TopDecl::ExternType(type_decl)
                    if matches!(type_decl.vis, ast::Visibility::Public) =>
                {
                    Some(type_decl.name.clone())
                }
                _ => None,
            })
            .collect();

        let mut requested_concrete_type_names = HashSet::new();
        if let Some(items) = &import.items {
            for item in items {
                if public_concrete_type_decls.contains_key(item) {
                    requested_concrete_type_names.insert(item.clone());
                }
            }
        }

        if !requested_concrete_type_names.is_empty() {
            let mut concrete_type_names_to_import = HashSet::new();
            let mut extern_type_names_to_import = HashSet::new();
            let mut pending = VecDeque::new();

            for name in requested_concrete_type_names {
                if concrete_type_names_to_import.insert(name.clone()) {
                    pending.push_back(name);
                }
            }

            while let Some(name) = pending.pop_front() {
                let Some(type_decl) = public_concrete_type_decls.get(&name) else {
                    continue;
                };
                for referenced_name in collect_type_decl_named_refs(type_decl) {
                    if public_concrete_type_decls.contains_key(&referenced_name) {
                        if concrete_type_names_to_import.insert(referenced_name.clone()) {
                            pending.push_back(referenced_name);
                        }
                    } else if public_extern_type_names.contains(&referenced_name) {
                        extern_type_names_to_import.insert(referenced_name);
                    }
                }
            }

            if let Some(items) = &import.items {
                for item in items {
                    if public_extern_type_names.contains(item) {
                        extern_type_names_to_import.insert(item.clone());
                    }
                }
            }

            for decl in &imported.ast.decls {
                match decl {
                    ast::TopDecl::Type(type_decl)
                        if matches!(type_decl.vis, ast::Visibility::Public)
                            && concrete_type_names_to_import.contains(&type_decl.name)
                            && known_type_names.insert(type_decl.name.clone()) =>
                    {
                        let mut ty = type_decl.clone();
                        ty.vis = ast::Visibility::Private;
                        ast.decls.push(ast::TopDecl::Type(ty));
                    }
                    ast::TopDecl::ExternType(type_decl)
                        if matches!(type_decl.vis, ast::Visibility::Public)
                            && extern_type_names_to_import.contains(&type_decl.name)
                            && known_type_names.insert(type_decl.name.clone()) =>
                    {
                        let mut ty = type_decl.clone();
                        ty.vis = ast::Visibility::Private;
                        ast.decls.push(ast::TopDecl::ExternType(ty));
                    }
                    _ => {}
                }
            }
        } else {
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
                if let Some(extern_type) = extern_type
                    && known_type_names.insert(extern_type.name.clone())
                {
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

fn collect_type_decl_named_refs(type_decl: &ast::TypeDecl) -> Vec<String> {
    let mut refs = Vec::new();
    match &type_decl.body {
        ast::TypeDeclBody::Alias(expr) | ast::TypeDeclBody::Newtype(expr) => {
            collect_type_expr_named_refs(expr, &mut refs);
        }
        ast::TypeDeclBody::Record(fields) => {
            for field in fields {
                collect_type_expr_named_refs(&field.ty, &mut refs);
            }
        }
        ast::TypeDeclBody::Sum(variants) => {
            for variant in variants {
                match &variant.payload {
                    ast::SumVariantPayload::None => {}
                    ast::SumVariantPayload::Positional(types) => {
                        for ty in types {
                            collect_type_expr_named_refs(ty, &mut refs);
                        }
                    }
                    ast::SumVariantPayload::Record(fields) => {
                        for field in fields {
                            collect_type_expr_named_refs(&field.ty, &mut refs);
                        }
                    }
                }
            }
        }
    }
    refs
}

fn collect_type_expr_named_refs(expr: &ast::TypeExpr, refs: &mut Vec<String>) {
    match &expr.kind {
        ast::TypeExprKind::Named { name, args } => {
            refs.push(name.clone());
            for arg in args {
                collect_type_expr_named_refs(arg, refs);
            }
        }
        ast::TypeExprKind::Func { params, ret } => {
            for param in params {
                collect_type_expr_named_refs(param, refs);
            }
            collect_type_expr_named_refs(ret, refs);
        }
        ast::TypeExprKind::Nullable { inner } => {
            collect_type_expr_named_refs(inner, refs);
        }
        ast::TypeExprKind::Nil | ast::TypeExprKind::Unit => {}
    }
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

fn parse_playdate_bootstrap_options(
    update_target: Option<&str>,
    preload_specs: &[String],
) -> Result<PlaydateBootstrapOptions, i32> {
    let mut options = PlaydateBootstrapOptions::default();

    if let Some(target) = update_target {
        let target = target.trim();
        if !is_valid_lua_path(target) {
            eprintln!(
                "invalid --playdate-bootstrap-target '{}': expected dot-separated Lua identifiers (for example 'playdate.update' or 'playdate.gameUpdate')",
                target
            );
            return Err(2);
        }
        options.update_target = Some(target.to_string());
    }

    for spec in preload_specs {
        let spec = spec.trim();
        if spec.is_empty() {
            eprintln!("invalid --playdate-bootstrap-preload: value cannot be empty");
            return Err(2);
        }

        let (assign_target, import_path_raw) = match spec.split_once('=') {
            Some((target, path)) => {
                let target = target.trim();
                let path = path.trim();
                if target.is_empty() || path.is_empty() {
                    eprintln!(
                        "invalid --playdate-bootstrap-preload '{}': use 'module/path' or 'lua.path=module/path'",
                        spec
                    );
                    return Err(2);
                }
                if !is_valid_lua_path(target) {
                    eprintln!(
                        "invalid preload assign target '{}': expected dot-separated Lua identifiers",
                        target
                    );
                    return Err(2);
                }
                (Some(target.to_string()), path.to_string())
            }
            None => (None, spec.to_string()),
        };

        let import_path = normalize_bootstrap_import_path(&import_path_raw);
        if import_path.is_empty() {
            eprintln!(
                "invalid --playdate-bootstrap-preload '{}': import path cannot be empty",
                spec
            );
            return Err(2);
        }
        if import_path.contains('\n') || import_path.contains('\r') {
            eprintln!(
                "invalid --playdate-bootstrap-preload '{}': import path must be single-line",
                spec
            );
            return Err(2);
        }
        options.preloads.push(PlaydateBootstrapPreload {
            assign_target,
            import_path,
        });
    }

    Ok(options)
}

fn normalize_bootstrap_import_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.contains('/') {
        trimmed.replace('\\', "/")
    } else if trimmed.contains('.') {
        trimmed.replace('.', "/")
    } else {
        trimmed.to_string()
    }
}

fn is_valid_lua_path(path: &str) -> bool {
    let mut segments = path.split('.');
    let mut seen = false;
    for segment in segments.by_ref() {
        if !is_valid_lua_identifier(segment) {
            return false;
        }
        seen = true;
    }
    seen
}

fn is_valid_lua_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn lua_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}

fn write_playdate_bootstrap(
    out_dir: &Path,
    input: &Path,
    project: &CompiledProject,
    options: &PlaydateBootstrapOptions,
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

    if let Err(problems) = validate_playdate_bootstrap_contract(entry) {
        let module_name = entry_module_name(entry);
        eprintln!(
            "--playdate-bootstrap requires entry module '{}' to export:\n  pub fn init() -> S\n  pub fn update(state: S) -> S\n  pub fn render(state: S) -> Unit",
            module_name
        );
        for problem in problems {
            eprintln!("  - {}", problem);
        }
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

    let mut lua = String::new();
    for (index, preload) in options.preloads.iter().enumerate() {
        let quoted_path = lua_quote(&preload.import_path);
        if let Some(assign_target) = &preload.assign_target {
            lua.push_str(&format!(
                "local __preload_{} = import \"{}\"\n",
                index, quoted_path
            ));
            lua.push_str(&format!(
                "if __preload_{} ~= nil then\n    {} = __preload_{}\nend\n",
                index, assign_target, index
            ));
        } else {
            lua.push_str(&format!("import \"{}\"\n", quoted_path));
        }
    }
    if !options.preloads.is_empty() {
        lua.push('\n');
    }
    let update_target = options
        .update_target
        .as_deref()
        .unwrap_or("playdate.update");
    lua.push_str(&format!(
        "local game = import \"{}\"\nlocal __state = game.init()\n\nfunction {}()\n    __state = game.update(__state)\n    game.render(__state)\nend\n",
        lua_quote(&import_path),
        update_target
    ));
    write_lua_file(&main_path, &lua)?;
    println!("wrote {}", main_path.display());
    Ok(())
}

fn find_public_normal_func<'a>(
    entry: &'a CompiledModule,
    name: &str,
) -> Option<&'a types::FuncInfo> {
    entry.resolved.func_infos.iter().find(|info| {
        matches!(info.vis, ast::Visibility::Public)
            && matches!(info.kind, types::FuncKind::Normal)
            && info.name == name
    })
}

fn validate_playdate_bootstrap_contract(entry: &CompiledModule) -> Result<(), Vec<String>> {
    let mut problems = Vec::new();

    let init = find_public_normal_func(entry, "init");
    let update = find_public_normal_func(entry, "update");
    let render = find_public_normal_func(entry, "render");

    if init.is_none() {
        problems.push("missing `pub fn init() -> S`".to_string());
    }
    if update.is_none() {
        problems.push("missing `pub fn update(state: S) -> S`".to_string());
    }
    if render.is_none() {
        problems.push("missing `pub fn render(state: S) -> Unit`".to_string());
    }

    let Some(init) = init else {
        return Err(problems);
    };
    let Some(update) = update else {
        return Err(problems);
    };
    let Some(render) = render else {
        return Err(problems);
    };

    if !init.params.is_empty() {
        problems.push(format!(
            "`init` must have zero parameters, found {}",
            init.params.len()
        ));
    }
    let state_ty = init.ret.clone();

    if update.params.len() != 1 {
        problems.push(format!(
            "`update` must take exactly one state parameter, found {}",
            update.params.len()
        ));
    } else if update.params[0] != state_ty {
        problems.push(format!(
            "`update` parameter type {:?} does not match init state type {:?}",
            update.params[0], state_ty
        ));
    }
    if update.ret != state_ty {
        problems.push(format!(
            "`update` return type {:?} does not match init state type {:?}",
            update.ret, state_ty
        ));
    }

    if render.params.len() != 1 {
        problems.push(format!(
            "`render` must take exactly one state parameter, found {}",
            render.params.len()
        ));
    } else if render.params[0] != state_ty {
        problems.push(format!(
            "`render` parameter type {:?} does not match init state type {:?}",
            render.params[0], state_ty
        ));
    }
    if render.ret != types::Type::Unit {
        problems.push(format!("`render` must return Unit, found {:?}", render.ret));
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

fn entry_module_name(entry: &CompiledModule) -> String {
    if entry.parsed.module_path.is_empty() {
        "<entry>".to_string()
    } else {
        entry.parsed.module_path.join(".")
    }
}

fn write_lua_file(path: &Path, lua: &str) -> Result<(), i32> {
    if let Some(parent) = path.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        eprintln!(
            "failed to create output directory '{}': {}",
            parent.display(),
            err
        );
        return Err(1);
    }
    if let Err(err) = std::fs::write(path, lua) {
        eprintln!("failed to write '{}': {}", path.display(), err);
        return Err(1);
    }
    Ok(())
}

fn write_project_file(path: PathBuf, contents: &str) -> Result<(), i32> {
    if let Some(parent) = path.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        eprintln!(
            "failed to create project directory '{}': {}",
            parent.display(),
            err
        );
        return Err(1);
    }
    if let Err(err) = std::fs::write(&path, contents) {
        eprintln!("failed to write '{}': {}", path.display(), err);
        return Err(1);
    }
    println!("wrote {}", path.display());
    Ok(())
}

fn sanitize_package_segment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_underscore = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    out.trim_matches('_').to_string()
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
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use crate::{
        codegen_lua, config::ConfigSource, diagnostics::Diagnostics, lexer, parser, resolve,
        source::SourceDb, typecheck,
    };

    use super::{
        PlaydateTemplateWorkflow, ProjectOptions, build_playdate_command_with_overrides,
        check_command, compile_pipeline, compile_pipeline_with_options, emit_lua_command,
        emit_lua_command_with_bootstrap_options, emit_lua_command_with_overrides,
        init_playdate_template_command, parse_playdate_bootstrap_options, resolve_output_path,
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

    fn normalize_line_endings(text: &str) -> String {
        text.replace("\r\n", "\n")
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
        let actual_normalized = normalize_line_endings(&actual);
        let expected_normalized = normalize_line_endings(&expected);

        assert_eq!(
            actual_normalized,
            expected_normalized,
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
        let actual_normalized = normalize_line_endings(&actual);
        let expected_normalized = normalize_line_endings(&expected);

        assert_eq!(
            actual_normalized,
            expected_normalized,
            "lua golden '{}' mismatch",
            golden_path.display()
        );
    }

    #[test]
    fn full_pipeline_compiles_and_emits_lua_for_feature_rich_module() {
        let source = r#"
type Point { x: Int, y: Int }
type MaybeInt = | Missing | Present(Int)

impl MaybeInt {
  fn unwrap_or(self: MaybeInt, fallback: Int) -> Int {
match self {
case Present(v) => v
case Missing => fallback
}
  }
}

fn add(a: Int, b: Int) -> Int {
a + b
}

pub fn main() -> Int {
let p = Point { x = 1, y = 2 }
var total: Int = add(p.x, p.y)
if true {
total = total + 1
} else {
total = total + 2
}
for i in 0..1 {
total = total + i
}
let inc = fn (x: Int) -> Int => x + 1
let m = Present(inc(total))
m.unwrap_or(0)
}
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
    fn parser_accepts_v0_8_brace_block_forms() {
        let source = r#"
extern module foo.bar {
  extern fn get() -> Int
}

type Flag = | On | Off

impl Flag {
  fn value(self: Flag) -> Int {
    match self {
      case On => 1
      case Off => 0
    }
  }
}

fn main(flag: Bool) -> Int {
  var total: Int = 0
  while total < 2 {
    total = total + 1
  }
  for i in 0..1 {
    total = total + i
  }
  if flag {
    total
  } else if total > 1 {
    foo.bar.get()
  } else {
    0
  }
}
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (_, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
    }

    #[test]
    fn parser_rejects_old_block_delimiters_with_migration_codes() {
        let cases = [
            "fn main() -> Int do\n0\nend\n",
            "fn main() -> Int {\nif true then\n1\nelse\n0\nend\n}\n",
            "fn main() -> Int {\nwhile true do\nreturn 0\nend\n0\n}\n",
            "fn main() -> Int {\nfor i in 0..1 do\nreturn i\nend\n0\n}\n",
            "impl Thing do\nfn value(self: Thing) -> Int { 0 }\nend\n",
            "extern module foo.bar do\nextern fn get() -> Int\nend\n",
            "fn main() -> Int {\nmatch true do\ncase true => 1\ncase false => 0\nend\n}\n",
        ];

        for source in cases {
            let (tokens, lex_diags) = lexer::lex(0, source);
            assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
            let (_, parse_diags) = parser::parse(tokens);
            assert!(parse_diags.has_errors(), "{source}");
            assert!(
                parse_diags.items.iter().any(|d| {
                    d.code.as_deref() == Some("CAL-PAR-001") && d.message.contains("use `{ ... }`")
                }),
                "{:?}",
                parse_diags.items
            );
        }
    }

    #[test]
    fn parser_rejects_elseif_with_migration_code() {
        let source = r#"
fn main(flag: Bool) -> Int {
  if flag {
    1
  } elseif true {
    2
  } else {
    0
  }
}
"#;
        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (_, parse_diags) = parser::parse(tokens);
        assert!(parse_diags.has_errors());
        assert!(
            parse_diags.items.iter().any(|d| {
                d.code.as_deref() == Some("CAL-PAR-002") && d.message.contains("else if cond")
            }),
            "{:?}",
            parse_diags.items
        );
    }

    #[test]
    fn typecheck_reports_assignment_to_immutable_parameter() {
        let source = r#"
fn bad(x: Int) -> Int {
x = 2
x
}
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
        std::fs::write(&path, "fn ok() -> Int {\n1\n}\n").expect("failed to write temp file");

        let (_, _, diagnostics, compiled) = compile_pipeline(&path).expect("pipeline failed");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.items);
        assert!(compiled.is_some());

        let missing = path.with_extension("missing.luna");
        assert_eq!(compile_pipeline(&missing).unwrap_err(), 1);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn init_playdate_template_creates_expected_files() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_init_playdate_{}", nonce));

        init_playdate_template_command(&root, PlaydateTemplateWorkflow::AutoBootstrap, false)
            .expect("init playdate template");

        assert!(root.join("callisto.toml").is_file());
        assert!(root.join("README.md").is_file());
        assert!(root.join("Makefile").is_file());
        assert!(root.join("src").join("game.cal").is_file());
        assert!(!root.join("Source").join("main.lua").is_file());
        assert!(root.join("bindings").join("math.cal").is_file());
        assert!(root.join("bindings").join("playdate.cal").is_file());
        assert!(
            root.join("bindings")
                .join("playdate")
                .join("graphics.cal")
                .is_file()
        );
        let config = fs::read_to_string(root.join("callisto.toml")).expect("read config");
        assert!(config.contains("module_roots"), "{config}");
        assert!(config.contains("\"bindings\""), "{config}");
        assert!(!config.contains("../playdate_bindings/src"), "{config}");

        let entry = root.join("src").join("game.cal");
        check_command(&entry, Some(&root.join("callisto.toml")), &[]).expect("check scaffold");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn init_playdate_template_manual_workflow_writes_source_main_shim() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_init_playdate_manual_{}", nonce));

        init_playdate_template_command(&root, PlaydateTemplateWorkflow::ManualShim, false)
            .expect("init playdate template");

        let main_lua = root.join("Source").join("main.lua");
        assert!(main_lua.is_file());
        let text = fs::read_to_string(main_lua).expect("read main.lua");
        assert!(text.contains("local game = import \"game\""), "{text}");
        assert!(text.contains("function playdate.update()"), "{text}");

        let makefile = fs::read_to_string(root.join("Makefile")).expect("read makefile");
        assert!(makefile.contains("build-lua"), "{makefile}");
        assert!(!makefile.contains("--playdate-bootstrap"), "{makefile}");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn init_playdate_template_with_starter_assets_creates_asset_dirs() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_init_playdate_assets_{}", nonce));

        init_playdate_template_command(&root, PlaydateTemplateWorkflow::AutoBootstrap, true)
            .expect("init playdate template");

        assert!(root.join("Source").join("images").join(".keep").is_file());
        assert!(root.join("Source").join("sounds").join(".keep").is_file());
        assert!(root.join("Source").join("fonts").join(".keep").is_file());

        let readme = fs::read_to_string(root.join("README.md")).expect("read README");
        assert!(readme.contains("Starter Assets"), "{readme}");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn init_playdate_template_rejects_non_empty_dir() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_init_nonempty_{}", nonce));
        std::fs::create_dir_all(&root).expect("create root");
        std::fs::write(root.join("existing.txt"), "occupied").expect("write file");

        assert_eq!(
            init_playdate_template_command(&root, PlaydateTemplateWorkflow::AutoBootstrap, false)
                .unwrap_err(),
            1
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn compile_pipeline_omits_compiled_output_when_diagnostics_have_errors() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("callisto_compile_errors_{}.luna", nonce));
        std::fs::write(&path, "fn main() -> Int {\ntrue\n}\n").expect("failed to write temp file");

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
        std::fs::write(&entry, "fn main() -> Int {\n0\n}\n").expect("failed to write entry");
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
        std::fs::write(&entry, "fn main() -> Int {\n0\n}\n").expect("failed to write entry");
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
        std::fs::write(&entry, "fn main() -> Int {\n0\n}\n").expect("failed to write entry");
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
type MaybeInt = | Missing | Present(Int)

fn main() -> Int {
let p = Point { z = 1 }
let m = Present(1, 2)
p.x
}
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
        assert!(type_diags.items.iter().any(|d| {
            d.message.contains("constructor argument count mismatch")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("try `Present(arg1)`"))
        }));
    }

    #[test]
    fn reports_non_exhaustive_match_for_sum_types() {
        let source = r#"
type MaybeInt = | Missing | Present(Int)

fn unwrap(m: MaybeInt) -> Int {
match m {
case Present(v) => v
}
}
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
    fn reports_duplicate_constructor_match_arm() {
        let source = r#"
type MaybeInt = | Missing | Present(Int)

fn main(m: MaybeInt) -> Int {
  match m {
    case Present(x) => x
    case Present(_) => 0
    case Missing => 0
  }
}
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
            d.code.as_deref() == Some("CAL-TYP-031")
                && d.message.contains("duplicate constructor match arm")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("already covered by this earlier arm"))
        }));
    }

    #[test]
    fn reports_unreachable_match_arm_after_catch_all() {
        let source = r#"
fn main(x: Int) -> Int {
  match x {
    case _ => 0
    case 1 => 1
  }
}
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
            d.code.as_deref() == Some("CAL-TYP-032")
                && d.message.contains("unreachable match arm")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("already cover all remaining cases"))
        }));
    }

    #[test]
    fn reports_non_exhaustive_match_for_bool_cases() {
        let source = r#"
fn main(flag: Bool) -> Int {
  match flag {
    case true => 1
  }
}
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
            d.code.as_deref() == Some("CAL-TYP-030")
                && d.message
                    .contains("non-exhaustive match, missing cases: false")
        }));
    }

    #[test]
    fn complete_bool_match_flags_following_arm_as_unreachable() {
        let source = r#"
fn main(flag: Bool) -> Int {
  match flag {
    case true => 1
    case false => 0
    case _ => 2
  }
}
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert_eq!(
            type_diags
                .items
                .iter()
                .filter(|d| d.code.as_deref() == Some("CAL-TYP-032"))
                .count(),
            1
        );
    }

    #[test]
    fn complete_sum_match_flags_following_arm_as_unreachable_without_duplicate_cascade() {
        let source = r#"
type MaybeInt = | Missing | Present(Int)

fn main(m: MaybeInt) -> Int {
  match m {
    case Present(_) => 1
    case Missing => 0
    case Present(x) => x
  }
}
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert_eq!(
            type_diags
                .items
                .iter()
                .filter(|d| d.code.as_deref() == Some("CAL-TYP-032"))
                .count(),
            1
        );
        assert_eq!(
            type_diags
                .items
                .iter()
                .filter(|d| d.code.as_deref() == Some("CAL-TYP-031"))
                .count(),
            0
        );
    }

    #[test]
    fn infers_generic_function_call_type_parameters() {
        let source = r#"
fn id[T](x: T) -> T {
x
}

fn main() -> Int {
id(1)
}
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

extern module foo.bar {
extern fn baz() -> Int
extern fn qux(x: Int) -> Int
}

fn main() -> Int {
bar.baz() + qux(1)
}
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

extern module foo.bar {
extern fn baz() -> Int
extern fn qux(x: Int) -> Int
}

fn main() -> Int {
bar.baz() + qux(1)
}
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

fn main() -> Int {
qux(1)
}
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
                "imported item 'qux' resolves to 'foo.bar.qux' but no matching public function/extern declaration exists",
            )
        }));
        assert!(type_diags.items.iter().any(|d| {
            d.notes
                .iter()
                .any(|(_, note)| note.contains("mark it 'pub'"))
        }));
    }

    #[test]
    fn imported_module_missing_member_reports_clear_error() {
        let source = r#"
import foo.bar

extern module foo.bar {
extern fn baz() -> Int
}

fn main() -> Int {
bar.qux()
}
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
                "unknown imported module function 'foo.bar.qux'; no matching public function/extern declaration was found",
            )
        }));
        assert!(type_diags.items.iter().any(|d| {
            d.notes
                .iter()
                .any(|(_, note)| note.contains("mark it 'pub'"))
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

pub fn add(a: Int, b: Int) -> Int {
  a + b
}
"#,
        )
        .expect("failed to write lib module");

        let entry = root.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app

import lib.math

fn main() -> Int {
  math.add(1, 2)
}
"#,
        )
        .expect("failed to write entry module");

        let (_, _, diagnostics, compiled) = compile_pipeline(&entry).expect("pipeline failed");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.items);
        assert!(compiled.is_some());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn compile_pipeline_importing_private_item_reports_pub_hint() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_private_import_item_{}", nonce));
        let lib_dir = root.join("lib");
        std::fs::create_dir_all(&lib_dir).expect("failed to create temp dirs");

        let lib = lib_dir.join("math.luna");
        std::fs::write(
            &lib,
            r#"
module lib.math

fn hidden(x: Int) -> Int {
  x
}
"#,
        )
        .expect("failed to write lib module");

        let entry = root.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app

import lib.math.{hidden}

fn main() -> Int {
  hidden(1)
}
"#,
        )
        .expect("failed to write entry module");

        let (_, _, diagnostics, compiled) = compile_pipeline(&entry).expect("pipeline failed");
        assert!(diagnostics.has_errors());
        assert!(compiled.is_none());
        assert!(diagnostics.items.iter().any(|d| {
            d.message.contains(
                "imported item 'hidden' resolves to 'lib.math.hidden' but no matching public function/extern declaration exists",
            )
        }));
        assert!(diagnostics.items.iter().any(|d| {
            d.notes
                .iter()
                .any(|(_, note)| note.contains("mark it 'pub'"))
        }));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn compile_pipeline_import_item_type_enables_cross_module_record_usage() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_type_import_items_{}", nonce));
        let lib_dir = root.join("lib");
        std::fs::create_dir_all(&lib_dir).expect("failed to create temp dirs");

        let lib = lib_dir.join("math.luna");
        std::fs::write(
            &lib,
            r#"
module lib.math

pub type Pair { x: Int, y: Int }

pub fn mk() -> Pair {
  Pair { x = 10, y = 32 }
}
"#,
        )
        .expect("failed to write lib module");

        let entry = root.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app

import lib.math.{Pair}

fn main() -> Int {
  let p = Pair { x = 1, y = 2 }
  p.x + math.mk().y
}
"#,
        )
        .expect("failed to write entry module");

        let (_, _, diagnostics, compiled) = compile_pipeline(&entry).expect("pipeline failed");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.items);
        assert!(compiled.is_some());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn compile_pipeline_plain_import_keeps_imported_type_opaque() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_type_import_plain_{}", nonce));
        let lib_dir = root.join("lib");
        std::fs::create_dir_all(&lib_dir).expect("failed to create temp dirs");

        let lib = lib_dir.join("math.luna");
        std::fs::write(
            &lib,
            r#"
module lib.math

pub type Pair { x: Int, y: Int }
"#,
        )
        .expect("failed to write lib module");

        let entry = root.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app

import lib.math

fn main() -> Int {
  let p = Pair { x = 1, y = 2 }
  p.x
}
"#,
        )
        .expect("failed to write entry module");

        let (_, _, diagnostics, compiled) = compile_pipeline(&entry).expect("pipeline failed");
        assert!(diagnostics.has_errors());
        assert!(compiled.is_none());
        assert!(
            diagnostics
                .items
                .iter()
                .any(|d| d.message.contains("type 'Pair' is not a record type"))
        );
        assert!(diagnostics.items.iter().any(|d| {
            d.notes
                .iter()
                .any(|(_, note)| note.contains("import lib.math.{Pair}"))
        }));

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

pub fn value() -> Int {
  true
}
"#,
        )
        .expect("failed to write root_a module");
        std::fs::write(
            root_b.join("foo").join("bar.luna"),
            r#"
module foo.bar

pub fn value() -> Int {
  1
}
"#,
        )
        .expect("failed to write root_b module");

        let entry = entry_dir.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app

import foo.bar

fn main() -> Int {
  bar.value()
}
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

pub fn add(a: Int, b: Int) -> Int {
  true
}
"#,
        )
        .expect("failed to write first root module");
        std::fs::write(
            second_root.join("lib").join("math.luna"),
            r#"
module lib.math

pub fn add(a: Int, b: Int) -> Int {
  a + b
}
"#,
        )
        .expect("failed to write second root module");

        let entry = entry_dir.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app
import lib.math

fn main() -> Int {
  math.add(1, 2)
}
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

pub fn add(a: Int, b: Int) -> Int {
  a + b
}
"#,
        )
        .expect("failed to write lib module");

        let entry = root.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app

import lib.math

fn main() -> Int {
  math.add(1, 2)
}
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

fn main() -> Int {
  0
}
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

pub fn add(a: Int, b: Int) -> Int {
  a + b
}
"#,
        )
        .expect("failed to write lib module");

        let entry = root.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app

import lib.math

pub fn main() -> Int {
  math.add(1, 2)
}
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

pub fn add(a: Int, b: Int) -> Int {
  a + b
}
"#,
        )
        .expect("failed to write shared module");
        let entry = src_dir.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app
import lib.math

pub fn main() -> Int {
  math.add(1, 2)
}
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

pub fn add(a: Int, b: Int) -> Int {
  a + b
}
"#,
        )
        .expect("failed to write shared module");
        let entry = src_dir.join("main.luna");
        std::fs::write(
            &entry,
            r#"
module app
import lib.math

pub fn main() -> Int {
  math.add(1, 2)
}
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
    fn emit_lua_expr_statement_discards_value_with_local_binding() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_emit_expr_stmt_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("main.cal");
        std::fs::write(
            &entry,
            r#"
module app

pub fn tick() -> Unit {
  let x = 1
  x
  ()
}
"#,
        )
        .expect("failed to write entry");

        emit_lua_command_with_overrides(&entry, Some(out_dir.as_path()), None, &[], false)
            .expect("emit failed");

        let module_lua = out_dir.join("app.lua");
        let lua_text = std::fs::read_to_string(&module_lua).expect("read module lua");
        assert!(lua_text.contains("local _ = l0"), "{lua_text}");

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

pub fn init() -> Int {
  0
}

pub fn update(state: Int) -> Int {
  state + 1
}

pub fn render(state: Int) -> Unit {
  state
  ()
}
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
        assert!(
            shim_text.contains("local __state = game.init()"),
            "{shim_text}"
        );
        assert!(
            shim_text.contains("__state = game.update(__state)"),
            "{shim_text}"
        );
        assert!(shim_text.contains("game.render(__state)"), "{shim_text}");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_playdate_bootstrap_supports_custom_target_and_preloads() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("callisto_playdate_bootstrap_customized_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("game.cal");
        std::fs::write(
            &entry,
            r#"
module app.game

pub fn init() -> Int {
  0
}

pub fn update(state: Int) -> Int {
  state + 1
}

pub fn render(state: Int) -> Unit {
  state
  ()
}
"#,
        )
        .expect("failed to write entry");

        let preloads = vec![
            "playdate.input=playdate/input".to_string(),
            "playdate.audio".to_string(),
        ];
        let bootstrap =
            parse_playdate_bootstrap_options(Some("playdate.gameUpdate"), &preloads).unwrap();

        emit_lua_command_with_bootstrap_options(
            &entry,
            Some(out_dir.as_path()),
            None,
            &[],
            Some(&bootstrap),
        )
        .expect("emit failed");

        let shim = out_dir.join("main.lua");
        let shim_text = std::fs::read_to_string(&shim).expect("read shim");
        assert!(
            shim_text.contains("local __preload_0 = import \"playdate/input\""),
            "{shim_text}"
        );
        assert!(
            shim_text.contains("playdate.input = __preload_0"),
            "{shim_text}"
        );
        assert!(
            shim_text.contains("import \"playdate/audio\""),
            "{shim_text}"
        );
        assert!(
            shim_text.contains("function playdate.gameUpdate()"),
            "{shim_text}"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn playdate_bootstrap_rejects_invalid_update_target() {
        let opts = parse_playdate_bootstrap_options(Some("playdate.update()"), &[]);
        assert_eq!(opts.unwrap_err(), 2);
    }

    #[test]
    fn build_playdate_command_emits_source_and_invokes_pdc() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_build_playdate_{}", nonce));
        let src_dir = root.join("src");
        let out_dir = root.join("Source");
        std::fs::create_dir_all(&src_dir).expect("create src");

        let entry = src_dir.join("game.cal");
        std::fs::write(
            &entry,
            r#"
module game

import playdate.graphics

type State {
  frames: Int
}

pub fn init() -> State {
  State { frames = 0 }
}

pub fn update(state: State) -> State {
  state with { frames = state.frames + 1 }
}

pub fn render(state: State) -> Unit {
  graphics.clear()
  ()
}
"#,
        )
        .expect("write entry");

        let (pdc_script, pdc_script_contents) = if cfg!(windows) {
            (
                root.join("fake-pdc.cmd"),
                "@echo off\r\nif not exist \"%~1\" exit /b 9\r\ntype nul > \"%~2\"\r\n",
            )
        } else {
            (
                root.join("fake-pdc.sh"),
                "#!/bin/sh\nsrc=\"$1\"\nout=\"$2\"\n[ -d \"$src\" ] || exit 9\ntouch \"$out\"\n",
            )
        };
        std::fs::write(&pdc_script, pdc_script_contents).expect("write script");
        #[cfg(unix)]
        {
            let perms = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(&pdc_script, perms).expect("chmod script");
        }

        let pdx_path = root.join("Game.pdx");
        let bindings_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("playdate_bindings")
            .join("src");
        build_playdate_command_with_overrides(
            &entry,
            Some(out_dir.as_path()),
            Some(pdx_path.as_path()),
            pdc_script.to_str(),
            false,
            None,
            std::slice::from_ref(&bindings_root),
        )
        .expect("build playdate");

        assert!(
            out_dir.join("main.lua").is_file(),
            "missing generated main.lua"
        );
        assert!(out_dir.join("game.lua").is_file(), "missing game.lua");
        assert!(pdx_path.is_file(), "missing pdx output");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_playdate_bootstrap_requires_stateful_contract() {
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

pub fn tick() -> Unit {
  ()
}
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
    fn emit_lua_playdate_bootstrap_rejects_update_with_wrong_arity() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("callisto_playdate_bootstrap_wrong_arity_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("game.cal");
        std::fs::write(
            &entry,
            r#"
module app.game

pub fn init() -> Int {
  0
}

pub fn update() -> Int {
  1
}

pub fn render(state: Int) -> Unit {
  state
  ()
}
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
    fn emit_lua_playdate_bootstrap_rejects_state_type_mismatch() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "callisto_playdate_bootstrap_state_mismatch_{}",
            nonce
        ));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("game.cal");
        std::fs::write(
            &entry,
            r#"
module app.game

pub fn init() -> Int {
  0
}

pub fn update(state: Int) -> Int {
  state + 1
}

pub fn render(state: Float) -> Unit {
  state
  ()
}
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

pub fn poll() -> Bool {
  input.a_pressed()
}
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
        assert!(
            input_text.contains("playdate.buttonIsPressed"),
            "{input_text}"
        );
        assert!(
            input_text.contains("M.a_pressed = a_pressed"),
            "{input_text}"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_playdate_graphics_binding_emits_shape_paths() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_playdate_graphics_emit_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("main.cal");
        std::fs::write(
            &entry,
            r#"
module app

import playdate.graphics

pub fn render() -> Unit {
  graphics.drawLine(10.0, 20.0, 30.0, 40.0)
  graphics.drawRect(12.5, 22.0, 40.0, 12.0)
  graphics.fillRect(14.0, 24.0, 36.0, 8.0)
  ()
}
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

        let game_lua = out_dir.join("app.lua");
        let game_text = std::fs::read_to_string(&game_lua).expect("read game lua");
        assert!(
            game_text.contains("playdate.graphics.drawLine(10.0, 20.0, 30.0, 40.0)"),
            "{game_text}"
        );
        assert!(
            game_text.contains("playdate.graphics.drawRect(12.5, 22.0, 40.0, 12.0)"),
            "{game_text}"
        );
        assert!(
            game_text.contains("playdate.graphics.fillRect(14.0, 24.0, 36.0, 8.0)"),
            "{game_text}"
        );
        assert!(game_text.contains("M.render = render"), "{game_text}");

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

pub fn cue() -> Unit {
  audio.bounce_blip()
}
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
        assert!(
            audio_text.contains("playdate.sound.playNote"),
            "{audio_text}"
        );
        assert!(
            audio_text.contains("M.bounce_blip = bounce_blip"),
            "{audio_text}"
        );

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

pub fn right_half() -> Bool {
  system.crank_is_right_half()
}
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
        assert!(
            system_text.contains("playdate.getCrankPosition"),
            "{system_text}"
        );
        assert!(
            system_text.contains("M.crank_is_right_half = crank_is_right_half"),
            "{system_text}"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_playdate_timer_binding_emits_update_timers_paths() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_playdate_timer_emit_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("main.cal");
        std::fs::write(
            &entry,
            r#"
module app

import playdate.timer

pub fn tick() -> Unit {
  timer.updateTimers()
}
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

        let game_lua = out_dir.join("app.lua");
        let timer_text = std::fs::read_to_string(&game_lua).expect("read game lua");
        assert!(
            timer_text.contains("playdate.timer.updateTimers()"),
            "{timer_text}"
        );
        assert!(timer_text.contains("M.tick = tick"), "{timer_text}");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emit_lua_math_binding_emits_sin_path() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_math_binding_emit_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("main.cal");
        std::fs::write(
            &entry,
            r#"
module app

import math

pub fn wave(theta: Float) -> Float {
  math.sin(theta)
}
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

        let game_lua = out_dir.join("app.lua");
        let game_text = std::fs::read_to_string(&game_lua).expect("read game lua");
        assert!(game_text.contains("math.sin("), "{game_text}");

        let math_lua = out_dir.join("math.lua");
        let math_text = std::fs::read_to_string(&math_lua).expect("read math lua");
        assert!(math_text.contains("local M = {}"), "{math_text}");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emitted_lua_bootstraps_imported_project_modules_for_qualified_calls() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("callisto_import_bootstrap_{}", nonce));
        let out_dir = root.join("out");
        std::fs::create_dir_all(&root).expect("failed to create root");

        let entry = root.join("game.cal");
        std::fs::write(
            &entry,
            r#"
module game

import core.{State}

pub fn init() -> State {
  State { frame = core.next_frame() }
}
"#,
        )
        .expect("failed to write entry");
        std::fs::write(
            root.join("core.cal"),
            r#"
module core

pub type State { frame: Int }

pub fn next_frame() -> Int {
  1
}
"#,
        )
        .expect("failed to write core");

        emit_lua_command_with_overrides(&entry, Some(out_dir.as_path()), None, &[], false)
            .expect("emit failed");

        let game_lua = out_dir.join("game.lua");
        let game_text = std::fs::read_to_string(&game_lua).expect("read game lua");
        assert!(game_text.contains("pcall(import, \"core\")"), "{game_text}");
        assert!(
            game_text.contains("core = core or __import_mod_0"),
            "{game_text}"
        );
        assert!(game_text.contains("core.next_frame()"), "{game_text}");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn nullary_constructor_pattern_is_not_lowered_as_bind() {
        let source = r#"
type MaybeInt = | Missing | Present(Int)

fn pick(m: MaybeInt) -> Int {
match m {
case Missing => 0
case Present(v) => v
}
}
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
        assert!(lua.contains("__scrutinee.tag == \"Missing\""), "{lua}");
    }

    #[test]
    fn parses_multiline_and_single_line_sum_declarations() {
        let source = r#"
type Maybe[T] =
  | Missing
  | Present(T)

type Status = | Idle | Busy

fn choose(flag: Bool) -> Maybe[Int] {
  if flag {
    Present(1)
  } else {
    Missing
  }
}

fn status() -> Status {
  Idle
}
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
    fn parser_accepts_trailing_commas_and_record_field_punning() {
        let source = r#"
type Maybe[T,] =
  | Missing
  | Present(T,)

type Point {
  x: Int,
  y: Int,
}

fn add(
  a: Int,
  b: Int,
) -> Int {
  a + b
}

fn from_opt(v: Maybe[Int]) -> Int {
  match v {
    case Present(x,) => x,
    case Missing => 0,
  }
}

fn main(x: Int) -> Int {
  let base = add(
    1,
    2,
  )
  let p = Point {
    x,
    y = base,
  }
  match Present(p.x) {
    case Present(v,) => v,
    case Missing => 0,
  }
}
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
    fn prelude_option_is_available_without_local_declaration() {
        let source = r#"
fn choose(flag: Bool) -> Option[Int] {
  if flag {
    Some(1)
  } else {
    None
  }
}

fn main(flag: Bool) -> Int {
  match choose(flag) {
    case Some(v) => v
    case None => 0
  }
}
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
        assert!(lua.contains("tag = \"None\""), "{lua}");
    }

    #[test]
    fn reserved_prelude_names_are_rejected() {
        let source = r#"
type Option[T] = | Empty
type Maybe = | Some(Int)
fn map(x: Int) -> Int {
  x
}
fn append(x: Int) -> Int {
  x
}
fn filter(x: Int) -> Int {
  x
}
fn fold(x: Int) -> Int {
  x
}
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (_, resolve_diags) = resolve::resolve(&ast);
        assert!(resolve_diags.has_errors());
        assert_eq!(
            resolve_diags
                .items
                .iter()
                .filter(|d| d.code.as_deref() == Some("CAL-RES-070"))
                .count(),
            6
        );
    }

    #[test]
    fn list_literals_length_and_map_compile_to_lua_arrays() {
        let source = r#"
fn main() -> Int {
  let xs: List[Int] = [1, 2, 3]
  let ys = map(xs, fn (x: Int) -> Int => x + 1)
  length(ys)
}
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
        assert!(lua.contains("{ 1, 2, 3 }"), "{lua}");
        assert!(lua.contains("ipairs(__list)"), "{lua}");
        assert!(lua.contains("return #"), "{lua}");
    }

    #[test]
    fn list_index_append_filter_and_fold_compile_to_lua_arrays() {
        let source = r#"
fn main() -> Int {
  let xs: List[Int] = [1, 2, 3]
  let ys = append(xs, 4)
  let zs = filter(ys, fn (x: Int) -> Bool => x > 1)
  fold(zs, zs[1], fn (acc: Int, x: Int) -> Int => acc + x)
}
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
        assert!(lua.contains("__out[#__out + 1] = __value"), "{lua}");
        assert!(lua.contains("if __predicate(__v)"), "{lua}");
        assert!(lua.contains("__acc = __reducer(__acc, __v)"), "{lua}");
        assert!(lua.contains("[1]"), "{lua}");
    }

    #[test]
    fn empty_list_literals_require_context() {
        let ok_source = r#"
fn main() -> Int {
  let xs: List[Int] = []
  length(xs)
}
"#;
        let (tokens, lex_diags) = lexer::lex(0, ok_source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);

        let err_source = r#"
fn main() -> Unit {
  let xs = []
  ()
}
"#;
        let (tokens, lex_diags) = lexer::lex(0, err_source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.items.iter().any(|d| {
            d.code.as_deref() == Some("CAL-TYP-025")
                && d.message
                    .contains("cannot infer type of empty list literal")
        }));
    }

    #[test]
    fn list_literal_rejects_incompatible_elements() {
        let source = r#"
fn main() -> Unit {
  let xs = [1, true]
  ()
}
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
                .contains("incompatible list literal element types")
        }));
    }

    #[test]
    fn list_helpers_report_arity_and_argument_errors() {
        let source = r#"
fn main() -> Unit {
  let xs: List[Int] = [1]
  let a = length(xs, xs)
  let b = map(1, fn (x: Int) -> Int => x)
  ()
}
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
            d.code.as_deref() == Some("CAL-TYP-011")
                && d.message.contains("length expects 1 argument")
        }));
        assert!(type_diags.items.iter().any(|d| {
            d.code.as_deref() == Some("CAL-TYP-012")
                && d.message.contains("map expects first argument List[T]")
        }));
    }

    #[test]
    fn record_field_punning_all_shorthand_compiles_and_codegen() {
        let source = r#"
type Pair { a: Int, b: Int }

fn make(a: Int, b: Int) -> Pair {
  Pair { a, b }
}

fn swap(p: Pair) -> Pair {
  let a = p.b
  let b = p.a
  Pair { a, b }
}
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
        // Both fields emitted as locals, not as literal field names
        assert!(lua.contains("a ="), "{lua}");
        assert!(lua.contains("b ="), "{lua}");
    }

    #[test]
    fn parser_accepts_newtype_declarations() {
        let source = r#"
newtype UserId = Int
newtype Box[T] = T

fn make() -> UserId {
  UserId(1)
}
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
    fn malformed_newtype_reports_parser_error() {
        let source = r#"
newtype UserId Int

fn main() -> Int {
  0
}
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (_, parse_diags) = parser::parse(tokens);
        assert!(parse_diags.has_errors());
        assert!(
            parse_diags
                .items
                .iter()
                .any(|d| { d.message.contains("expected '=' in newtype declaration") })
        );
    }

    #[test]
    fn malformed_multiline_sum_reports_parser_error() {
        let source = r#"
type Maybe[T] =
  |

fn main() -> Int {
  0
}
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (_, parse_diags) = parser::parse(tokens);
        assert!(parse_diags.has_errors());
        assert!(
            parse_diags
                .items
                .iter()
                .any(|d| d.message.contains("expected variant name")),
            "{:?}",
            parse_diags.items
        );
    }

    #[test]
    fn functions_are_predeclared_for_forward_references() {
        let source = r#"
fn main() -> Int {
helper()
}

fn helper() -> Int {
1
}
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

fn area(s: Shape) -> Int {
match s {
case Circle { radius } => radius
}
}
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

fn main() -> Int {
0
}
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
fn main(x: Int not, y: Nil) -> Int {
1
}
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
type MaybeInt = | Missing | Present(Int)

fn main() -> Int {
let x = Missing(1)
0
}
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
    fn constructor_pattern_record_payload_shape_mismatch_is_non_cascading() {
        let source = r#"
type MoveMsg = | Move(Int, Int)

fn main(msg: MoveMsg) -> Int {
  match msg {
    case Move { x, y } => x
  }
}
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert_eq!(
            type_diags
                .items
                .iter()
                .filter(|d| d.code.as_deref() == Some("CAL-TYP-023"))
                .count(),
            1
        );
        assert!(type_diags.items.iter().any(|d| {
            d.code.as_deref() == Some("CAL-TYP-023")
                && d.message
                    .contains("constructor pattern requires positional arguments")
        }));
        assert!(!type_diags.items.iter().any(|d| {
            d.message
                .contains("constructor pattern argument count mismatch")
        }));
    }

    #[test]
    fn constructor_pattern_positional_payload_shape_mismatch_is_non_cascading() {
        let source = r#"
type Shape = | Rect { w: Int, h: Int }

fn main(s: Shape) -> Int {
  match s {
    case Rect(w, h) => w
  }
}
"#;

        let (tokens, lex_diags) = lexer::lex(0, source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert_eq!(
            type_diags
                .items
                .iter()
                .filter(|d| d.code.as_deref() == Some("CAL-TYP-023"))
                .count(),
            1
        );
        assert!(type_diags.items.iter().any(|d| {
            d.code.as_deref() == Some("CAL-TYP-023")
                && d.message
                    .contains("constructor pattern requires record payload")
        }));
        assert!(!type_diags.items.iter().any(|d| {
            d.message
                .contains("missing field 'w' in constructor pattern")
                || d.message
                    .contains("missing field 'h' in constructor pattern")
        }));
    }

    #[test]
    fn constructor_pattern_arity_mismatch_has_fixit_shape() {
        let source = r#"
type MoveMsg = | Move(Int, Int)

fn main(msg: MoveMsg) -> Int {
  match msg {
    case Move(x) => x
  }
}
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
            d.code.as_deref() == Some("CAL-TYP-023")
                && d.message
                    .contains("constructor pattern argument count mismatch")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("try `Move(_, _)`"))
        }));
    }

    #[test]
    fn constructor_pattern_record_field_typos_have_fixit_and_code() {
        let source = r#"
type Shape = | Rect { width: Int, height: Int }

fn main(s: Shape) -> Int {
  match s {
    case Rect { widht } => widht
  }
}
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
            d.code.as_deref() == Some("CAL-TYP-024")
                && d.message
                    .contains("unknown field 'widht' in constructor pattern")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("did you mean 'width'"))
        }));
    }

    #[test]
    fn record_update_reports_unknown_and_mistyped_fields() {
        let source = r#"
type Point { x: Int, y: Int }

fn main() -> Int {
let p = Point { x = 1, y = 2 } with { x = true, z = 3 }
p.x
}
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
        assert!(!type_diags.items.iter().any(|d| {
            d.message.contains("unknown field 'z' in record update")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("did you mean"))
        }));
    }

    #[test]
    fn unknown_record_field_reports_did_you_mean_fixit() {
        let source = r#"
type Point { x: Int, y: Int }

fn main() -> Int {
  let p = Point { xx = 1, y = 2 }
  p.x
}
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
                .contains("unknown field 'xx' in record initializer")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("did you mean 'x'"))
        }));
    }

    #[test]
    fn duplicate_record_field_reports_fixit_note() {
        let source = r#"
type Point { x: Int, y: Int }

fn main() -> Int {
  let p = Point { x = 1, x = 2, y = 3 }
  p.y
}
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
                .contains("duplicate field 'x' in record initializer")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("remove one of the duplicate 'x' fields"))
        }));
    }

    #[test]
    fn missing_record_field_reports_fixit_note() {
        let source = r#"
type Point { x: Int, y: Int }

fn main() -> Int {
  let p = Point { x = 1 }
  p.x
}
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
                .contains("missing field 'y' in record initializer")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("add `y = ...`"))
        }));
    }

    #[test]
    fn record_update_codegen_copies_base_before_overrides() {
        let source = r#"
type Point { x: Int, y: Int }

fn bump(p: Point) -> Point {
p with { x = p.x + 1 }
}
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
type Box[T] { value: T }

fn opt() -> Option[Int] {
Some(1)
}

fn boxify() -> Box[Int] {
Box { value = 1 }
}

fn unbox(b: Box[Int]) -> Int {
b.value
}

fn main() -> Int {
let b = boxify()
let b2 = b with { value = 2 }
match opt() {
case Some(v) => unbox(b2) + v
case None => 0
}
}
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
fn takes(v: Option[Int]) -> Int {
  match v {
    case Some(x) => x
    case None => 0
  }
}

fn main() -> Int {
  let a: Option[Int] = None
  takes(None) + takes(a)
}
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
    fn nullary_generic_constructor_infers_in_record_initializer_field_context() {
        let source = r#"
type Wrapper { value: Option[Int] }

fn make() -> Wrapper {
  Wrapper { value = None }
}

fn main() -> Int {
  match make().value {
    case Some(v) => v
    case None => 0
  }
}
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
    fn nullary_generic_constructor_infers_in_constructor_payload_context() {
        let source = r#"
type Wrapped = | Wrapped(Option[Int])

fn make() -> Wrapped {
  Wrapped(None)
}

fn main() -> Int {
  match make() {
    case Wrapped(Some(v)) => v
    case Wrapped(None) => 0
  }
}
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
    fn nullary_generic_constructor_infers_in_record_update_field_context() {
        let source = r#"
type State { value: Option[Int] }

fn clear(s: State) -> State {
  s with { value = None }
}
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
fn main() -> Unit {
  let x = None
  ()
}
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
    fn type_aliases_compile_and_codegen_like_underlying_types() {
        let source = r#"
type Distance = Int

fn inc(d: Distance) -> Distance {
  d + 1
}

fn main() -> Distance {
  inc(41)
}
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
        assert!(lua.contains("inc = function(d)"), "{lua}");
        assert!(lua.contains("return inc(41)"), "{lua}");
    }

    #[test]
    fn transparent_aliases_work_for_assignability_and_control_flow() {
        let source = r#"
type Distance = Int
type Flag = Bool
type Id[T] = T
type IntOpt = Option[Int]

fn choose(flag: Flag) -> Id[Int] {
  if flag {
    1
  } else {
    2
  }
}

fn pick(flag: Flag) -> IntOpt {
  if flag {
    Some(choose(flag))
  } else {
    None
  }
}

fn len(d: Distance) -> Distance {
  d + 1
}

fn main(flag: Flag) -> Distance {
  let base: Distance = 41
  let out: Id[Int] = choose(flag)
  let chosen: IntOpt = pick(flag)
  match chosen {
    case Some(v) => len(base + v)
    case None => len(base + out)
  }
}
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
    fn newtype_constructor_compiles_and_codegen_is_zero_overhead() {
        let source = r#"
newtype UserId = Int

fn make() -> UserId {
  UserId(42)
}

fn same(a: UserId, b: UserId) -> Bool {
  a == b
}

fn main() -> Int {
  let a = make()
  let b = UserId(7)
  if same(a, b) {
    1
  } else {
    0
  }
}
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
        assert!(lua.contains("return 42"), "{lua}");
        assert!(lua.contains("local l"), "{lua}");
        assert!(!lua.contains("tag = \"UserId\""), "{lua}");
    }

    #[test]
    fn newtype_is_not_assignable_from_underlying_type() {
        let source = r#"
newtype UserId = Int

fn takes_user(id: UserId) -> Int {
  0
}

fn main() -> Int {
  takes_user(1)
}
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
            d.message.contains("argument 1 expects") && d.message.contains("but got Int")
        }));
    }

    #[test]
    fn generic_newtype_infers_from_payload_but_errors_when_unconstrained() {
        let ok_source = r#"
newtype Box[T] = T

fn make() -> Box[Int] {
  Box(1)
}

fn main() -> Unit {
  let a: Box[Int] = Box(2)
  let b = make()
  let _ = a == b
  ()
}
"#;

        let (tokens, lex_diags) = lexer::lex(0, ok_source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(!type_diags.has_errors(), "{:?}", type_diags.items);

        let bad_source = r#"
newtype Phantom[T] = Int

fn main() -> Unit {
  let p = Phantom(1)
  ()
}
"#;

        let (tokens, lex_diags) = lexer::lex(0, bad_source);
        assert!(!lex_diags.has_errors(), "{:?}", lex_diags.items);
        let (ast, parse_diags) = parser::parse(tokens);
        assert!(!parse_diags.has_errors(), "{:?}", parse_diags.items);
        let (resolved, resolve_diags) = resolve::resolve(&ast);
        assert!(!resolve_diags.has_errors(), "{:?}", resolve_diags.items);
        let (_, type_diags) = typecheck::typecheck_and_lower(&resolved);
        assert!(type_diags.has_errors());
        assert!(type_diags.items.iter().any(|d| {
            d.message.contains("could not infer generic type parameter")
                && d.message.contains("newtype 'Phantom'")
        }));
    }

    #[test]
    fn newtype_record_initializer_form_reports_constructor_hint() {
        let source = r#"
newtype UserId = Int

fn main() -> Unit {
  let x = UserId { value = 1 }
  ()
}
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
                .contains("type 'UserId' is a newtype, not a record type")
                && d.notes
                    .iter()
                    .any(|(_, note)| note.contains("construct it with `UserId(value)`"))
        }));
    }

    #[test]
    fn generic_record_constructor_without_type_context_reports_inference_failure() {
        let source = r#"
type Phantom[T] { value: Int }

fn main() -> Unit {
  let p = Phantom { value = 1 }
  ()
}
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

fn takes_id(x: Id[Int]) -> Int {
  x
}

fn main() -> Int {
  takes_id(true)
}
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

extern module foo.bar {
  extern fn baz(x: Int) -> Int
}

fn main() -> Int {
  bar(1)
}
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
fn unwrap(v: Option[Int]) -> Int {
  match v {
    case Some(x) => x
  }
}
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
type MaybeInt = | Missing | Present(Int)

fn main() -> Int {
  let x = Missing(1)
  0
}
"#;
        assert_diagnostics_golden(
            "constructor_payload_note",
            "golden_constructor_payload.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_constructor_pattern_shape_mismatch() {
        let source = r#"
type MoveMsg = | Move(Int, Int)

fn main(msg: MoveMsg) -> Int {
  match msg {
    case Move { x, y } => x
  }
}
"#;
        assert_diagnostics_golden(
            "constructor_pattern_shape_mismatch",
            "golden_constructor_pattern_shape_mismatch.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_constructor_pattern_arity_mismatch() {
        let source = r#"
type MoveMsg = | Move(Int, Int)

fn main(msg: MoveMsg) -> Int {
  match msg {
    case Move(x) => x
  }
}
"#;
        assert_diagnostics_golden(
            "constructor_pattern_arity_mismatch",
            "golden_constructor_pattern_arity_mismatch.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_constructor_pattern_field_typo() {
        let source = r#"
type Shape = | Rect { width: Int, height: Int }

fn main(s: Shape) -> Int {
  match s {
    case Rect { widht } => widht
  }
}
"#;
        assert_diagnostics_golden(
            "constructor_pattern_field_typo",
            "golden_constructor_pattern_field_typo.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_imported_module_member_missing() {
        let source = r#"
import foo.bar

extern module foo.bar {
  extern fn baz() -> Int
}

fn main() -> Int {
  bar.qux()
}
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
fn main() -> Int {
  missing_name
}
"#;
        assert_diagnostics_golden("unresolved_name", "golden_unresolved_name.luna", source);
    }

    #[test]
    fn diagnostics_golden_non_exhaustive_match() {
        let source = r#"
type MaybeInt = | Missing | Present(Int)

fn unwrap(m: MaybeInt) -> Int {
  match m {
    case Present(v) => v
  }
}
"#;
        assert_diagnostics_golden(
            "non_exhaustive_match",
            "golden_non_exhaustive_match.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_bool_non_exhaustive_match() {
        let source = r#"
fn main(flag: Bool) -> Int {
  match flag {
    case true => 1
  }
}
"#;
        assert_diagnostics_golden(
            "bool_non_exhaustive_match",
            "golden_bool_non_exhaustive_match.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_duplicate_constructor_match_arm() {
        let source = r#"
type MaybeInt = | Missing | Present(Int)

fn main(m: MaybeInt) -> Int {
  match m {
    case Present(v) => v
    case Present(_) => 0
    case Missing => 0
  }
}
"#;
        assert_diagnostics_golden(
            "duplicate_constructor_match_arm",
            "golden_duplicate_constructor_match_arm.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_unreachable_match_arm() {
        let source = r#"
fn main(value: Int) -> Int {
  match value {
    case _ => 0
    case 1 => 1
  }
}
"#;
        assert_diagnostics_golden(
            "unreachable_match_arm",
            "golden_unreachable_match_arm.luna",
            source,
        );
    }

    #[test]
    fn diagnostics_golden_duplicate_import_alias() {
        let source = r#"
import foo.bar
import baz.bar

fn main() -> Int {
  0
}
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

fn main() -> Int {
  qux(1)
}
"#;
        assert_diagnostics_golden(
            "imported_item_missing_declaration",
            "golden_import_item_missing.luna",
            source,
        );
    }

    #[test]
    fn int_argument_is_assignable_to_float_parameter() {
        let source = r#"
extern fn wave(theta: Float) -> Float

fn main() -> Float {
  wave(1)
}
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
    fn lua_golden_record_update() {
        let source = r#"
type Point { x: Int, y: Int }

fn bump(p: Point) -> Point {
  p with { x = p.x + 1 }
}
"#;
        assert_lua_golden("record_update", "golden_record_update.luna", source);
    }

    #[test]
    fn lua_golden_sum_match() {
        let source = r#"
type MaybeInt = | Missing | Present(Int)

fn pick(m: MaybeInt) -> Int {
  match m {
    case Present(v) => v
    case Missing => 0
  }
}
"#;
        assert_lua_golden("sum_match", "golden_sum_match.luna", source);
    }

    #[test]
    fn lua_golden_string_interpolation() {
        let source = r#"
fn banner(name: String, count: Int) -> String {
  "Hello ${name}! #${count + 1} \${literal}"
}
"#;
        assert_lua_golden(
            "string_interpolation",
            "golden_string_interpolation.luna",
            source,
        );
    }
}
