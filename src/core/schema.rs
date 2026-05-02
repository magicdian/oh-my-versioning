use serde::{Deserialize, Serialize};

use super::adapter::{AdapterInstallMode, AdapterKind, AdapterTargetMode};
use super::finalization::{
    ChangeType, FinalizationOutcome, FinalizationReason, TaskStatus, TestsStatus,
};
use super::locale::OperatorLocale;
use super::target::{
    CargoLockfileStrategy, CargoMembers, CargoVersionLocation, CargoVersionPolicy,
    PreProjectStrategy, ProjectProfile, TargetKind, TargetLanguage, TargetMode,
};
use super::time::LastTimeSource;
use super::versioning::{BuildPolicy, VersionOutput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmvConfig {
    pub schema_version: u32,
    pub locale: OperatorLocale,
    pub timezone: String,
    pub project_profile: ProjectProfile,
    pub version_output: VersionOutput,
    pub build_policy: BuildPolicy,
    pub ntp_enabled: bool,
}

impl Default for OmvConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            locale: OperatorLocale::EnUs,
            timezone: String::from("UTC+0"),
            project_profile: ProjectProfile::Personal,
            version_output: VersionOutput::DateTriplet,
            build_policy: BuildPolicy::DailyReset,
            ntp_enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmvState {
    pub schema_version: u32,
    pub logical_date: String,
    pub build_number: u32,
    pub last_issued_version: String,
    pub last_time_source: LastTimeSource,
}

impl OmvState {
    pub fn new(logical_date: impl Into<String>, build_number: u32) -> Self {
        let date = logical_date.into();
        Self {
            schema_version: 1,
            logical_date: date,
            build_number,
            last_issued_version: String::new(),
            last_time_source: LastTimeSource::Ntp,
        }
    }
}

impl Default for OmvState {
    fn default() -> Self {
        Self::new("1970-01-01", 1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmvTargets {
    pub schema_version: u32,
    pub targets: Vec<OmvTargetRecord>,
    pub v2_targets: Vec<OmvV2TargetRecord>,
    pub unsupported_targets: Vec<OmvUnsupportedTargetRecord>,
}

impl Default for OmvTargets {
    fn default() -> Self {
        Self {
            schema_version: 1,
            targets: Vec::new(),
            v2_targets: Vec::new(),
            unsupported_targets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmvTargetRecord {
    pub id: String,
    pub language: TargetLanguage,
    pub root: String,
    pub manifest_path: String,
    pub runtime_export_path: String,
    pub strategy: PreProjectStrategy,
    pub enabled: bool,
}

impl OmvTargetRecord {
    pub fn new(id: impl Into<String>, language: TargetLanguage) -> Self {
        Self {
            id: id.into(),
            language,
            root: String::from("."),
            manifest_path: String::new(),
            runtime_export_path: String::new(),
            strategy: PreProjectStrategy::IntentOnly,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmvUnsupportedTargetRecord {
    pub id: String,
    pub kind: String,
    pub adapter: String,
    pub root: String,
    pub enabled: bool,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmvV2TargetRecord {
    pub id: String,
    pub kind: TargetKind,
    pub adapter: String,
    pub root: String,
    pub enabled: bool,
    pub mode: TargetMode,
    pub config: OmvV2TargetConfig,
}

impl OmvV2TargetRecord {
    pub fn path(&self) -> Option<&str> {
        match &self.config {
            OmvV2TargetConfig::TextScalar(config) => Some(config.path.as_str()),
            OmvV2TargetConfig::RegexReplace(config) => Some(config.path.as_str()),
            OmvV2TargetConfig::MarkdownManagedBlock(config) => Some(config.path.as_str()),
            OmvV2TargetConfig::YamlScalar(config) => Some(config.path.as_str()),
            OmvV2TargetConfig::CHeaderMacro(config) => Some(config.path.as_str()),
            OmvV2TargetConfig::CargoWorkspace(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmvV2TargetConfig {
    TextScalar(TextScalarTarget),
    RegexReplace(RegexReplaceTarget),
    MarkdownManagedBlock(MarkdownManagedBlockTarget),
    YamlScalar(YamlScalarTarget),
    CHeaderMacro(CHeaderMacroTarget),
    CargoWorkspace(CargoWorkspaceTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextScalarTarget {
    pub path: String,
    pub selector: String,
    pub template: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexReplaceTarget {
    pub path: String,
    pub pattern: String,
    pub template: String,
    pub allow_multiple: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownManagedBlockTarget {
    pub path: String,
    pub begin_marker: String,
    pub end_marker: String,
    pub template: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlScalarTarget {
    pub path: String,
    pub key: String,
    pub template: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CHeaderMacroTarget {
    pub path: String,
    pub macro_name: String,
    pub template: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoWorkspaceTarget {
    pub root: String,
    pub members: CargoMembers,
    pub version_policy: CargoVersionPolicy,
    pub version_location: CargoVersionLocation,
    pub lockfile: CargoLockfileStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmvAdapters {
    pub schema_version: u32,
    pub installations: Vec<OmvAdapterInstallation>,
}

impl Default for OmvAdapters {
    fn default() -> Self {
        Self {
            schema_version: 1,
            installations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmvAdapterInstallation {
    pub kind: AdapterKind,
    pub name: String,
    pub install_mode: AdapterInstallMode,
    pub source_contract_version: u32,
    pub targets: Vec<OmvAdapterTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmvAdapterTarget {
    pub path: String,
    pub source_path: String,
    pub mode: AdapterTargetMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmvFinalizations {
    pub schema_version: u32,
    pub entries: Vec<OmvFinalizationRecord>,
}

impl Default for OmvFinalizations {
    fn default() -> Self {
        Self {
            schema_version: 1,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmvFinalizationRecord {
    pub task_id: String,
    pub fingerprint: String,
    pub change_type: ChangeType,
    pub task_status: TaskStatus,
    pub tests_status: TestsStatus,
    pub source: String,
    pub outcome: FinalizationOutcome,
    pub reason: FinalizationReason,
    pub version_before: String,
    pub version_after: String,
    pub recorded_at: String,
}
