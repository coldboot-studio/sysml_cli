//! `sysml-validate.toml` configuration file discovery and loading.
//!
//! Search order: explicit `--config <path>`, then `sysml-validate.toml` in
//! the current working directory and each ancestor up to the filesystem
//! root. The first match wins.
//!
//! Command-line flags always override file values.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::diag::Severity;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub project_root: Option<PathBuf>,
    #[serde(default)]
    pub default_format: Option<String>,
    #[serde(default)]
    pub default_strict: Option<bool>,
    #[serde(default)]
    pub default_fail_on_warning: Option<bool>,
    /// Per-rule severity overrides. Keys are SYSML* rule IDs; values are
    /// `"error" | "warning" | "info" | "off"`.
    #[serde(default)]
    pub rules: HashMap<String, String>,
    /// Glob patterns that limit which discovered files are validated. An
    /// empty list means "all `.sysml` and `.kerml` files".
    #[serde(default)]
    pub include: Vec<String>,
    /// Glob patterns that drop discovered files. Applied after include.
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleOverride {
    Level(Severity),
    Off,
}

impl Config {
    pub fn rule_override(&self, code: &str) -> Option<RuleOverride> {
        let raw = self.rules.get(code)?;
        match raw.to_ascii_lowercase().as_str() {
            "error" => Some(RuleOverride::Level(Severity::Error)),
            "warning" | "warn" => Some(RuleOverride::Level(Severity::Warning)),
            "info" => Some(RuleOverride::Level(Severity::Info)),
            "off" | "disabled" | "ignore" => Some(RuleOverride::Off),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LoadedConfig {
    pub config: Config,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug)]
pub enum ConfigDiscovery<'a> {
    Auto,
    Explicit(&'a Path),
    Disabled,
}

pub fn load(start_dir: &Path, discovery: ConfigDiscovery<'_>) -> Result<LoadedConfig, String> {
    let path = match discovery {
        ConfigDiscovery::Disabled => return Ok(LoadedConfig::default()),
        ConfigDiscovery::Explicit(path) => Some(path.to_path_buf()),
        ConfigDiscovery::Auto => find_config(start_dir),
    };
    let Some(path) = path else {
        return Ok(LoadedConfig::default());
    };

    let text = fs::read_to_string(&path)
        .map_err(|error| format!("unable to read config '{}': {error}", path.display()))?;
    let config: Config = toml::from_str(&text)
        .map_err(|error| format!("invalid config '{}': {error}", path.display()))?;
    Ok(LoadedConfig {
        config,
        path: Some(path),
    })
}

fn find_config(start_dir: &Path) -> Option<PathBuf> {
    let mut current = Some(start_dir.to_path_buf());
    while let Some(directory) = current {
        let candidate = directory.join("sysml-validate.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        current = directory.parent().map(Path::to_path_buf);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = env::temp_dir().join(format!("sysml_validate_config_{label}_{unique}"));
        fs::create_dir_all(&path).expect("temp dir");
        path
    }

    #[test]
    fn loads_minimal_config() {
        let dir = temp_dir("min");
        let path = dir.join("sysml-validate.toml");
        fs::write(&path, "default_strict = true\n").unwrap();
        let loaded = load(&dir, ConfigDiscovery::Auto).unwrap();
        assert_eq!(loaded.config.default_strict, Some(true));
        assert_eq!(loaded.path, Some(path));
    }

    #[test]
    fn rejects_unknown_fields() {
        let dir = temp_dir("unknown");
        fs::write(
            dir.join("sysml-validate.toml"),
            "made_up_field = \"oops\"\n",
        )
        .unwrap();
        let err = load(&dir, ConfigDiscovery::Auto).unwrap_err();
        assert!(err.contains("unknown field"), "got: {err}");
    }

    #[test]
    fn parses_rule_severity_overrides() {
        let dir = temp_dir("rules");
        fs::write(
            dir.join("sysml-validate.toml"),
            "[rules]\nSYSML040 = \"error\"\nSYSML041 = \"off\"\n",
        )
        .unwrap();
        let loaded = load(&dir, ConfigDiscovery::Auto).unwrap();
        assert_eq!(
            loaded.config.rule_override("SYSML040"),
            Some(RuleOverride::Level(Severity::Error))
        );
        assert_eq!(loaded.config.rule_override("SYSML041"), Some(RuleOverride::Off));
        assert_eq!(loaded.config.rule_override("SYSML999"), None);
    }

    #[test]
    fn ascends_to_find_config() {
        let dir = temp_dir("ascend");
        let nested = dir.join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();
        fs::write(dir.join("sysml-validate.toml"), "default_strict = true\n").unwrap();
        let loaded = load(&nested, ConfigDiscovery::Auto).unwrap();
        assert_eq!(loaded.config.default_strict, Some(true));
    }

    #[test]
    fn disabled_discovery_returns_empty() {
        let dir = temp_dir("disabled");
        fs::write(dir.join("sysml-validate.toml"), "default_strict = true\n").unwrap();
        let loaded = load(&dir, ConfigDiscovery::Disabled).unwrap();
        assert!(loaded.path.is_none());
        assert_eq!(loaded.config.default_strict, None);
    }

    #[test]
    fn parses_include_and_exclude_globs() {
        let dir = temp_dir("globs");
        fs::write(
            dir.join("sysml-validate.toml"),
            "include = [\"src/**/*.sysml\"]\nexclude = [\"**/generated/**\"]\n",
        )
        .unwrap();
        let loaded = load(&dir, ConfigDiscovery::Auto).unwrap();
        assert_eq!(loaded.config.include, vec!["src/**/*.sysml"]);
        assert_eq!(loaded.config.exclude, vec!["**/generated/**"]);
    }
}
