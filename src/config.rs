use std::{
    collections::HashSet,
    fmt, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectConfig {
    pub module_roots: Vec<PathBuf>,
    pub out_dir: Option<PathBuf>,
    pub package: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    Default,
    Discovered(PathBuf),
    Explicit(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProjectConfig {
    pub source: ConfigSource,
    pub config: ProjectConfig,
}

#[derive(Debug)]
pub enum ConfigError {
    MissingExplicitConfig {
        path: PathBuf,
    },
    ReadFailed {
        path: PathBuf,
        error: String,
    },
    ParseFailed {
        path: PathBuf,
        error: String,
    },
    InvalidField {
        path: PathBuf,
        field: &'static str,
        error: String,
    },
}

impl ConfigError {
    fn code(&self) -> &'static str {
        match self {
            ConfigError::MissingExplicitConfig { .. } => "CAL-CFG-001",
            ConfigError::ReadFailed { .. } => "CAL-CFG-002",
            ConfigError::ParseFailed { .. } => "CAL-CFG-003",
            ConfigError::InvalidField { .. } => "CAL-CFG-004",
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] ", self.code())?;
        match self {
            ConfigError::MissingExplicitConfig { path } => {
                write!(f, "explicit config '{}' was not found", path.display())
            }
            ConfigError::ReadFailed { path, error } => {
                write!(f, "failed to read config '{}': {}", path.display(), error)
            }
            ConfigError::ParseFailed { path, error } => {
                write!(f, "failed to parse config '{}': {}", path.display(), error)
            }
            ConfigError::InvalidField { path, field, error } => write!(
                f,
                "invalid field '{}' in config '{}': {}",
                field,
                path.display(),
                error
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Deserialize, Default)]
struct RawProjectConfig {
    module_roots: Option<Vec<String>>,
    out_dir: Option<String>,
    package: Option<String>,
}

pub fn load_project_config(
    entry_input: &Path,
    explicit_config_path: Option<&Path>,
) -> Result<LoadedProjectConfig, ConfigError> {
    let entry_path = normalize_path(entry_input.to_path_buf());
    let entry_dir = entry_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| normalize_path(PathBuf::from(".")));

    if let Some(explicit) = explicit_config_path {
        let explicit = normalize_path(explicit.to_path_buf());
        if !explicit.is_file() {
            return Err(ConfigError::MissingExplicitConfig { path: explicit });
        }
        return load_config_file(explicit.clone(), ConfigSource::Explicit(explicit));
    }

    let discovered = normalize_path(entry_dir.join("callisto.toml"));
    if discovered.is_file() {
        return load_config_file(discovered.clone(), ConfigSource::Discovered(discovered));
    }

    Ok(LoadedProjectConfig {
        source: ConfigSource::Default,
        config: ProjectConfig::default(),
    })
}

fn load_config_file(
    path: PathBuf,
    source: ConfigSource,
) -> Result<LoadedProjectConfig, ConfigError> {
    let text = fs::read_to_string(&path).map_err(|err| ConfigError::ReadFailed {
        path: path.clone(),
        error: err.to_string(),
    })?;
    let raw =
        toml::from_str::<RawProjectConfig>(&text).map_err(|err| ConfigError::ParseFailed {
            path: path.clone(),
            error: err.to_string(),
        })?;
    let config_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| normalize_path(PathBuf::from(".")));
    let config = parse_raw_config(&path, &config_dir, raw)?;
    Ok(LoadedProjectConfig { source, config })
}

fn parse_raw_config(
    config_path: &Path,
    config_dir: &Path,
    raw: RawProjectConfig,
) -> Result<ProjectConfig, ConfigError> {
    let mut module_roots = Vec::new();
    let mut seen_roots = HashSet::new();
    for value in raw.module_roots.unwrap_or_default() {
        if value.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                path: config_path.to_path_buf(),
                field: "module_roots",
                error: "entries must be non-empty strings".to_string(),
            });
        }
        let resolved = resolve_config_path(config_dir, &value);
        if !seen_roots.insert(resolved.clone()) {
            return Err(ConfigError::InvalidField {
                path: config_path.to_path_buf(),
                field: "module_roots",
                error: format!("duplicate module root '{}'", value),
            });
        }
        module_roots.push(resolved);
    }

    let out_dir = raw
        .out_dir
        .map(|value| {
            if value.trim().is_empty() {
                Err(ConfigError::InvalidField {
                    path: config_path.to_path_buf(),
                    field: "out_dir",
                    error: "value must be a non-empty path".to_string(),
                })
            } else {
                Ok(resolve_config_path(config_dir, &value))
            }
        })
        .transpose()?;

    let package = raw
        .package
        .map(|value| {
            if value.trim().is_empty() {
                Err(ConfigError::InvalidField {
                    path: config_path.to_path_buf(),
                    field: "package",
                    error: "value must be a non-empty string".to_string(),
                })
            } else {
                Ok(value)
            }
        })
        .transpose()?;

    Ok(ProjectConfig {
        module_roots,
        out_dir,
        package,
    })
}

