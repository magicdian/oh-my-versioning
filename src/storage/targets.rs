use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;
use toml::value::Table;

use crate::core::schema::{
    CHeaderMacroTarget, CargoWorkspaceTarget, MarkdownManagedBlockTarget, OmvTargetRecord,
    OmvTargets, OmvUnsupportedTargetRecord, OmvV2TargetConfig, OmvV2TargetRecord,
    RegexReplaceTarget, TextScalarTarget, YamlScalarTarget,
};
use crate::core::target::{
    CargoLockfileStrategy, CargoMembers, CargoVersionLocation, CargoVersionPolicy,
    PreProjectStrategy, TargetKind, TargetLanguage, TargetMode,
};
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

    let parsed = content.parse::<Value>().map_err(|err| TargetError::Parse {
        path: path.clone(),
        reason: err.to_string(),
    })?;
    let table = parsed.as_table().ok_or_else(|| TargetError::Parse {
        path: path.clone(),
        reason: String::from("targets file root must be a TOML table"),
    })?;

    let schema_version = optional_u32(table, "schema_version")?.unwrap_or(1);
    if schema_version == 0 {
        return Err(TargetError::InvalidTargetRecord(format!(
            "unsupported targets schema_version: {schema_version}"
        ))
        .into());
    }

    let mut targets = OmvTargets {
        schema_version,
        targets: Vec::new(),
        v2_targets: Vec::new(),
        unsupported_targets: Vec::new(),
    };

    let Some(records) = table.get("targets") else {
        return Ok(targets);
    };
    let records = records.as_array().ok_or_else(|| {
        TargetError::InvalidTargetRecord(String::from("targets must be an array of tables"))
    })?;

    for value in records {
        let record = value.as_table().ok_or_else(|| {
            TargetError::InvalidTargetRecord(String::from("target entry must be a table"))
        })?;

        if record.contains_key("kind") {
            let kind_value = required_string(record, "kind")?;
            if TargetKind::parse(&kind_value).is_some() {
                targets.v2_targets.push(parse_v2_record(record)?);
            } else {
                targets
                    .unsupported_targets
                    .push(parse_unsupported_record(record, kind_value)?);
            }
        } else {
            targets.targets.push(parse_v1_record(record)?);
        }
    }

    Ok(targets)
}

pub fn save_targets(root: &Path, targets: &OmvTargets) -> Result<(), OmvError> {
    let path = path_for(root);
    let schema_version = if targets.v2_targets.is_empty() && targets.unsupported_targets.is_empty()
    {
        targets.schema_version
    } else {
        2
    };
    let mut content = format!("schema_version = {schema_version}\n\n");

    for record in &targets.targets {
        content.push_str("[[targets]]\n");
        content.push_str(&format!("id = \"{}\"\n", escape_toml(&record.id)));
        content.push_str(&format!("language = \"{}\"\n", record.language.as_str()));
        content.push_str(&format!("root = \"{}\"\n", escape_toml(&record.root)));
        content.push_str(&format!(
            "manifest_path = \"{}\"\n",
            escape_toml(&record.manifest_path)
        ));
        content.push_str(&format!(
            "runtime_export_path = \"{}\"\n",
            escape_toml(&record.runtime_export_path)
        ));
        content.push_str(&format!("strategy = \"{}\"\n", record.strategy.as_str()));
        content.push_str(&format!("enabled = {}\n\n", record.enabled));
    }

    for record in &targets.v2_targets {
        content.push_str("[[targets]]\n");
        content.push_str(&format!("id = \"{}\"\n", escape_toml(&record.id)));
        content.push_str(&format!("kind = \"{}\"\n", record.kind.as_str()));
        content.push_str(&format!("adapter = \"{}\"\n", escape_toml(&record.adapter)));
        content.push_str(&format!("root = \"{}\"\n", escape_toml(&record.root)));
        content.push_str(&format!("enabled = {}\n", record.enabled));
        content.push_str(&format!("mode = \"{}\"\n", record.mode.as_str()));

        match &record.config {
            OmvV2TargetConfig::TextScalar(config) => {
                write_common_path_template(&mut content, &config.path, &config.template);
                content.push_str(&format!(
                    "selector = \"{}\"\n",
                    escape_toml(&config.selector)
                ));
            }
            OmvV2TargetConfig::RegexReplace(config) => {
                write_common_path_template(&mut content, &config.path, &config.template);
                content.push_str(&format!("pattern = \"{}\"\n", escape_toml(&config.pattern)));
                content.push_str(&format!("allow_multiple = {}\n", config.allow_multiple));
            }
            OmvV2TargetConfig::MarkdownManagedBlock(config) => {
                write_common_path_template(&mut content, &config.path, &config.template);
                content.push_str(&format!(
                    "begin_marker = \"{}\"\n",
                    escape_toml(&config.begin_marker)
                ));
                content.push_str(&format!(
                    "end_marker = \"{}\"\n",
                    escape_toml(&config.end_marker)
                ));
            }
            OmvV2TargetConfig::YamlScalar(config) => {
                write_common_path_template(&mut content, &config.path, &config.template);
                content.push_str(&format!("key = \"{}\"\n", escape_toml(&config.key)));
            }
            OmvV2TargetConfig::CHeaderMacro(config) => {
                write_common_path_template(&mut content, &config.path, &config.template);
                content.push_str(&format!(
                    "macro = \"{}\"\n",
                    escape_toml(&config.macro_name)
                ));
            }
            OmvV2TargetConfig::CargoWorkspace(config) => {
                content.push_str(&format!("members = \"{}\"\n", config.members.as_str()));
                content.push_str(&format!(
                    "version_policy = \"{}\"\n",
                    config.version_policy.as_str()
                ));
                content.push_str(&format!(
                    "version_location = \"{}\"\n",
                    config.version_location.as_str()
                ));
                content.push_str(&format!("lockfile = \"{}\"\n", config.lockfile.as_str()));
            }
        }
        content.push('\n');
    }

    for record in &targets.unsupported_targets {
        content.push_str("[[targets]]\n");
        content.push_str(&format!("id = \"{}\"\n", escape_toml(&record.id)));
        content.push_str(&format!("kind = \"{}\"\n", escape_toml(&record.kind)));
        content.push_str(&format!("adapter = \"{}\"\n", escape_toml(&record.adapter)));
        content.push_str(&format!("root = \"{}\"\n", escape_toml(&record.root)));
        content.push_str(&format!("enabled = {}\n", record.enabled));
        for path in &record.paths {
            content.push_str(&format!("path = \"{}\"\n", escape_toml(path)));
        }
        content.push('\n');
    }

    atomic::write_atomically(&path, content.as_bytes())
}

