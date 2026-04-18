use std::fs;
use std::path::{Path, PathBuf};

use crate::core::schema::OmvFinalizations;
use crate::errors::{FinalizationError, OmvError};

use super::{FINALIZATIONS_FILE, atomic};

pub fn path_for(root: &Path) -> PathBuf {
    root.join(FINALIZATIONS_FILE)
}

pub fn load_finalizations(root: &Path) -> Result<OmvFinalizations, OmvError> {
    let path = path_for(root);
    let content = fs::read_to_string(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            FinalizationError::Missing { path: path.clone() }
        } else {
            FinalizationError::Parse {
                path: path.clone(),
                reason: err.to_string(),
            }
        }
    })?;

    toml::from_str(&content).map_err(|err| {
        FinalizationError::Parse {
            path,
            reason: err.to_string(),
        }
        .into()
    })
}

pub fn load_finalizations_if_exists(root: &Path) -> Result<OmvFinalizations, OmvError> {
    match load_finalizations(root) {
        Ok(records) => Ok(records),
        Err(OmvError::Finalization(FinalizationError::Missing { .. })) => {
            Ok(OmvFinalizations::default())
        }
        Err(err) => Err(err),
    }
}

pub fn save_finalizations(root: &Path, finalizations: &OmvFinalizations) -> Result<(), OmvError> {
    let path = path_for(root);
    let content =
        toml::to_string_pretty(finalizations).map_err(|err| FinalizationError::Parse {
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

    use crate::core::finalization::{
        ChangeType, FinalizationOutcome, FinalizationReason, TaskStatus, TestsStatus,
    };
    use crate::core::schema::{OmvFinalizationRecord, OmvFinalizations};
    use crate::errors::{FinalizationError, OmvError};

    use super::{load_finalizations, load_finalizations_if_exists, save_finalizations};

    #[test]
    fn finalizations_round_trip_preserves_entries() {
        let root = temp_omv_root("finalizations-roundtrip");
        let finalizations = OmvFinalizations {
            schema_version: 1,
            entries: vec![OmvFinalizationRecord {
                task_id: "04-18-product-gaps-automation-hooks".to_owned(),
                fingerprint: "task-1".to_owned(),
                change_type: ChangeType::Bugfix,
                task_status: TaskStatus::Done,
                tests_status: TestsStatus::Passed,
                source: "trellis-finish-work".to_owned(),
                outcome: FinalizationOutcome::Bumped,
                reason: FinalizationReason::SemanticChange,
                version_before: "2604.13.1".to_owned(),
                version_after: "2604.13.2".to_owned(),
                recorded_at: "1713446400".to_owned(),
            }],
        };

        save_finalizations(&root, &finalizations).expect("finalizations should save");
        let loaded = load_finalizations(&root).expect("finalizations should load");
        assert_eq!(loaded, finalizations);

        cleanup_root(&root);
    }

    #[test]
    fn missing_finalizations_defaults_to_empty_registry() {
        let root = temp_omv_root("finalizations-missing");
        let loaded = load_finalizations_if_exists(&root).expect("missing file should default");
        assert_eq!(loaded, OmvFinalizations::default());

        cleanup_root(&root);
    }

    #[test]
    fn missing_finalizations_can_return_typed_error() {
        let root = temp_omv_root("finalizations-missing-error");
        let err = load_finalizations(&root).expect_err("missing file should error");
        assert!(matches!(
            err,
            OmvError::Finalization(FinalizationError::Missing { .. })
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
