use std::fs;
use std::path::{Path, PathBuf};

use crate::core::schema::{OmvTargetRecord, OmvTargets};
use crate::core::target::{PreProjectStrategy, TargetLanguage};
use crate::errors::{OmvError, TargetError};

use super::{TARGETS_FILE, atomic};

pub fn path_for(root: &Path) -> PathBuf {
    root.join(TARGETS_FILE)
}

pub fn load_targets(root: &Path) -> Result<OmvTargets, OmvError> {
    let path = path_for(root);
    let content = fs::read_to_string(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            TargetError::Missing { path: path.clone() }
        } else {
            TargetError::Parse {
                path: path.clone(),
                reason: err.to_string(),
            }
        }
    })?;

    let mut targets = OmvTargets::default();
    let mut current: Option<OmvTargetRecord> = None;

    for raw in content.lines().map(str::trim) {
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }

        if raw == "[[targets]]" {
            if let Some(record) = current.take() {
                targets.targets.push(record);
            }
            current = Some(OmvTargetRecord::new("", TargetLanguage::Rust));
            continue;
        }

        let Some((key, value)) = raw.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"');

        if key == "schema_version" {
            targets.schema_version = value.parse::<u32>().map_err(|e| TargetError::Parse {
                path: path.clone(),
                reason: format!("invalid schema_version: {e}"),
            })?;
            continue;
        }

        let Some(record) = current.as_mut() else {
            continue;
        };

        match key {
            "id" => record.id = value.to_owned(),
            "language" => {
                record.language = TargetLanguage::parse(value).ok_or_else(|| {
                    TargetError::InvalidTargetRecord(format!("unsupported language: {value}"))
                })?
            }
            "root" => record.root = value.to_owned(),
            "manifest_path" => record.manifest_path = value.to_owned(),
            "runtime_export_path" => record.runtime_export_path = value.to_owned(),
            "strategy" => {
                record.strategy = PreProjectStrategy::parse(value).ok_or_else(|| {
                    TargetError::InvalidTargetRecord(format!("unsupported strategy: {value}"))
                })?
            }
            "enabled" => {
                record.enabled = value.parse::<bool>().map_err(|e| {
                    TargetError::InvalidTargetRecord(format!("invalid enabled value: {e}"))
                })?
            }
            _ => {}
        }
    }

    if let Some(record) = current.take() {
        targets.targets.push(record);
    }

    for record in &targets.targets {
        if record.id.is_empty() {
            return Err(TargetError::InvalidTargetRecord(String::from(
                "target id cannot be empty",
            ))
            .into());
        }
    }

    Ok(targets)
}

pub fn save_targets(root: &Path, targets: &OmvTargets) -> Result<(), OmvError> {
    let path = path_for(root);
    let mut content = format!("schema_version = {}\n\n", targets.schema_version);

    for record in &targets.targets {
        content.push_str("[[targets]]\n");
        content.push_str(&format!("id = \"{}\"\n", record.id));
        content.push_str(&format!("language = \"{}\"\n", record.language.as_str()));
        content.push_str(&format!("root = \"{}\"\n", record.root));
        content.push_str(&format!("manifest_path = \"{}\"\n", record.manifest_path));
        content.push_str(&format!(
            "runtime_export_path = \"{}\"\n",
            record.runtime_export_path
        ));
        content.push_str(&format!("strategy = \"{}\"\n", record.strategy.as_str()));
        content.push_str(&format!("enabled = {}\n\n", record.enabled));
    }

    atomic::write_atomically(&path, content.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::core::schema::{OmvTargetRecord, OmvTargets};
    use crate::core::target::{PreProjectStrategy, TargetLanguage};

    use super::{load_targets, save_targets};

    #[test]
    fn targets_round_trip_supports_multiple_flat_records() {
        let root = temp_omv_root("targets-roundtrip");

        let rust_target = OmvTargetRecord {
            id: "workspace-rust".to_owned(),
            language: TargetLanguage::Rust,
            root: ".".to_owned(),
            manifest_path: "Cargo.toml".to_owned(),
            runtime_export_path: "src/generated/version.rs".to_owned(),
            strategy: PreProjectStrategy::IntentOnly,
            enabled: true,
        };
        let python_target = OmvTargetRecord {
            id: "workspace-python".to_owned(),
            language: TargetLanguage::Python,
            root: ".".to_owned(),
            manifest_path: "pyproject.toml".to_owned(),
            runtime_export_path: "app/generated/version.py".to_owned(),
            strategy: PreProjectStrategy::InitExportTemplates,
            enabled: false,
        };
        let targets = OmvTargets {
            schema_version: 1,
            targets: vec![rust_target, python_target],
        };

        save_targets(&root, &targets).expect("targets should save");
        let loaded = load_targets(&root).expect("targets should load");
        assert_eq!(loaded, targets);

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
