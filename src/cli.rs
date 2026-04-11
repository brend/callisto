use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Cli {
    pub command: Command,
}

#[derive(Debug, Clone)]
pub enum Command {
    Parse {
        input: PathBuf,
    },
    InitPlaydate {
        dir: PathBuf,
    },
    Check {
        input: PathBuf,
        config: Option<PathBuf>,
        module_roots: Vec<PathBuf>,
    },
    EmitLua {
        input: PathBuf,
        output: Option<PathBuf>,
        config: Option<PathBuf>,
        module_roots: Vec<PathBuf>,
        playdate_bootstrap: bool,
    },
    Build {
        input: PathBuf,
        output: Option<PathBuf>,
        config: Option<PathBuf>,
        module_roots: Vec<PathBuf>,
        playdate_bootstrap: bool,
    },
    BuildPlaydate {
        input: PathBuf,
        source_dir: Option<PathBuf>,
        pdx: Option<PathBuf>,
        pdc: Option<String>,
        run: bool,
        config: Option<PathBuf>,
        module_roots: Vec<PathBuf>,
    },
}

impl Cli {
    pub fn parse_from_env() -> Result<Self, String> {
        Self::parse_from_args(std::env::args().skip(1))
    }

    fn parse_from_args<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let mut args = args.into_iter().map(Into::into);
        let Some(cmd) = args.next() else {
            return Err(usage());
        };

        let command = match cmd.as_str() {
            "parse" => {
                let input = args.next().ok_or_else(usage)?;
                Command::Parse {
                    input: PathBuf::from(input),
                }
            }
            "init" => {
                let mut template = None;
                let mut dir = None;
                while let Some(arg) = args.next() {
                    match arg.as_str() {
                        "--template" => {
                            let value = args
                                .next()
                                .ok_or_else(|| "expected name after --template".to_string())?;
                            if template.is_some() {
                                return Err("duplicate --template flag".to_string());
                            }
                            template = Some(value);
                        }
                        _ => {
                            if dir.is_some() {
                                return Err(format!("unexpected argument '{}'", arg));
                            }
                            dir = Some(PathBuf::from(arg));
                        }
                    }
                }

                let template = template.ok_or_else(|| "missing --template flag".to_string())?;
                let dir = dir.ok_or_else(|| "missing output directory for init".to_string())?;
                if template != "playdate" {
                    return Err(format!(
                        "unknown template '{}'; supported templates: playdate",
                        template
                    ));
                }
                Command::InitPlaydate { dir }
            }
            "check" => {
                let input = args.next().ok_or_else(usage)?;
                let mut config = None;
                let mut module_roots = Vec::new();
                parse_common_flags(&mut args, &mut config, &mut module_roots)?;
                Command::Check {
                    input: PathBuf::from(input),
                    config,
                    module_roots,
                }
            }
            "emit-lua" => {
                let input = args.next().ok_or_else(usage)?;
                let mut output = None;
                let mut config = None;
                let mut module_roots = Vec::new();
                let mut playdate_bootstrap = false;
                while let Some(flag) = args.next() {
                    match flag.as_str() {
                        "-o" => {
                            let path = args
                                .next()
                                .ok_or_else(|| "expected path after -o".to_string())?;
                            output = Some(PathBuf::from(path));
                        }
                        "--config" => {
                            let path = args
                                .next()
                                .ok_or_else(|| "expected path after --config".to_string())?;
                            if config.is_some() {
                                return Err("duplicate --config flag".to_string());
                            }
                            config = Some(PathBuf::from(path));
                        }
                        "--module-root" => {
                            let path = args
                                .next()
                                .ok_or_else(|| "expected path after --module-root".to_string())?;
                            module_roots.push(PathBuf::from(path));
                        }
                        "--playdate-bootstrap" => {
                            playdate_bootstrap = true;
                        }
                        _ => return Err(format!("unknown flag '{}'", flag)),
                    }
                }
                Command::EmitLua {
                    input: PathBuf::from(input),
                    output,
                    config,
                    module_roots,
                    playdate_bootstrap,
                }
            }
            "build" => {
                let input = args.next().ok_or_else(usage)?;
                let mut output = None;
                let mut config = None;
                let mut module_roots = Vec::new();
                let mut playdate_bootstrap = false;
                while let Some(flag) = args.next() {
                    match flag.as_str() {
                        "-o" => {
                            let path = args
                                .next()
                                .ok_or_else(|| "expected path after -o".to_string())?;
                            output = Some(PathBuf::from(path));
                        }
                        "--config" => {
                            let path = args
                                .next()
                                .ok_or_else(|| "expected path after --config".to_string())?;
                            if config.is_some() {
                                return Err("duplicate --config flag".to_string());
                            }
                            config = Some(PathBuf::from(path));
                        }
                        "--module-root" => {
                            let path = args
                                .next()
                                .ok_or_else(|| "expected path after --module-root".to_string())?;
                            module_roots.push(PathBuf::from(path));
                        }
                        "--playdate-bootstrap" => {
                            playdate_bootstrap = true;
                        }
                        _ => return Err(format!("unknown flag '{}'", flag)),
                    }
                }
                Command::Build {
                    input: PathBuf::from(input),
                    output,
                    config,
                    module_roots,
                    playdate_bootstrap,
                }
            }
            "build-playdate" => {
                let input = args.next().ok_or_else(usage)?;
                let mut source_dir = None;
                let mut pdx = None;
                let mut pdc = None;
                let mut run = false;
                let mut config = None;
                let mut module_roots = Vec::new();
                while let Some(flag) = args.next() {
                    match flag.as_str() {
                        "--source-dir" => {
                            let path = args
                                .next()
                                .ok_or_else(|| "expected path after --source-dir".to_string())?;
                            source_dir = Some(PathBuf::from(path));
                        }
                        "--pdx" => {
                            let path = args
                                .next()
                                .ok_or_else(|| "expected path after --pdx".to_string())?;
                            pdx = Some(PathBuf::from(path));
                        }
                        "--pdc" => {
                            let exe = args
                                .next()
                                .ok_or_else(|| "expected executable after --pdc".to_string())?;
                            pdc = Some(exe);
                        }
                        "--run" => {
                            run = true;
                        }
                        "--config" => {
                            let path = args
                                .next()
                                .ok_or_else(|| "expected path after --config".to_string())?;
                            if config.is_some() {
                                return Err("duplicate --config flag".to_string());
                            }
                            config = Some(PathBuf::from(path));
                        }
                        "--module-root" => {
                            let path = args
                                .next()
                                .ok_or_else(|| "expected path after --module-root".to_string())?;
                            module_roots.push(PathBuf::from(path));
                        }
                        _ => return Err(format!("unknown flag '{}'", flag)),
                    }
                }
                Command::BuildPlaydate {
                    input: PathBuf::from(input),
                    source_dir,
                    pdx,
                    pdc,
                    run,
                    config,
                    module_roots,
                }
            }
            _ => return Err(usage()),
        };

