use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Top-level config, loaded from `squint.toml`.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub rules: RulesConfig,
    /// Glob patterns for files/directories to exclude from linting.
    /// Matched against each path relative to the directory containing
    /// `squint.toml` (or the current working directory if no config
    /// file is found). Examples: `["target/**", "**/node_modules/**"]`.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Directory used as the base for resolving relative exclude patterns.
    /// Set automatically when the config file is loaded; not read from TOML.
    #[serde(skip)]
    pub base_dir: PathBuf,
}

#[derive(Debug, Deserialize, Default)]
pub struct RulesConfig {
    #[serde(rename = "LT05", default)]
    pub lt05: Lt05Config,
    #[serde(rename = "AL06", default)]
    pub al06: Al06Config,
    #[serde(rename = "CV03", default)]
    pub cv03: Cv03Config,
    #[serde(rename = "CV04", default)]
    pub cv04: Cv04Config,
    #[serde(rename = "AM06", default)]
    pub am06: Am06Config,
    /// Per-rule severity overrides: `{ "CP01" = "warning", "LT05" = "error" }`.
    /// Deserialized from `[rules.severity]` table in TOML.
    #[serde(default)]
    pub severity: HashMap<String, String>,
}

// ── LT05 ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct Lt05Config {
    pub max_line_length: usize,
    /// Parsed but not yet enforced — reserved for future implementation.
    #[allow(dead_code)]
    #[serde(default)]
    pub ignore_comment_lines: bool,
}

impl Default for Lt05Config {
    fn default() -> Self {
        Lt05Config {
            max_line_length: 120,
            ignore_comment_lines: false,
        }
    }
}

// ── AL06 ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct Al06Config {
    pub min_alias_length: usize,
    /// 0 means unlimited
    pub max_alias_length: usize,
}

impl Default for Al06Config {
    fn default() -> Self {
        Al06Config {
            min_alias_length: 1,
            max_alias_length: 0,
        }
    }
}

// ── CV03 ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TrailingCommaPolicy {
    #[default]
    Forbid,
    Require,
}

#[derive(Debug, Deserialize, Default)]
pub struct Cv03Config {
    #[serde(default)]
    pub select_clause_trailing_comma: TrailingCommaPolicy,
}

// ── CV04 ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct Cv04Config {
    /// Prefer COUNT(1) over COUNT(*). Default: false (prefer COUNT(*))
    #[serde(default)]
    pub prefer_count_1: bool,
}

// ── AM06 ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum GroupByStyle {
    /// All references must be explicit column names
    #[default]
    Explicit,
    /// All references must be positional numbers
    Implicit,
    /// All references must be consistent (either all explicit or all positional)
    Consistent,
}

#[derive(Debug, Deserialize, Default)]
pub struct Am06Config {
    #[serde(default)]
    pub group_by_and_order_by_style: GroupByStyle,
}

// ── Loading ──────────────────────────────────────────────────────────────────

enum ConfigFile {
    SquintToml(PathBuf),
    PyprojectToml(PathBuf),
}

impl Config {
    /// Load config by walking up from `start_dir`.
    ///
    /// Search order at each directory level:
    /// 1. `squint.toml` — dedicated config file
    /// 2. `pyproject.toml` containing a `[tool.squint]` section
    ///
    /// Returns `Config::default()` with `base_dir = start_dir` if neither is found.
    pub fn load(start_dir: &Path) -> Self {
        match find_config_file(start_dir) {
            Some(ConfigFile::SquintToml(path)) => {
                let base_dir = path.parent().unwrap_or(start_dir).to_path_buf();
                match std::fs::read_to_string(&path) {
                    Ok(contents) => match toml::from_str::<Config>(&contents) {
                        Ok(mut cfg) => {
                            cfg.base_dir = base_dir;
                            return cfg;
                        }
                        Err(e) => eprintln!("warning: could not parse {}: {}", path.display(), e),
                    },
                    Err(e) => eprintln!("warning: could not read {}: {}", path.display(), e),
                }
            }
            Some(ConfigFile::PyprojectToml(path)) => {
                let base_dir = path.parent().unwrap_or(start_dir).to_path_buf();
                if let Some(mut cfg) = load_from_pyproject(&path) {
                    cfg.base_dir = base_dir;
                    return cfg;
                }
            }
            None => {}
        }
        Config {
            base_dir: start_dir.to_path_buf(),
            ..Config::default()
        }
    }
}

