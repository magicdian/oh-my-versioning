use std::path::{Path, PathBuf};

use crate::errors::{ConfigError, OmvError};

pub mod adapters;
pub mod atomic;
pub mod config;
pub mod finalizations;
pub mod state;
pub mod targets;

pub const OMV_DIR: &str = ".omv";
pub const CONFIG_FILE: &str = "config.toml";
pub const STATE_FILE: &str = "state.toml";
pub const TARGETS_FILE: &str = "targets.toml";
pub const ADAPTERS_FILE: &str = "adapters.toml";
pub const FINALIZATIONS_FILE: &str = "finalizations.toml";

pub fn resolve_project_root(cwd: &Path) -> Result<PathBuf, OmvError> {
    let absolute = cwd
        .canonicalize()
        .map_err(|_| ConfigError::RootResolution {
            cwd: cwd.display().to_string(),
        })?;

    for ancestor in absolute.ancestors() {
        if ancestor.join(".git").exists() {
            return Ok(ancestor.to_path_buf());
        }
    }

    Ok(absolute)
}

pub fn resolve_omv_root(cwd: &Path) -> Result<PathBuf, OmvError> {
    Ok(resolve_project_root(cwd)?.join(OMV_DIR))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{resolve_omv_root, resolve_project_root};

    #[test]
    fn resolve_project_root_returns_git_ancestor_when_present() {
        let repo_root = temp_dir("resolve-root-git");
        fs::create_dir_all(repo_root.join(".git")).expect("should create fake git dir");
        let nested = repo_root.join("a/b/c");
        fs::create_dir_all(&nested).expect("should create nested path");

        let canonical_repo_root = repo_root
            .canonicalize()
            .expect("repo root should canonicalize");
        let resolved = resolve_project_root(&nested).expect("project root should resolve");
        assert_eq!(resolved, canonical_repo_root);

        cleanup_dir(&repo_root);
    }

    #[test]
    fn resolve_omv_root_appends_omv_dir() {
        let cwd = temp_dir("resolve-omv");
        let expected = cwd
            .canonicalize()
            .expect("cwd should canonicalize")
            .join(".omv");
        let resolved = resolve_omv_root(&cwd).expect("omv root should resolve");
        assert_eq!(resolved, expected);

        cleanup_dir(&cwd);
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("omv-{prefix}-{stamp}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn cleanup_dir(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }
}
