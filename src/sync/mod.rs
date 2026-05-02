use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::contract::generated::omv::contract::v1::OmvPlanStatus;
use crate::contract::registry::{CapabilityRegistry, stage1_registry};
use crate::core::schema::{
    OmvTargetRecord, OmvTargets, OmvUnsupportedTargetRecord, OmvV2TargetRecord,
};
use crate::core::target::{TargetKind, TargetLanguage, TargetMode};
use crate::errors::{OmvError, TargetError};

pub mod c_family;
pub mod cargo_workspace;
pub mod generic;
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

pub struct V2SyncContext<'a> {
    pub project_root: &'a Path,
    pub target: &'a OmvV2TargetRecord,
    pub version: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanSummary {
    pub contract_version: u32,
    pub version: String,
    pub project_root: String,
    pub project_status: String,
    pub migration_status: Vec<String>,
    pub totals: PlanTotals,
    pub targets: Vec<PlanTargetResult>,
}

impl PlanSummary {
    pub fn has_required_drift(&self) -> bool {
        self.targets
            .iter()
            .any(|target| target.required && target.status.is_failure())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanTotals {
    pub ok: usize,
    pub drift: usize,
    pub missing: usize,
    pub unsupported: usize,
    pub error: usize,
    pub skipped: usize,
}

impl PlanTotals {
    fn from_targets(targets: &[PlanTargetResult]) -> Self {
        let mut totals = Self {
            ok: 0,
            drift: 0,
            missing: 0,
            unsupported: 0,
            error: 0,
            skipped: 0,
        };

        for target in targets {
            match target.status {
                PlanStatus::Ok => totals.ok += 1,
                PlanStatus::Drift => totals.drift += 1,
                PlanStatus::Missing => totals.missing += 1,
                PlanStatus::Unsupported => totals.unsupported += 1,
                PlanStatus::Error => totals.error += 1,
                PlanStatus::Skipped => totals.skipped += 1,
            }
        }

        totals
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanTargetResult {
    pub id: String,
    pub adapter: String,
    pub kind: String,
    pub language: String,
    pub paths: Vec<String>,
    pub current_value_summary: String,
    pub expected_value_summary: String,
    pub status: PlanStatus,
    pub operations: Vec<PlanOperation>,
    pub diagnostics: Vec<String>,
    pub required: bool,
}

impl PlanTargetResult {
    fn skipped(target: &OmvTargetRecord) -> Self {
        Self {
            id: target.id.clone(),
            adapter: target.language.as_str().to_owned(),
            kind: format!("v1-{}", target.language.as_str()),
            language: target.language.as_str().to_owned(),
            paths: vec![
                target.manifest_path.clone(),
                target.runtime_export_path.clone(),
            ],
            current_value_summary: String::from("target disabled"),
            expected_value_summary: String::from("no write expected"),
            status: PlanStatus::Skipped,
            operations: Vec::new(),
            diagnostics: vec![String::from("target is disabled")],
            required: false,
        }
    }

    fn unsupported(target: &OmvTargetRecord, diagnostic: String) -> Self {
        Self {
            id: target.id.clone(),
            adapter: target.language.as_str().to_owned(),
            kind: format!("v1-{}", target.language.as_str()),
            language: target.language.as_str().to_owned(),
            paths: vec![
                target.manifest_path.clone(),
                target.runtime_export_path.clone(),
            ],
            current_value_summary: String::from("unsupported"),
            expected_value_summary: String::from("unsupported"),
            status: PlanStatus::Unsupported,
            operations: Vec::new(),
            diagnostics: vec![diagnostic],
            required: true,
        }
    }

    fn errored(target: &OmvTargetRecord, err: OmvError) -> Self {
        Self {
            id: target.id.clone(),
            adapter: target.language.as_str().to_owned(),
            kind: format!("v1-{}", target.language.as_str()),
            language: target.language.as_str().to_owned(),
            paths: vec![
                target.manifest_path.clone(),
                target.runtime_export_path.clone(),
            ],
            current_value_summary: String::from("error"),
            expected_value_summary: String::from("unknown"),
            status: PlanStatus::Error,
            operations: Vec::new(),
            diagnostics: vec![err.to_string()],
            required: true,
        }
    }

    fn skipped_v2(target: &OmvV2TargetRecord) -> Self {
        Self {
            id: target.id.clone(),
            adapter: target.adapter.clone(),
            kind: target.kind.as_str().to_owned(),
            language: String::new(),
            paths: target
                .path()
                .map(|path| vec![path.to_owned()])
                .unwrap_or_default(),
            current_value_summary: String::from("target disabled"),
            expected_value_summary: String::from("no write expected"),
            status: PlanStatus::Skipped,
            operations: Vec::new(),
            diagnostics: vec![String::from("target is disabled")],
            required: false,
        }
    }

    fn unsupported_v2(target: &OmvV2TargetRecord, diagnostic: String) -> Self {
        Self {
            id: target.id.clone(),
            adapter: target.adapter.clone(),
            kind: target.kind.as_str().to_owned(),
            language: String::new(),
            paths: target
                .path()
                .map(|path| vec![path.to_owned()])
                .unwrap_or_default(),
            current_value_summary: String::from("unsupported"),
            expected_value_summary: String::from("unsupported"),
            status: PlanStatus::Unsupported,
            operations: Vec::new(),
            diagnostics: vec![diagnostic],
            required: true,
        }
    }

    fn unsupported_kind(target: &OmvUnsupportedTargetRecord) -> Self {
        Self {
            id: target.id.clone(),
            adapter: target.adapter.clone(),
            kind: target.kind.clone(),
            language: String::new(),
            paths: target.paths.clone(),
            current_value_summary: String::from("unsupported"),
            expected_value_summary: String::from("unsupported"),
            status: PlanStatus::Unsupported,
            operations: Vec::new(),
            diagnostics: vec![format!(
                "current OMV binary does not support target kind {}; update OMV to use this capability",
                target.kind
            )],
            required: target.enabled,
        }
    }

    fn skipped_unsupported_kind(target: &OmvUnsupportedTargetRecord) -> Self {
        Self {
            id: target.id.clone(),
            adapter: target.adapter.clone(),
            kind: target.kind.clone(),
            language: String::new(),
            paths: target.paths.clone(),
            current_value_summary: String::from("target disabled"),
            expected_value_summary: String::from("no write expected"),
            status: PlanStatus::Skipped,
            operations: Vec::new(),
            diagnostics: vec![format!(
                "target kind {} is not supported by this OMV binary but target is disabled",
                target.kind
            )],
            required: false,
        }
    }

    fn errored_v2(target: &OmvV2TargetRecord, err: OmvError) -> Self {
        Self {
            id: target.id.clone(),
            adapter: target.adapter.clone(),
            kind: target.kind.as_str().to_owned(),
            language: String::new(),
            paths: target
                .path()
                .map(|path| vec![path.to_owned()])
                .unwrap_or_default(),
            current_value_summary: String::from("error"),
            expected_value_summary: String::from("unknown"),
            status: PlanStatus::Error,
            operations: Vec::new(),
            diagnostics: vec![err.to_string()],
            required: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanOperation {
    pub kind: String,
    pub path: String,
    pub summary: String,
    #[serde(skip_serializing)]
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanStatus {
    Ok,
    Drift,
    Missing,
    Unsupported,
    Error,
    Skipped,
}

impl PlanStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Drift => "drift",
            Self::Missing => "missing",
            Self::Unsupported => "unsupported",
            Self::Error => "error",
            Self::Skipped => "skipped",
        }
    }

    pub fn is_failure(self) -> bool {
        matches!(
            self,
            Self::Drift | Self::Missing | Self::Unsupported | Self::Error
        )
    }

    pub fn proto(self) -> OmvPlanStatus {
        match self {
            Self::Ok => OmvPlanStatus::Ok,
            Self::Drift => OmvPlanStatus::Drift,
            Self::Missing => OmvPlanStatus::Missing,
            Self::Unsupported => OmvPlanStatus::Unsupported,
            Self::Error => OmvPlanStatus::Error,
            Self::Skipped => OmvPlanStatus::Skipped,
        }
    }
}

pub trait TargetSyncAdapter {
    fn language_key(&self) -> &'static str;
    fn plan(&self, context: &SyncContext<'_>) -> Result<PlanTargetResult, OmvError>;

    fn sync(&self, context: &SyncContext<'_>) -> Result<(), OmvError> {
        let result = self.plan(context)?;
        apply_target_result(context.project_root, &result)
    }
}

pub trait V2TargetSyncAdapter {
    fn kind(&self) -> TargetKind;
    fn plan(&self, context: &V2SyncContext<'_>) -> Result<PlanTargetResult, OmvError>;
}

pub fn plan_all_targets(project_root: &Path, targets: &OmvTargets, version: &str) -> PlanSummary {
    let registry = stage1_registry();
    plan_all_targets_with_registry(project_root, targets, version, &registry)
}

pub fn plan_all_targets_with_registry(
    project_root: &Path,
    targets: &OmvTargets,
    version: &str,
    registry: &CapabilityRegistry,
) -> PlanSummary {
    let mut target_results = Vec::new();

    for target in &targets.targets {
        if !target.enabled {
            target_results.push(PlanTargetResult::skipped(target));
            continue;
        }

        if !registry.supports_language(target.language) {
            target_results.push(PlanTargetResult::unsupported(
                target,
                format!(
                    "contract v{} does not support target language {}",
                    registry.contract_version,
                    target.language.as_str()
                ),
            ));
            continue;
        }

        let context = SyncContext {
            project_root,
            target,
            version,
        };

        match adapter_for_language(target.language).and_then(|adapter| adapter.plan(&context)) {
            Ok(result) => target_results.push(result),
            Err(err) => target_results.push(PlanTargetResult::errored(target, err)),
        }
    }

    for target in &targets.v2_targets {
        if !target.enabled {
            target_results.push(PlanTargetResult::skipped_v2(target));
            continue;
        }

        if !registry.supports_kind(target.kind) {
            target_results.push(PlanTargetResult::unsupported_v2(
                target,
                format!(
                    "current OMV contract v{} does not support target kind {}; update OMV to use this capability",
                    registry.contract_version,
                    target.kind.as_str()
                ),
            ));
            continue;
        }

        let context = V2SyncContext {
            project_root,
            target,
            version,
        };

        match adapter_for_kind(target.kind).and_then(|adapter| adapter.plan(&context)) {
            Ok(mut result) => {
                if target.mode == TargetMode::Check {
                    result.operations.clear();
                    result.diagnostics.push(String::from(
                        "target mode is check; no write operation planned",
                    ));
                }
                target_results.push(result);
            }
            Err(err) => target_results.push(PlanTargetResult::errored_v2(target, err)),
        }
    }

    for target in &targets.unsupported_targets {
        if target.enabled {
            target_results.push(PlanTargetResult::unsupported_kind(target));
        } else {
            target_results.push(PlanTargetResult::skipped_unsupported_kind(target));
        }
    }

    let mut migration_status = Vec::new();
    if targets.schema_version >= 1 {
        migration_status.push(String::from("current-project"));
    } else {
        migration_status.push(String::from("compatible-old-project"));
    }

    if target_results
        .iter()
        .any(|target| target.status == PlanStatus::Unsupported)
    {
        migration_status.push(String::from("missing-capability"));
    }

    let project_status = if target_results
        .iter()
        .any(|target| target.status == PlanStatus::Unsupported)
    {
        String::from("missing-capability")
    } else if targets.schema_version < 1 {
        String::from("compatible-old-project")
    } else {
        String::from("current-project")
    };

    let totals = PlanTotals::from_targets(&target_results);
    PlanSummary {
        contract_version: registry.contract_version,
        version: version.to_owned(),
        project_root: project_root.display().to_string(),
        project_status,
        migration_status,
        totals,
        targets: target_results,
    }
}

pub fn sync_all_targets(
    project_root: &Path,
    targets: &OmvTargets,
    version: &str,
) -> Result<SyncSummary, OmvError> {
    let plan = plan_all_targets(project_root, targets, version);
    if plan
        .targets
        .iter()
        .any(|target| matches!(target.status, PlanStatus::Unsupported | PlanStatus::Error))
    {
        let details = plan
            .targets
            .iter()
            .filter(|target| matches!(target.status, PlanStatus::Unsupported | PlanStatus::Error))
            .map(|target| format!("{}: {}", target.id, target.diagnostics.join("; ")))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(TargetError::InvalidTargetRecord(format!(
            "target plan contains unsupported or errored targets: {details}"
        ))
        .into());
    }

    apply_plan(project_root, &plan)
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

fn adapter_for_kind(kind: TargetKind) -> Result<Box<dyn V2TargetSyncAdapter>, OmvError> {
    let adapter: Box<dyn V2TargetSyncAdapter> = match kind {
        TargetKind::TextScalar => Box::new(generic::TextScalarAdapter),
        TargetKind::RegexReplace => Box::new(generic::RegexReplaceAdapter),
        TargetKind::MarkdownManagedBlock => Box::new(generic::MarkdownManagedBlockAdapter),
        TargetKind::YamlScalar => Box::new(generic::YamlScalarAdapter),
        TargetKind::CHeaderMacro => Box::new(generic::CHeaderMacroAdapter),
        TargetKind::CargoWorkspace => Box::new(cargo_workspace::CargoWorkspaceAdapter),
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

pub(crate) fn planned_write(
    project_root: &Path,
    path: &Path,
    content: String,
    summary: impl Into<String>,
) -> PlanOperation {
    PlanOperation {
        kind: String::from("write"),
        path: project_relative_path(project_root, path),
        summary: summary.into(),
        content,
    }
}

pub(crate) fn plan_manifest_runtime(
    context: &SyncContext<'_>,
    adapter: &str,
    kind: &str,
    manifest_content: String,
    runtime_content: String,
) -> Result<PlanTargetResult, OmvError> {
    let manifest_path = resolve_target_path(
        context.project_root,
        &context.target.root,
        &context.target.manifest_path,
    );
    let runtime_path = resolve_target_path(
        context.project_root,
        &context.target.root,
        &context.target.runtime_export_path,
    );

    let current_manifest = read_text_if_exists(&manifest_path)?;
    let current_runtime = read_text_if_exists(&runtime_path)?;
    let manifest_missing = current_manifest.is_none();
    let runtime_missing = current_runtime.is_none();

    let status = if manifest_missing || runtime_missing {
        PlanStatus::Missing
    } else if current_manifest.as_deref() == Some(manifest_content.as_str())
        && current_runtime.as_deref() == Some(runtime_content.as_str())
    {
        PlanStatus::Ok
    } else {
        PlanStatus::Drift
    };

    let mut diagnostics = Vec::new();
    if manifest_missing {
        diagnostics.push(format!("missing {}", context.target.manifest_path));
    }
    if runtime_missing {
        diagnostics.push(format!("missing {}", context.target.runtime_export_path));
    }
    if status == PlanStatus::Drift {
        diagnostics.push(String::from(
            "target content differs from .omv version truth",
        ));
    }

    Ok(PlanTargetResult {
        id: context.target.id.clone(),
        adapter: adapter.to_owned(),
        kind: kind.to_owned(),
        language: context.target.language.as_str().to_owned(),
        paths: vec![
            project_relative_path(context.project_root, &manifest_path),
            project_relative_path(context.project_root, &runtime_path),
        ],
        current_value_summary: current_summary(
            current_manifest.as_deref(),
            current_runtime.as_deref(),
        ),
        expected_value_summary: format!("version {}", context.version),
        status,
        operations: vec![
            planned_write(
                context.project_root,
                &manifest_path,
                manifest_content,
                "write native manifest from .omv version truth",
            ),
            planned_write(
                context.project_root,
                &runtime_path,
                runtime_content,
                "write runtime export from .omv version truth",
            ),
        ],
        diagnostics,
        required: true,
    })
}

fn apply_plan(project_root: &Path, plan: &PlanSummary) -> Result<SyncSummary, OmvError> {
    let mut synced = 0;
    let mut skipped = 0;

    for target in &plan.targets {
        match target.status {
            PlanStatus::Skipped => {
                skipped += 1;
            }
            PlanStatus::Ok | PlanStatus::Drift | PlanStatus::Missing => {
                if target.status.is_failure() && target.operations.is_empty() {
                    return Err(TargetError::InvalidTargetRecord(format!(
                        "target {} has status {} and no safe write operation",
                        target.id,
                        target.status.as_str()
                    ))
                    .into());
                }
                apply_target_result(project_root, target)?;
                synced += 1;
            }
            PlanStatus::Unsupported | PlanStatus::Error => {
                return Err(TargetError::InvalidTargetRecord(format!(
                    "cannot apply target {} with status {}",
                    target.id,
                    target.status.as_str()
                ))
                .into());
            }
        }
    }

    Ok(SyncSummary { synced, skipped })
}

fn apply_target_result(project_root: &Path, result: &PlanTargetResult) -> Result<(), OmvError> {
    for operation in &result.operations {
        if operation.kind == "write" {
            write_text(&project_root.join(&operation.path), &operation.content)?;
        }
    }
    Ok(())
}

fn current_summary(manifest: Option<&str>, runtime: Option<&str>) -> String {
    format!(
        "manifest {}; runtime {}",
        if manifest.is_some() {
            "present"
        } else {
            "missing"
        },
        if runtime.is_some() {
            "present"
        } else {
            "missing"
        }
    )
}

pub(crate) fn project_relative_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .display()
        .to_string()
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
    use crate::core::schema::{OmvTargetRecord, OmvTargets, OmvUnsupportedTargetRecord};
    use crate::core::target::TargetLanguage;

    use super::{PlanStatus, plan_all_targets, replace_or_append_line, sync_all_targets};

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
            v2_targets: Vec::new(),
            unsupported_targets: Vec::new(),
        };

        let summary = sync_all_targets(std::path::Path::new("."), &targets, "2604.13.1")
            .expect("sync should succeed");
        assert_eq!(summary.synced, 0);
        assert_eq!(summary.skipped, 1);
    }

    #[test]
    fn plan_all_targets_reports_missing_then_ok_after_sync() {
        let targets = OmvTargets {
            schema_version: 1,
            targets: vec![OmvTargetRecord {
                id: "workspace-rust".to_owned(),
                language: TargetLanguage::Rust,
                root: ".".to_owned(),
                manifest_path: "Cargo.toml".to_owned(),
                runtime_export_path: "src/generated/version.rs".to_owned(),
                strategy: crate::core::target::PreProjectStrategy::IntentOnly,
                enabled: true,
            }],
            v2_targets: Vec::new(),
            unsupported_targets: Vec::new(),
        };
        let root = std::env::temp_dir().join(format!(
            "omv-plan-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should work")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root should exist");

        let missing = plan_all_targets(&root, &targets, "2604.13.1");
        assert_eq!(missing.targets[0].status, PlanStatus::Missing);

        sync_all_targets(&root, &targets, "2604.13.1").expect("sync should apply plan");
        let ok = plan_all_targets(&root, &targets, "2604.13.1");
        assert_eq!(ok.targets[0].status, PlanStatus::Ok);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plan_all_targets_reports_unknown_kind_as_unsupported() {
        let targets = OmvTargets {
            schema_version: 99,
            targets: Vec::new(),
            v2_targets: Vec::new(),
            unsupported_targets: vec![OmvUnsupportedTargetRecord {
                id: "future-target".to_owned(),
                kind: "future-workspace".to_owned(),
                adapter: "unknown".to_owned(),
                root: ".".to_owned(),
                enabled: true,
                paths: vec!["future.toml".to_owned()],
            }],
        };

        let plan = plan_all_targets(std::path::Path::new("."), &targets, "2605.1.3");

        assert_eq!(plan.project_status, "missing-capability");
        assert_eq!(plan.targets[0].status, PlanStatus::Unsupported);
        assert!(plan.targets[0].operations.is_empty());
        assert!(
            plan.targets[0].diagnostics[0].contains("update OMV"),
            "diagnostic should point users toward a newer binary"
        );
    }
}