fn parse_v1_record(record: &Table) -> Result<OmvTargetRecord, OmvError> {
    let id = required_string(record, "id")?;
    let language_value = required_string(record, "language")?;
    let language = TargetLanguage::parse(&language_value).ok_or_else(|| {
        TargetError::InvalidTargetRecord(format!(
            "target {id}: unsupported language: {language_value}"
        ))
    })?;
    let strategy_value = optional_string(record, "strategy")?
        .unwrap_or_else(|| PreProjectStrategy::IntentOnly.as_str().to_owned());
    let strategy = PreProjectStrategy::parse(&strategy_value).ok_or_else(|| {
        TargetError::InvalidTargetRecord(format!(
            "target {id}: unsupported strategy: {strategy_value}"
        ))
    })?;

    Ok(OmvTargetRecord {
        id,
        language,
        root: optional_string(record, "root")?.unwrap_or_else(|| String::from(".")),
        manifest_path: required_string(record, "manifest_path")?,
        runtime_export_path: required_string(record, "runtime_export_path")?,
        strategy,
        enabled: optional_bool(record, "enabled")?.unwrap_or(true),
    })
}

fn parse_v2_record(record: &Table) -> Result<OmvV2TargetRecord, OmvError> {
    let id = required_string(record, "id")?;
    let kind_value = required_string(record, "kind")?;
    let kind = TargetKind::parse(&kind_value).ok_or_else(|| {
        TargetError::InvalidTargetRecord(format!("target {id}: unsupported kind: {kind_value}"))
    })?;
    let mode_value =
        optional_string(record, "mode")?.unwrap_or_else(|| TargetMode::Write.as_str().to_owned());
    let mode = TargetMode::parse(&mode_value).ok_or_else(|| {
        TargetError::InvalidTargetRecord(format!("target {id}: unsupported mode: {mode_value}"))
    })?;
    let adapter =
        optional_string(record, "adapter")?.unwrap_or_else(|| default_adapter(kind).to_owned());
    let root = optional_string(record, "root")?.unwrap_or_else(|| String::from("."));

    let config = match kind {
        TargetKind::TextScalar => {
            let selector =
                optional_string(record, "selector")?.unwrap_or_else(|| String::from("whole-file"));
            if selector != "whole-file" {
                return Err(TargetError::InvalidTargetRecord(format!(
                    "target {id}: text-scalar selector must be whole-file"
                ))
                .into());
            }
            OmvV2TargetConfig::TextScalar(TextScalarTarget {
                path: required_string(record, "path")?,
                selector,
                template: required_string(record, "template")?,
            })
        }
        TargetKind::RegexReplace => OmvV2TargetConfig::RegexReplace(RegexReplaceTarget {
            path: required_string(record, "path")?,
            pattern: required_string(record, "pattern")?,
            template: required_string(record, "template")?,
            allow_multiple: optional_bool(record, "allow_multiple")?.unwrap_or(false),
        }),
        TargetKind::MarkdownManagedBlock => {
            OmvV2TargetConfig::MarkdownManagedBlock(MarkdownManagedBlockTarget {
                path: required_string(record, "path")?,
                begin_marker: required_string(record, "begin_marker")?,
                end_marker: required_string(record, "end_marker")?,
                template: required_string(record, "template")?,
            })
        }
        TargetKind::YamlScalar => OmvV2TargetConfig::YamlScalar(YamlScalarTarget {
            path: required_string(record, "path")?,
            key: required_string(record, "key")?,
            template: required_string(record, "template")?,
        }),
        TargetKind::CHeaderMacro => OmvV2TargetConfig::CHeaderMacro(CHeaderMacroTarget {
            path: required_string(record, "path")?,
            macro_name: required_string(record, "macro")?,
            template: required_string(record, "template")?,
        }),
        TargetKind::CargoWorkspace => {
            let members_value = optional_string(record, "members")?
                .unwrap_or_else(|| CargoMembers::All.as_str().to_owned());
            let members = CargoMembers::parse(&members_value).ok_or_else(|| {
                TargetError::InvalidTargetRecord(format!(
                    "target {id}: unsupported cargo members: {members_value}"
                ))
            })?;
            let version_policy_value = optional_string(record, "version_policy")?
                .unwrap_or_else(|| CargoVersionPolicy::Same.as_str().to_owned());
            let version_policy =
                CargoVersionPolicy::parse(&version_policy_value).ok_or_else(|| {
                    TargetError::InvalidTargetRecord(format!(
                        "target {id}: unsupported cargo version_policy: {version_policy_value}"
                    ))
                })?;
            let version_location_value = optional_string(record, "version_location")?
                .unwrap_or_else(|| CargoVersionLocation::Auto.as_str().to_owned());
            let version_location = CargoVersionLocation::parse(&version_location_value)
                .ok_or_else(|| {
                    TargetError::InvalidTargetRecord(format!(
                        "target {id}: unsupported cargo version_location: {version_location_value}"
                    ))
                })?;
            let lockfile_value = optional_string(record, "lockfile")?
                .unwrap_or_else(|| CargoLockfileStrategy::Check.as_str().to_owned());
            let lockfile = CargoLockfileStrategy::parse(&lockfile_value).ok_or_else(|| {
                TargetError::InvalidTargetRecord(format!(
                    "target {id}: unsupported lockfile: {lockfile_value}"
                ))
            })?;
            OmvV2TargetConfig::CargoWorkspace(CargoWorkspaceTarget {
                root: root.clone(),
                members,
                version_policy,
                version_location,
                lockfile,
            })
        }
    };

    Ok(OmvV2TargetRecord {
        id,
        kind,
        adapter,
        root,
        enabled: optional_bool(record, "enabled")?.unwrap_or(true),
        mode,
        config,
    })
}