fn find_config_file(start: &Path) -> Option<ConfigFile> {
    let mut dir = start.to_path_buf();
    loop {
        let squint = dir.join("squint.toml");
        if squint.exists() {
            return Some(ConfigFile::SquintToml(squint));
        }
        let pyproject = dir.join("pyproject.toml");
        if pyproject.exists() && pyproject_has_squint_section(&pyproject) {
            return Some(ConfigFile::PyprojectToml(pyproject));
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Returns true if `pyproject.toml` at `path` contains a `[tool.squint]` table.
fn pyproject_has_squint_section(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str::<toml::Value>(&s).ok())
        .and_then(|v| v.get("tool").and_then(|t| t.get("squint")).cloned())
        .is_some()
}

/// Extract `[tool.squint]` from a `pyproject.toml` and deserialise it as `Config`.
fn load_from_pyproject(path: &Path) -> Option<Config> {
    let contents = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("warning: could not read {}: {}", path.display(), e);
            return None;
        }
    };
    let doc: toml::Value = match toml::from_str(&contents) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("warning: could not parse {}: {}", path.display(), e);
            return None;
        }
    };
    let squint_section = doc.get("tool")?.get("squint")?;
    // Re-serialise the subtable to a TOML string so we can reuse toml::from_str::<Config>.
    let section_str = match toml::to_string(squint_section) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "warning: could not re-serialise [tool.squint] from {}: {}",
                path.display(),
                e
            );
            return None;
        }
    };
    match toml::from_str::<Config>(&section_str) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!(
                "warning: could not parse [tool.squint] in {}: {}",
                path.display(),
                e
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.rules.lt05.max_line_length, 120);
        assert_eq!(cfg.rules.al06.min_alias_length, 1);
        assert_eq!(cfg.rules.al06.max_alias_length, 0);
    }

    #[test]
    fn test_parse_toml() {
        let toml = r#"
[rules.LT05]
max_line_length = 80

[rules.AL06]
min_alias_length = 2
max_alias_length = 20

[rules.CV03]
select_clause_trailing_comma = "require"

[rules.AM06]
group_by_and_order_by_style = "implicit"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.rules.lt05.max_line_length, 80);
        assert_eq!(cfg.rules.al06.min_alias_length, 2);
        assert_eq!(
            cfg.rules.cv03.select_clause_trailing_comma,
            TrailingCommaPolicy::Require
        );
        assert_eq!(
            cfg.rules.am06.group_by_and_order_by_style,
            GroupByStyle::Implicit
        );
    }

    #[test]
    fn test_partial_toml_uses_defaults() {
        let cfg: Config = toml::from_str("").unwrap();
        assert_eq!(cfg.rules.lt05.max_line_length, 120);
    }

    #[test]
    fn test_exclude_patterns() {
        let toml = r#"
exclude = ["target/**", "**/node_modules/**", "vendor/*.sql"]
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.exclude.len(), 3);
        assert_eq!(cfg.exclude[0], "target/**");
        assert_eq!(cfg.exclude[1], "**/node_modules/**");
        assert_eq!(cfg.exclude[2], "vendor/*.sql");
    }

    #[test]
    fn test_exclude_defaults_empty() {
        let cfg: Config = toml::from_str("").unwrap();
        assert!(cfg.exclude.is_empty());
    }

    #[test]
    fn test_load_from_pyproject() {
        let toml = r#"
[tool.squint]
exclude = ["target/**"]

[tool.squint.rules.LT05]
max_line_length = 88

[tool.squint.rules.severity]
LT05 = "warning"
"#;
        let doc: toml::Value = toml::from_str(toml).unwrap();
        let section = doc["tool"]["squint"].clone();
        let section_str = toml::to_string(&section).unwrap();
        let cfg: Config = toml::from_str(&section_str).unwrap();
        assert_eq!(cfg.rules.lt05.max_line_length, 88);
        assert_eq!(cfg.exclude, vec!["target/**"]);
        assert_eq!(
            cfg.rules.severity.get("LT05").map(|s| s.as_str()),
            Some("warning")
        );
    }

    #[test]
    fn test_pyproject_without_squint_section_ignored() {
        // A pyproject.toml with no [tool.squint] should not be treated as config
        let toml = r#"
[tool.black]
line-length = 88
"#;
        let doc: toml::Value = toml::from_str(toml).unwrap();
        let has_squint = doc.get("tool").and_then(|t| t.get("squint")).is_some();
        assert!(!has_squint);
    }
}
