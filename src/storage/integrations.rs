use std::fs;
use std::path::{Path, PathBuf};

use crate::core::integration::OmvIntegrations;
use crate::errors::{IntegrationError, OmvError};

use super::{INTEGRATIONS_FILE, atomic};

pub fn path_for(root: &Path) -> PathBuf {
    root.join(INTEGRATIONS_FILE)
}

pub fn load_integrations(root: &Path) -> Result<OmvIntegrations, OmvError> {
    let path = path_for(root);
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(OmvIntegrations::default());
        }
        Err(err) => {
            return Err(IntegrationError::Parse {
                path,
                reason: err.to_string(),
            }
            .into());
        }
    };

    toml::from_str(&content).map_err(|err| {
        IntegrationError::Parse {
            path,
            reason: err.to_string(),
        }
        .into()
    })
}

pub fn save_integrations(root: &Path, integrations: &OmvIntegrations) -> Result<(), OmvError> {
    let path = path_for(root);
    let content = toml::to_string_pretty(integrations).map_err(|err| IntegrationError::Parse {
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

    use crate::core::integration::{
        IntegrationCapability, IntegrationCapabilityStatus, IntegrationDetectionSnapshot,
        IntegrationFailure, IntegrationProvider, OmvIntegrationCapabilityState,
        OmvIntegrationProviderState, OmvIntegrations,
    };
    use crate::errors::{IntegrationError, OmvError};

    use super::{load_integrations, path_for, save_integrations};

    #[test]
    fn integrations_round_trip_preserves_provider_state() {
        let root = temp_omv_root("integrations-roundtrip");
        let integrations = OmvIntegrations {
            schema_version: 1,
            providers: vec![
                OmvIntegrationProviderState {
                    provider: IntegrationProvider::Codex,
                    selected: true,
                    detection: IntegrationDetectionSnapshot {
                        detected: true,
                        recommended: true,
                    },
                    capabilities: vec![
                        OmvIntegrationCapabilityState {
                            capability: IntegrationCapability::ProjectInstructions,
                            selected: true,
                            status: IntegrationCapabilityStatus::Installed,
                            failure: None,
                        },
                        OmvIntegrationCapabilityState {
                            capability: IntegrationCapability::HostSkill,
                            selected: true,
                            status: IntegrationCapabilityStatus::Pending,
                            failure: None,
                        },
                    ],
                },
                OmvIntegrationProviderState {
                    provider: IntegrationProvider::Trellis,
                    selected: true,
                    detection: IntegrationDetectionSnapshot {
                        detected: false,
                        recommended: true,
                    },
                    capabilities: vec![OmvIntegrationCapabilityState {
                        capability: IntegrationCapability::FinalizeBoundary,
                        selected: true,
                        status: IntegrationCapabilityStatus::Failed,
                        failure: Some(IntegrationFailure {
                            reason_code: String::from("provider_not_detected"),
                            display_message: String::from("Trellis was not detected"),
                        }),
                    }],
                },
            ],
        };

        save_integrations(&root, &integrations).expect("integrations should save");
        let loaded = load_integrations(&root).expect("integrations should load");
        assert_eq!(loaded, integrations);

        cleanup_root(&root);
    }

    #[test]
    fn absent_integrations_file_defaults_to_empty_state() {
        let root = temp_omv_root("integrations-missing");
        let loaded = load_integrations(&root).expect("missing file should default");
        assert_eq!(loaded, OmvIntegrations::default());

        cleanup_root(&root);
    }

    #[test]
    fn malformed_integrations_toml_returns_typed_error() {
        let root = temp_omv_root("integrations-malformed");
        fs::write(path_for(&root), "schema_version = \"bad\"\n")
            .expect("fixture should be written");

        let err = load_integrations(&root).expect_err("malformed file should fail");
        assert!(matches!(
            err,
            OmvError::Integration(IntegrationError::Parse { .. })
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
