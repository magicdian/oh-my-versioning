use std::fs;
use std::path::{Path, PathBuf};

use crate::core::schema::{OmvTargetRecord, OmvTargets};
use crate::core::target::TargetLanguage;
use crate::errors::OmvError;

pub mod c_family;
pub mod go;
pub mod java;
pub mod python;
pub mod rust;
pub mod skills;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncSummary {
    pub synced: usize,
    pub skipped: usize,
}

pub struct SyncContext<'a> {
    pub project_root: &'a Path,
    pub target: &'a OmvTargetRecord,
    pub version: &'a str,
}

pub trait TargetSyncAdapter {
    fn language_key(&self) -> &'static str;
    fn sync(&self, context: &SyncContext<'_>) -> Result<(), OmvError>;
}

pub fn sync_all_targets(
    project_root: &Path,
    targets: &OmvTargets,
    version: &str,
) -> Result<SyncSummary, OmvError> {
    let mut synced = 0;
    let mut skipped = 0;

    for target in &targets.targets {
        if !target.enabled {
            skipped += 1;
            continue;
        }

        let context = SyncContext {
            project_root,
            target,
            version,
        };

        adapter_for_language(target.language)?.sync(&context)?;
        synced += 1;
    }

    Ok(SyncSummary { synced, skipped })
}

fn adapter_for_language(language: TargetLanguage) -> Result<Box<dyn TargetSyncAdapter>, OmvError> {
    let adapter: Box<dyn TargetSyncAdapter> = match language {
        TargetLanguage::Rust => Box::new(rust::RustSyncAdapter),
        TargetLanguage::Python => Box::new(python::PythonSyncAdapter),
        TargetLanguage::Go => Box::new(go::GoSyncAdapter),
        TargetLanguage::Java => Box::new(java::JavaSyncAdapter),
        TargetLanguage::CFamily => Box::new(c_family::CFamilySyncAdapter),
    };

    Ok(adapter)
}

pub(crate) fn resolve_target_path(
    project_root: &Path,
    target_root: &str,
    relative_path: &str,
) -> PathBuf {
    project_root.join(target_root).join(relative_path)
}

pub(crate) fn read_text_if_exists(path: &Path) -> Result<Option<String>, OmvError> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

pub(crate) fn write_text(path: &Path, content: &str) -> Result<(), OmvError> {
    crate::storage::atomic::write_atomically(path, content.as_bytes())
}

pub(crate) fn replace_or_append_line(content: &str, prefix: &str, new_line: &str) -> String {
    let mut replaced = false;
    let mut lines = Vec::new();

    for line in content.lines() {
        if !replaced && line.trim_start().starts_with(prefix) {
            lines.push(new_line.to_owned());
            replaced = true;
        } else {
            lines.push(line.to_owned());
        }
    }

    if !replaced {
        if !content.trim().is_empty() {
            lines.push(String::new());
        }
        lines.push(new_line.to_owned());
    }

    let mut output = lines.join("\n");
    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use crate::core::schema::{OmvTargetRecord, OmvTargets};
    use crate::core::target::TargetLanguage;

    use super::{replace_or_append_line, sync_all_targets};

    #[test]
    fn replace_or_append_updates_existing_line() {
        let input = "[package]\nname = \"omv\"\nversion = \"0.1.0\"\n";
        let output = replace_or_append_line(input, "version =", "version = \"1.2.3\"");

        assert!(output.contains("version = \"1.2.3\""));
        assert!(!output.contains("version = \"0.1.0\""));
    }

    #[test]
    fn replace_or_append_adds_line_when_missing() {
        let input = "[package]\nname = \"omv\"\n";
        let output = replace_or_append_line(input, "version =", "version = \"1.2.3\"");

        assert!(output.contains("version = \"1.2.3\""));
    }

    #[test]
    fn sync_all_targets_counts_skipped_targets() {
        let targets = OmvTargets {
            schema_version: 1,
            targets: vec![OmvTargetRecord {
                id: "workspace-rust".to_owned(),
                language: TargetLanguage::Rust,
                root: ".".to_owned(),
                manifest_path: "Cargo.toml".to_owned(),
                runtime_export_path: "src/generated/version.rs".to_owned(),
                strategy: crate::core::target::PreProjectStrategy::IntentOnly,
                enabled: false,
            }],
        };

        let summary = sync_all_targets(std::path::Path::new("."), &targets, "2604.13.1")
            .expect("sync should succeed");
        assert_eq!(summary.synced, 0);
        assert_eq!(summary.skipped, 1);
    }
}
