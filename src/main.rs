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

use std::path::{Path, PathBuf};

use cli::{Cli, Command};
use diagnostics::Diagnostics;
use source::SourceDb;

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
    let (sources, ast, diagnostics, compiled) = compile_pipeline(input)?;
    if !diagnostics.is_empty() {
        eprint!("{}", diagnostics.render(&sources));
    }
    if diagnostics.has_errors() {
        return Err(1);
    }

    let (resolved, tir) = compiled.expect("compiled output present");
    let lua = codegen_lua::emit_lua_module(&tir, &resolved);

    let output_path = resolve_output_path(output, input, &ast);
    if let Some(parent) = output_path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            eprintln!(
                "failed to create output directory '{}': {}",
                parent.display(),
                err
            );
            return Err(1);
        }
    }
    if let Err(err) = std::fs::write(&output_path, lua) {
        eprintln!("failed to write '{}': {}", output_path.display(), err);
        return Err(1);
    }

    println!("wrote {}", output_path.display());
    Ok(())
}

fn build_command(input: &Path, output: Option<&Path>) -> Result<(), i32> {
    emit_lua_command(input, output)
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
    let mut sources = SourceDb::new();
    let file_id = match sources.load_file(input) {
        Ok(id) => id,
        Err(err) => {
            eprintln!("failed to read '{}': {}", input.display(), err);
            return Err(1);
        }
    };
    let source = sources.get(file_id).expect("source exists");

    let (tokens, mut diagnostics) = lexer::lex(file_id, &source.text);
    let (ast, parse_diags) = parser::parse(tokens);
    diagnostics.extend(parse_diags);

    let (resolved, resolve_diags) = resolve::resolve(&ast);
    diagnostics.extend(resolve_diags);

    let (tir, type_diags) = typecheck::typecheck_and_lower(&resolved);
    diagnostics.extend(type_diags);

    let compiled = if diagnostics.has_errors() {
        None
    } else {
        Some((resolved, tir))
    };

    Ok((sources, ast, diagnostics, compiled))
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

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{codegen_lua, lexer, parser, resolve, typecheck};

    use super::{compile_pipeline, resolve_output_path};

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
}
