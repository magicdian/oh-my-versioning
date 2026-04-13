use super::locale::OperatorLocale;
use super::target::{PreProjectStrategy, ProjectProfile, TargetLanguage};
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
}

impl Default for OmvTargets {
    fn default() -> Self {
        Self {
            schema_version: 1,
            targets: Vec::new(),
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
