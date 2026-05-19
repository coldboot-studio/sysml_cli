//! Sysand-compatible project manifest (US-204).
//!
//! Sysand (`pip install sysand`) is the de-facto package manager for the
//! SysML v2 ecosystem. A project is a directory containing a
//! `.project.json` file plus one or more `.sysml` / `.kerml` source
//! files. We adopt the same convention rather than invent a competing
//! manifest format.
//!
//! Discovery: walk up from the working directory looking for the first
//! ancestor containing a `.project.json` file. If found, its `root`
//! field defines the source root; if absent, the manifest directory is
//! itself the source root.
//!
//! This first implementation parses the fields needed for resolution
//! (`name`, `version`, `root`) and reports them in the metadata block.
//! KPAR dependency support and full dependency-graph resolution land in
//! a Phase 2.x follow-up.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // Several fields are surfaced via the manifest API or reserved for follow-up batches.
pub struct ProjectManifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    /// Source root, relative to the manifest's directory. Defaults to
    /// the manifest's directory itself when absent.
    pub root: Option<PathBuf>,
    /// Sysand dependency references. Parsed but not yet resolved —
    /// dependency loading lands with KPAR support in a follow-up batch.
    #[serde(default)]
    pub dependencies: Vec<DependencyRef>,
    /// Allow extra Sysand fields without rejecting — Sysand's format
    /// has evolved and we don't want to reject a manifest the package
    /// manager itself accepts.
    #[serde(default, rename = "meta")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // Populated by serde; consumed by KPAR-loading work in a follow-up batch.
pub struct DependencyRef {
    pub name: String,
    pub version: Option<String>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct LoadedManifest {
    pub manifest: ProjectManifest,
    pub manifest_path: Option<PathBuf>,
    pub source_root: Option<PathBuf>,
}

impl LoadedManifest {
    pub fn display_label(&self) -> String {
        let name = self
            .manifest
            .name
            .as_deref()
            .unwrap_or("<unnamed sysand project>");
        match &self.manifest.version {
            Some(version) => format!("{name} {version}"),
            None => name.to_string(),
        }
    }
}

pub fn discover(start_dir: &Path) -> Result<LoadedManifest, String> {
    let mut current = Some(start_dir.to_path_buf());
    while let Some(directory) = current {
        let candidate = directory.join(".project.json");
        if candidate.is_file() {
            return load(&candidate);
        }
        current = directory.parent().map(Path::to_path_buf);
    }
    Ok(LoadedManifest::default())
}

pub fn load(path: &Path) -> Result<LoadedManifest, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("unable to read manifest '{}': {error}", path.display()))?;
    let manifest: ProjectManifest = serde_json::from_str(&text)
        .map_err(|error| format!("invalid manifest '{}': {error}", path.display()))?;

    let manifest_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let source_root = match &manifest.root {
        Some(relative) => Some(manifest_dir.join(relative)),
        None => Some(manifest_dir),
    };

    Ok(LoadedManifest {
        manifest,
        manifest_path: Some(path.to_path_buf()),
        source_root,
    })
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
        let path = env::temp_dir().join(format!("sysml_validate_manifest_{label}_{unique}"));
        fs::create_dir_all(&path).expect("temp dir");
        path
    }

    #[test]
    fn parses_minimal_manifest() {
        let dir = temp_dir("minimal");
        fs::write(
            dir.join(".project.json"),
            r#"{"name":"scamp","version":"0.1.0"}"#,
        )
        .unwrap();
        let loaded = discover(&dir).unwrap();
        assert_eq!(loaded.manifest.name.as_deref(), Some("scamp"));
        assert_eq!(loaded.manifest.version.as_deref(), Some("0.1.0"));
        assert_eq!(loaded.display_label(), "scamp 0.1.0");
        assert!(loaded.source_root.is_some());
    }

    #[test]
    fn discovers_from_subdir() {
        let dir = temp_dir("ascend");
        fs::write(dir.join(".project.json"), r#"{"name":"top"}"#).unwrap();
        let nested = dir.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        let loaded = discover(&nested).unwrap();
        assert_eq!(loaded.manifest.name.as_deref(), Some("top"));
    }

    #[test]
    fn no_manifest_returns_empty_loaded() {
        let dir = temp_dir("nomanifest");
        let loaded = discover(&dir).unwrap();
        assert!(loaded.manifest_path.is_none());
        assert!(loaded.manifest.name.is_none());
    }

    #[test]
    fn parses_dependencies_array() {
        let dir = temp_dir("deps");
        fs::write(
            dir.join(".project.json"),
            r#"{"name":"x","dependencies":[{"name":"y","version":"1.0"}]}"#,
        )
        .unwrap();
        let loaded = discover(&dir).unwrap();
        assert_eq!(loaded.manifest.dependencies.len(), 1);
        assert_eq!(loaded.manifest.dependencies[0].name, "y");
    }

    #[test]
    fn respects_root_field() {
        let dir = temp_dir("root");
        fs::create_dir_all(dir.join("model")).unwrap();
        fs::write(
            dir.join(".project.json"),
            r#"{"name":"x","root":"model"}"#,
        )
        .unwrap();
        let loaded = discover(&dir).unwrap();
        let root = loaded.source_root.unwrap();
        assert!(root.ends_with("model"), "expected root ending in 'model', got {root:?}");
    }

    #[test]
    fn rejects_unknown_top_level_field() {
        let dir = temp_dir("unknown");
        fs::write(
            dir.join(".project.json"),
            r#"{"name":"x","made_up":"oops"}"#,
        )
        .unwrap();
        let err = discover(&dir).unwrap_err();
        assert!(err.contains("unknown field"), "got: {err}");
    }
}
