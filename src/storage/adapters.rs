use std::fs;
use std::path::{Path, PathBuf};

use crate::core::schema::OmvAdapters;
use crate::errors::{AdapterError, OmvError};

use super::{ADAPTERS_FILE, atomic};

pub fn path_for(root: &Path) -> PathBuf {
    root.join(ADAPTERS_FILE)
}

pub fn load_adapters(root: &Path) -> Result<OmvAdapters, OmvError> {
    let path = path_for(root);
    let content = fs::read_to_string(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            AdapterError::MissingRegistry { path: path.clone() }
        } else {
            AdapterError::Parse {
                path: path.clone(),
                reason: err.to_string(),
            }
        }
    })?;

    toml::from_str(&content).map_err(|err| {
        AdapterError::Parse {
            path,
            reason: err.to_string(),
        }
        .into()
    })
}

pub fn load_adapters_if_exists(root: &Path) -> Result<OmvAdapters, OmvError> {
    match load_adapters(root) {
        Ok(registry) => Ok(registry),
        Err(OmvError::Adapter(AdapterError::MissingRegistry { .. })) => Ok(OmvAdapters::default()),
        Err(err) => Err(err),
    }
}

pub fn save_adapters(root: &Path, adapters: &OmvAdapters) -> Result<(), OmvError> {
    let path = path_for(root);
    let content = toml::to_string_pretty(adapters).map_err(|err| AdapterError::Parse {
        path: path.clone(),
        reason: err.to_string(),
    })?;

    atomic::write_atomically(&path, content.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::core::adapter::{AdapterInstallMode, AdapterKind, AdapterTargetMode};
    use crate::core::schema::{OmvAdapterInstallation, OmvAdapterTarget, OmvAdapters};
    use crate::errors::{AdapterError, OmvError};

    use super::{load_adapters, save_adapters};

    #[test]
    fn adapters_round_trip_preserves_installations() {
        let root = temp_omv_root("adapters-roundtrip");
        let adapters = OmvAdapters {
            schema_version: 1,
            installations: vec![OmvAdapterInstallation {
                kind: AdapterKind::Agent,
                name: String::from("codex"),
                install_mode: AdapterInstallMode::Hybrid,
                source_contract_version: 1,
                targets: vec![OmvAdapterTarget {
                    path: String::from("AGENTS.md"),
                    source_path: String::from(".omv/ai/adapters/codex/AGENTS.md"),
                    mode: AdapterTargetMode::ManagedBlock,
                }],
            }],
        };

        save_adapters(&root, &adapters).expect("adapters should save");
        let loaded = load_adapters(&root).expect("adapters should load");
        assert_eq!(loaded, adapters);

        cleanup_root(&root);
    }

    #[test]
    fn missing_registry_returns_typed_error() {
        let root = temp_omv_root("adapters-missing");
        let err = load_adapters(&root).expect_err("missing adapters should fail");
        assert!(matches!(
            err,
            OmvError::Adapter(AdapterError::MissingRegistry { .. })
        ));
        cleanup_root(&root);
    }

    fn temp_omv_root(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let root = std::env::temp_dir()
            .join(format!("omv-{prefix}-{stamp}"))
            .join(".omv");
        fs::create_dir_all(&root).expect("temp root should be created");
        root
    }

    fn cleanup_root(root: &Path) {
        if let Some(parent) = root.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
