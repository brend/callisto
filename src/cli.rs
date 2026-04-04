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
    },
    Build {
        input: PathBuf,
        output: Option<PathBuf>,
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
                        _ => return Err(format!("unknown flag '{}'", flag)),
                    }
                }
                Command::EmitLua {
                    input: PathBuf::from(input),
                    output,
                    config,
                    module_roots,
                }
            }
            "build" => {
                let input = args.next().ok_or_else(usage)?;
                let mut output = None;
                let mut config = None;
                let mut module_roots = Vec::new();
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
                        _ => return Err(format!("unknown flag '{}'", flag)),
                    }
                }
                Command::Build {
                    input: PathBuf::from(input),
                    output,
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
  callisto check <input.cal> [--config path] [--module-root path]...
  callisto emit-lua <input.cal> [-o out.lua|out_dir] [--config path] [--module-root path]...
  callisto build <input.cal> [-o out.lua|out_dir] [--config path] [--module-root path]...

Examples:
  callisto check src/main.cal --config callisto.toml
  callisto check src/main.cal --module-root ../shared --module-root /opt/vendor
  callisto emit-lua src/main.cal
  callisto emit-lua src/main.cal -o build

Precedence:
  CLI flags override config values.
  --config selects config source.
  --module-root values override config module_roots.
  -o overrides config out_dir."
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
            } => {
                assert_eq!(input, PathBuf::from("src/main.cal"));
                assert_eq!(output, Some(PathBuf::from("out_dir")));
                assert_eq!(config, Some(PathBuf::from("callisto.toml")));
                assert_eq!(module_roots, vec![PathBuf::from("deps")]);
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
            } => {
                assert_eq!(input, PathBuf::from("src/main.cal"));
                assert!(output.is_none());
                assert!(config.is_none());
                assert_eq!(
                    module_roots,
                    vec![PathBuf::from("lib"), PathBuf::from("vendor")]
                );
            }
            _ => panic!("expected build command"),
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
}