fn parse_unsupported_record(
    record: &Table,
    kind: String,
) -> Result<OmvUnsupportedTargetRecord, OmvError> {
    let id = required_string(record, "id")?;
    let adapter = optional_string(record, "adapter")?.unwrap_or_else(|| String::from("unknown"));
    let root = optional_string(record, "root")?.unwrap_or_else(|| String::from("."));
    let paths = optional_string(record, "path")?
        .map(|path| vec![path])
        .unwrap_or_default();

    Ok(OmvUnsupportedTargetRecord {
        id,
        kind,
        adapter,
        root,
        enabled: optional_bool(record, "enabled")?.unwrap_or(true),
        paths,
    })
}

fn required_string(record: &Table, key: &str) -> Result<String, OmvError> {
    optional_string(record, key)?.ok_or_else(|| {
        TargetError::InvalidTargetRecord(format!("missing required target field: {key}")).into()
    })
}

fn optional_string(record: &Table, key: &str) -> Result<Option<String>, OmvError> {
    match record.get(key) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(value) => Err(TargetError::InvalidTargetRecord(format!(
            "field {key} must be a string, got {}",
            value.type_str()
        ))
        .into()),
        None => Ok(None),
    }
}

fn optional_bool(record: &Table, key: &str) -> Result<Option<bool>, OmvError> {
    match record.get(key) {
        Some(Value::Boolean(value)) => Ok(Some(*value)),
        Some(value) => Err(TargetError::InvalidTargetRecord(format!(
            "field {key} must be a bool, got {}",
            value.type_str()
        ))
        .into()),
        None => Ok(None),
    }
}

