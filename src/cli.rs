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
    },
    EmitLua {
        input: PathBuf,
        output: Option<PathBuf>,
    },
    Build {
        input: PathBuf,
        output: Option<PathBuf>,
    },
}

impl Cli {
    pub fn parse_from_env() -> Result<Self, String> {
        let mut args = std::env::args().skip(1);
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
                Command::Check {
                    input: PathBuf::from(input),
                }
            }
            "emit-lua" => {
                let input = args.next().ok_or_else(usage)?;
                let mut output = None;
                while let Some(flag) = args.next() {
                    if flag == "-o" {
                        let path = args
                            .next()
                            .ok_or_else(|| "expected path after -o".to_string())?;
                        output = Some(PathBuf::from(path));
                    } else {
                        return Err(format!("unknown flag '{}'", flag));
                    }
                }
                Command::EmitLua {
                    input: PathBuf::from(input),
                    output,
                }
            }
            "build" => {
                let input = args.next().ok_or_else(usage)?;
                let mut output = None;
                while let Some(flag) = args.next() {
                    if flag == "-o" {
                        let path = args
                            .next()
                            .ok_or_else(|| "expected path after -o".to_string())?;
                        output = Some(PathBuf::from(path));
                    } else {
                        return Err(format!("unknown flag '{}'", flag));
                    }
                }
                Command::Build {
                    input: PathBuf::from(input),
                    output,
                }
            }
            _ => return Err(usage()),
        };

        Ok(Cli { command })
    }
}

fn usage() -> String {
    "Usage:\n  callisto parse <input.cal>\n  callisto check <input.cal>\n  callisto emit-lua <input.cal> [-o out.lua|out_dir]\n  callisto build <input.cal> [-o out.lua|out_dir]"
        .to_string()
}