fn resolve_config_path(config_dir: &Path, raw_path: &str) -> PathBuf {
    let path = PathBuf::from(raw_path);
    if path.is_absolute() {
        path
    } else {
        config_dir.join(path)
    }
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

#[cfg(test)]
mod tests {
    use super::{ConfigError, ConfigSource, load_project_config};
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("callisto-config-{label}-{nanos}"));
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    fn normalize(path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().expect("cwd").join(path)
        }
    }

    #[test]
    fn returns_default_when_no_config_exists() {
        let root = unique_temp_dir("default");
        let entry = root.join("main.cal");
        fs::write(&entry, "fn main() -> Int do 0 end\n").expect("write entry");

        let loaded = load_project_config(&entry, None).expect("load config");
        assert!(matches!(loaded.source, ConfigSource::Default));
        assert!(loaded.config.module_roots.is_empty());
        assert!(loaded.config.out_dir.is_none());
        assert!(loaded.config.package.is_none());
    }

    #[test]
    fn discovers_config_in_entry_directory_and_resolves_relative_paths() {
        let root = unique_temp_dir("discovered");
        let src_dir = root.join("src");
        fs::create_dir_all(&src_dir).expect("create src dir");
        let entry = src_dir.join("app.cal");
        fs::write(&entry, "fn main() -> Int do 0 end\n").expect("write entry");

        let absolute_dep = normalize(&root.join("dep"));
        let config_path = src_dir.join("callisto.toml");
        fs::write(
            &config_path,
            format!(
                "module_roots = [\"../lib\", \"{}\"]\nout_dir = \"build\"\npackage = \"demo.pkg\"\n",
                absolute_dep.display()
            ),
        )
        .expect("write config");

        let loaded = load_project_config(&entry, None).expect("load config");
        assert!(matches!(
            loaded.source,
            ConfigSource::Discovered(ref p) if p == &normalize(&config_path)
        ));
        assert_eq!(loaded.config.module_roots.len(), 2);
        assert_eq!(loaded.config.module_roots[0], src_dir.join("../lib"));
        assert_eq!(loaded.config.module_roots[1], absolute_dep);
        assert_eq!(loaded.config.out_dir, Some(src_dir.join("build")));
        assert_eq!(loaded.config.package.as_deref(), Some("demo.pkg"));
    }

    #[test]
    fn explicit_config_path_takes_precedence() {
        let root = unique_temp_dir("explicit");
        let entry_dir = root.join("entry");
        let cfg_dir = root.join("cfg");
        fs::create_dir_all(&entry_dir).expect("create entry dir");
        fs::create_dir_all(&cfg_dir).expect("create cfg dir");

        let entry = entry_dir.join("main.cal");
        fs::write(&entry, "fn main() -> Int do 0 end\n").expect("write entry");
        fs::write(
            entry_dir.join("callisto.toml"),
            "package = \"from_entry\"\n",
        )
        .expect("write discovered config");

        let explicit = cfg_dir.join("custom.toml");
        fs::write(&explicit, "package = \"from_explicit\"\n").expect("write explicit config");

        let loaded = load_project_config(&entry, Some(&explicit)).expect("load config");
        assert!(matches!(
            loaded.source,
            ConfigSource::Explicit(ref p) if p == &normalize(&explicit)
        ));
        assert_eq!(loaded.config.package.as_deref(), Some("from_explicit"));
    }

    #[test]
    fn rejects_empty_module_root_entry() {
        let root = unique_temp_dir("empty-root");
        let entry = root.join("main.cal");
        let config = root.join("callisto.toml");
        fs::write(&entry, "fn main() -> Int do 0 end\n").expect("write entry");
        fs::write(&config, "module_roots = [\"\", \"lib\"]\n").expect("write config");

        let err = load_project_config(&entry, None).expect_err("expected invalid config");
        match err {
            ConfigError::InvalidField { field, .. } => assert_eq!(field, "module_roots"),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_duplicate_module_roots() {
        let root = unique_temp_dir("dup-root");
        let entry = root.join("main.cal");
        let config = root.join("callisto.toml");
        fs::write(&entry, "fn main() -> Int do 0 end\n").expect("write entry");
        fs::write(&config, "module_roots = [\"lib\", \"lib\"]\n").expect("write config");

        let err = load_project_config(&entry, None).expect_err("expected invalid config");
        match err {
            ConfigError::InvalidField { field, error, .. } => {
                assert_eq!(field, "module_roots");
                assert!(error.contains("duplicate module root"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn config_errors_include_error_codes_in_display() {
        let err = ConfigError::MissingExplicitConfig {
            path: PathBuf::from("/tmp/missing.toml"),
        };
        let rendered = err.to_string();
        assert!(rendered.starts_with("[CAL-CFG-001] "));
    }
}