fn optional_u32(record: &Table, key: &str) -> Result<Option<u32>, OmvError> {
    match record.get(key) {
        Some(Value::Integer(value)) if *value >= 0 => Ok(Some(*value as u32)),
        Some(Value::Integer(value)) => Err(TargetError::InvalidTargetRecord(format!(
            "field {key} must be non-negative, got {value}"
        ))
        .into()),
        Some(value) => Err(TargetError::InvalidTargetRecord(format!(
            "field {key} must be an integer, got {}",
            value.type_str()
        ))
        .into()),
        None => Ok(None),
    }
}

fn default_adapter(kind: TargetKind) -> &'static str {
    match kind {
        TargetKind::TextScalar => "text",
        TargetKind::RegexReplace => "markdown",
        TargetKind::MarkdownManagedBlock => "markdown",
        TargetKind::YamlScalar => "yaml",
        TargetKind::CHeaderMacro => "c-header",
        TargetKind::CargoWorkspace => "cargo",
    }
}

fn write_common_path_template(content: &mut String, path: &str, template: &str) {
    content.push_str(&format!("path = \"{}\"\n", escape_toml(path)));
    content.push_str(&format!("template = \"{}\"\n", escape_toml(template)));
}

fn escape_toml(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::core::schema::{
        OmvTargetRecord, OmvTargets, OmvV2TargetConfig, OmvV2TargetRecord, TextScalarTarget,
    };
    use crate::core::target::{PreProjectStrategy, TargetKind, TargetLanguage, TargetMode};

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
            v2_targets: Vec::new(),
            unsupported_targets: Vec::new(),
        };

        save_targets(&root, &targets).expect("targets should save");
        let loaded = load_targets(&root).expect("targets should load");
        assert_eq!(loaded, targets);

        cleanup_root(&root);
    }

    #[test]
    fn v2_text_scalar_target_round_trips() {
        let root = temp_omv_root("targets-v2-roundtrip");
        let targets = OmvTargets {
            schema_version: 2,
            targets: Vec::new(),
            v2_targets: vec![OmvV2TargetRecord {
                id: "root-version-file".to_owned(),
                kind: TargetKind::TextScalar,
                adapter: "text".to_owned(),
                root: ".".to_owned(),
                enabled: true,
                mode: TargetMode::Write,
                config: OmvV2TargetConfig::TextScalar(TextScalarTarget {
                    path: "VERSION".to_owned(),
                    selector: "whole-file".to_owned(),
                    template: "{version}\n".to_owned(),
                }),
            }],
            unsupported_targets: Vec::new(),
        };

        save_targets(&root, &targets).expect("targets should save");
        let loaded = load_targets(&root).expect("targets should load");
        assert_eq!(loaded, targets);

        cleanup_root(&root);
    }

    #[test]
    fn v2_rejects_malformed_required_fields() {
        let root = temp_omv_root("targets-v2-invalid");
        fs::write(
            root.join("targets.toml"),
            "schema_version = 2\n\n[[targets]]\nid = \"bad\"\nkind = \"yaml-scalar\"\npath = \"component.yml\"\n",
        )
        .expect("target fixture should write");

        let err = load_targets(&root).expect_err("missing key should fail");
        assert_eq!(err.code(), "invalid_target_record");

        cleanup_root(&root);
    }

    #[test]
    fn kind_targets_load_without_schema_v2_gate() {
        let root = temp_omv_root("targets-kind-no-schema-gate");
        fs::write(
            root.join("targets.toml"),
            "[[targets]]\nid = \"root-version-file\"\nkind = \"text-scalar\"\npath = \"VERSION\"\ntemplate = \"{version}\\n\"\n",
        )
        .expect("target fixture should write");

        let loaded = load_targets(&root).expect("kind target should load without schema_version");
        assert_eq!(loaded.schema_version, 1);
        assert_eq!(loaded.v2_targets.len(), 1);
        assert!(loaded.unsupported_targets.is_empty());

        cleanup_root(&root);
    }

    #[test]
    fn unknown_kind_loads_as_unsupported_target() {
        let root = temp_omv_root("targets-unknown-kind");
        fs::write(
            root.join("targets.toml"),
            "schema_version = 99\n\n[[targets]]\nid = \"future-workspace\"\nkind = \"future-workspace\"\npath = \"future.toml\"\n",
        )
        .expect("target fixture should write");

        let loaded = load_targets(&root).expect("unknown kind should not block target loading");
        assert_eq!(loaded.schema_version, 99);
        assert_eq!(loaded.unsupported_targets.len(), 1);
        assert_eq!(loaded.unsupported_targets[0].kind, "future-workspace");
        assert_eq!(loaded.unsupported_targets[0].paths, vec!["future.toml"]);

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