        Ok(Cli { command })
    }
}

fn parse_common_flags(
    args: &mut impl Iterator<Item = String>,
    config: &mut Option<PathBuf>,
    module_roots: &mut Vec<PathBuf>,
) -> Result<(), String> {
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--config" => {
                let path = args
                    .next()
                    .ok_or_else(|| "expected path after --config".to_string())?;
                if config.is_some() {
                    return Err("duplicate --config flag".to_string());
                }
                *config = Some(PathBuf::from(path));
            }
            "--module-root" => {
                let path = args
                    .next()
                    .ok_or_else(|| "expected path after --module-root".to_string())?;
                module_roots.push(PathBuf::from(path));
            }
            _ => return Err(format!("unknown flag '{}'", flag)),
        }
    }

    Ok(())
}

fn usage() -> String {
    "Usage:
  callisto parse <input.cal>
  callisto init --template playdate <dir>
  callisto check <input.cal> [--config path] [--module-root path]...
  callisto emit-lua <input.cal> [-o out.lua|out_dir] [--config path] [--module-root path]... [--playdate-bootstrap]
  callisto build <input.cal> [-o out.lua|out_dir] [--config path] [--module-root path]... [--playdate-bootstrap]
  callisto build-playdate <input.cal> [--source-dir dir] [--pdx bundle.pdx] [--pdc exe] [--run] [--config path] [--module-root path]...

Examples:
  callisto init --template playdate my-game
  callisto check src/main.cal --config callisto.toml
  callisto check src/main.cal --module-root ../shared --module-root /opt/vendor
  callisto emit-lua src/main.cal
  callisto emit-lua src/main.cal -o build
  callisto build-playdate src/game.cal --config callisto.toml --pdx MyGame.pdx --run

Precedence:
  CLI flags override config values.
  --config selects config source.
  --module-root values override config module_roots.
  -o overrides config out_dir.
  --playdate-bootstrap writes a Playdate `main.lua` shim in output directories.
  Bootstrap contract: `pub fn init() -> S`, `pub fn update(state: S) -> S`, `pub fn render(state: S) -> Unit`."
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use std::path::PathBuf;

    #[test]
    fn parses_check_with_config_and_multiple_module_roots() {
        let cli = Cli::parse_from_args([
            "check",
            "src/main.cal",
            "--config",
            "cfg/callisto.toml",
            "--module-root",
            "lib",
            "--module-root",
            "vendor",
        ])
        .expect("parse cli");

        match cli.command {
            Command::Check {
                input,
                config,
                module_roots,
            } => {
                assert_eq!(input, PathBuf::from("src/main.cal"));
                assert_eq!(config, Some(PathBuf::from("cfg/callisto.toml")));
                assert_eq!(
                    module_roots,
                    vec![PathBuf::from("lib"), PathBuf::from("vendor")]
                );
            }
            _ => panic!("expected check command"),
        }
    }

    #[test]
    fn parses_init_playdate_template() {
        let cli =
            Cli::parse_from_args(["init", "--template", "playdate", "my-game"]).expect("parse cli");
        match cli.command {
            Command::InitPlaydate { dir } => {
                assert_eq!(dir, PathBuf::from("my-game"));
            }
            _ => panic!("expected init command"),
        }
    }

    #[test]
    fn parses_emit_lua_with_output_and_common_flags() {
        let cli = Cli::parse_from_args([
            "emit-lua",
            "src/main.cal",
            "-o",
            "out_dir",
            "--config",
            "callisto.toml",
            "--module-root",
            "deps",
        ])
        .expect("parse cli");

        match cli.command {
            Command::EmitLua {
                input,
                output,
                config,
                module_roots,
                playdate_bootstrap,
            } => {
                assert_eq!(input, PathBuf::from("src/main.cal"));
                assert_eq!(output, Some(PathBuf::from("out_dir")));
                assert_eq!(config, Some(PathBuf::from("callisto.toml")));
                assert_eq!(module_roots, vec![PathBuf::from("deps")]);
                assert!(!playdate_bootstrap);
            }
            _ => panic!("expected emit-lua command"),
        }
    }

    #[test]
    fn parses_build_with_common_flags_without_output() {
        let cli = Cli::parse_from_args([
            "build",
            "src/main.cal",
            "--module-root",
            "lib",
            "--module-root",
            "vendor",
        ])
        .expect("parse cli");

        match cli.command {
            Command::Build {
                input,
                output,
                config,
                module_roots,
                playdate_bootstrap,
            } => {
                assert_eq!(input, PathBuf::from("src/main.cal"));
                assert!(output.is_none());
                assert!(config.is_none());
                assert_eq!(
                    module_roots,
                    vec![PathBuf::from("lib"), PathBuf::from("vendor")]
                );
                assert!(!playdate_bootstrap);
            }
            _ => panic!("expected build command"),
        }
    }

    #[test]
    fn parses_emit_lua_with_playdate_bootstrap() {
        let cli = Cli::parse_from_args([
            "emit-lua",
            "src/main.cal",
            "--playdate-bootstrap",
            "--module-root",
            "deps",
        ])
        .expect("parse cli");

        match cli.command {
            Command::EmitLua {
                input,
                output,
                config,
                module_roots,
                playdate_bootstrap,
            } => {
                assert_eq!(input, PathBuf::from("src/main.cal"));
                assert!(output.is_none());
                assert!(config.is_none());
                assert_eq!(module_roots, vec![PathBuf::from("deps")]);
                assert!(playdate_bootstrap);
            }
            _ => panic!("expected emit-lua command"),
        }
    }

    #[test]
    fn parses_build_playdate_with_flags() {
        let cli = Cli::parse_from_args([
            "build-playdate",
            "src/game.cal",
            "--source-dir",
            "Source",
            "--pdx",
            "Game.pdx",
            "--pdc",
            "pdc-custom",
            "--run",
            "--config",
            "callisto.toml",
            "--module-root",
            "deps",
        ])
        .expect("parse cli");

        match cli.command {
            Command::BuildPlaydate {
                input,
                source_dir,
                pdx,
                pdc,
                run,
                config,
                module_roots,
            } => {
                assert_eq!(input, PathBuf::from("src/game.cal"));
                assert_eq!(source_dir, Some(PathBuf::from("Source")));
                assert_eq!(pdx, Some(PathBuf::from("Game.pdx")));
                assert_eq!(pdc.as_deref(), Some("pdc-custom"));
                assert!(run);
                assert_eq!(config, Some(PathBuf::from("callisto.toml")));
                assert_eq!(module_roots, vec![PathBuf::from("deps")]);
            }
            _ => panic!("expected build-playdate command"),
        }
    }

    #[test]
    fn rejects_unknown_flag() {
        let err =
            Cli::parse_from_args(["check", "src/main.cal", "--wat"]).expect_err("expected error");
        assert!(err.contains("unknown flag '--wat'"));
    }

    #[test]
    fn rejects_missing_path_after_config_flag() {
        let err = Cli::parse_from_args(["check", "src/main.cal", "--config"])
            .expect_err("expected error");
        assert!(err.contains("expected path after --config"));
    }

    #[test]
    fn rejects_missing_path_after_module_root_flag() {
        let err = Cli::parse_from_args(["check", "src/main.cal", "--module-root"])
            .expect_err("expected error");
        assert!(err.contains("expected path after --module-root"));
    }

    #[test]
    fn rejects_missing_path_after_output_flag() {
        let err =
            Cli::parse_from_args(["emit-lua", "src/main.cal", "-o"]).expect_err("expected error");
        assert!(err.contains("expected path after -o"));
    }

    #[test]
    fn rejects_duplicate_config_flag() {
        let err = Cli::parse_from_args([
            "emit-lua",
            "src/main.cal",
            "--config",
            "a.toml",
            "--config",
            "b.toml",
        ])
        .expect_err("expected error");
        assert!(err.contains("duplicate --config flag"));
    }

    #[test]
    fn rejects_init_without_template() {
        let err = Cli::parse_from_args(["init", "my-game"]).expect_err("expected error");
        assert!(err.contains("missing --template"));
    }

    #[test]
    fn rejects_unknown_template() {
        let err = Cli::parse_from_args(["init", "--template", "web", "my-game"])
            .expect_err("expected error");
        assert!(err.contains("unknown template"));
    }
}
