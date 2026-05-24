use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrationProvider {
    Codex,
    Trellis,
    OpenCode,
}

impl IntegrationProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Trellis => "trellis",
            Self::OpenCode => "opencode",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "codex" => Some(Self::Codex),
            "trellis" => Some(Self::Trellis),
            "opencode" => Some(Self::OpenCode),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrationProviderKind {
    AgentHost,
    SpecWorkflowHost,
}

impl IntegrationProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AgentHost => "agent-host",
            Self::SpecWorkflowHost => "spec-workflow-host",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrationBootstrapPolicy {
    BootstrapLightweightHost,
    RequireExistingHost,
}

impl IntegrationBootstrapPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BootstrapLightweightHost => "bootstrap-lightweight-host",
            Self::RequireExistingHost => "require-existing-host",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrationCapability {
    ProjectInstructions,
    HostSkill,
    SpecGuide,
    SpecIndexSnippet,
    FinalizeBoundary,
}

impl IntegrationCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProjectInstructions => "project-instructions",
            Self::HostSkill => "host-skill",
            Self::SpecGuide => "spec-guide",
            Self::SpecIndexSnippet => "spec-index-snippet",
            Self::FinalizeBoundary => "finalize-boundary",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "project-instructions" => Some(Self::ProjectInstructions),
            "host-skill" => Some(Self::HostSkill),
            "spec-guide" => Some(Self::SpecGuide),
            "spec-index-snippet" => Some(Self::SpecIndexSnippet),
            "finalize-boundary" => Some(Self::FinalizeBoundary),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrationCapabilityStatus {
    Selected,
    Pending,
    Installed,
    Failed,
}

impl IntegrationCapabilityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::Pending => "pending",
            Self::Installed => "installed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationFailure {
    pub reason_code: String,
    pub display_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationDetectionSnapshot {
    pub detected: bool,
    pub recommended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmvIntegrationCapabilityState {
    pub capability: IntegrationCapability,
    pub selected: bool,
    pub status: IntegrationCapabilityStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<IntegrationFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmvIntegrationProviderState {
    pub provider: IntegrationProvider,
    pub selected: bool,
    pub detection: IntegrationDetectionSnapshot,
    pub capabilities: Vec<OmvIntegrationCapabilityState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmvIntegrations {
    pub schema_version: u32,
    pub providers: Vec<OmvIntegrationProviderState>,
}

impl Default for OmvIntegrations {
    fn default() -> Self {
        Self {
            schema_version: 1,
            providers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrationCapabilityDescriptor {
    pub capability: IntegrationCapability,
    pub default_selected: bool,
    pub recommended: bool,
    pub target_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrationProviderDescriptor {
    pub provider: IntegrationProvider,
    pub kind: IntegrationProviderKind,
    pub bootstrap_policy: IntegrationBootstrapPolicy,
    pub capabilities: Vec<IntegrationCapabilityDescriptor>,
}

pub fn mvp_provider_descriptors() -> Vec<IntegrationProviderDescriptor> {
    vec![
        IntegrationProviderDescriptor {
            provider: IntegrationProvider::Codex,
            kind: IntegrationProviderKind::AgentHost,
            bootstrap_policy: IntegrationBootstrapPolicy::BootstrapLightweightHost,
            capabilities: vec![
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::ProjectInstructions,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![String::from("AGENTS.md")],
                },
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::HostSkill,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![String::from(".codex/skills/omv-versioning/SKILL.md")],
                },
            ],
        },
        IntegrationProviderDescriptor {
            provider: IntegrationProvider::OpenCode,
            kind: IntegrationProviderKind::AgentHost,
            bootstrap_policy: IntegrationBootstrapPolicy::BootstrapLightweightHost,
            capabilities: vec![
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::ProjectInstructions,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![String::from("AGENTS.md")],
                },
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::HostSkill,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![String::from(".opencode/skills/omv-versioning/SKILL.md")],
                },
            ],
        },
        IntegrationProviderDescriptor {
            provider: IntegrationProvider::Trellis,
            kind: IntegrationProviderKind::SpecWorkflowHost,
            bootstrap_policy: IntegrationBootstrapPolicy::RequireExistingHost,
            capabilities: vec![
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::SpecGuide,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![String::from(
                        ".trellis/spec/guides/omv-versioning-guide.md",
                    )],
                },
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::SpecIndexSnippet,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![String::from(".trellis/spec/guides/index.md")],
                },
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::FinalizeBoundary,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![
                        String::from(".agents/skills/trellis-finish-work/SKILL.md"),
                        String::from(".agents/skills/finish-work/SKILL.md"),
                    ],
                },
            ],
        },
    ]
}

/// Trellis version information read from `.trellis/.version`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrellisVersionInfo {
    /// Raw version string (e.g. "0.5.19").
    pub version: String,
    /// `true` when the Trellis major.minor version is >= 0.5.
    pub is_v05_or_later: bool,
}

/// Read the Trellis version from `.trellis/.version` under `project_root`.
///
/// Returns `None` when the file does not exist or cannot be parsed as semver.
pub fn detect_trellis_version(project_root: &Path) -> Option<TrellisVersionInfo> {
    let version_path = project_root.join(".trellis").join(".version");
    let raw = std::fs::read_to_string(&version_path).ok()?;
    let version = raw.trim().to_owned();
    if version.is_empty() {
        return None;
    }
    let is_v05_or_later = is_trellis_v05_or_later(&version);
    Some(TrellisVersionInfo {
        version,
        is_v05_or_later,
    })
}

/// Classify a Trellis version string as v0.5+ or pre-v0.5.
///
/// Returns `true` when `major == 0 && minor >= 5` or `major > 0`.
/// Any unparseable version string returns `false`.
pub fn is_trellis_v05_or_later(version: &str) -> bool {
    let version = version
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V');
    let mut parts = version.splitn(3, '.');
    let major: u32 = match parts.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return false,
    };
    let minor: u32 = match parts.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return false,
    };
    if major > 0 {
        return true;
    }
    minor >= 5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_trellis_version_v0519() {
        // When running outside a Trellis project, just test parsing.
        let _ = detect_trellis_version(Path::new("."));
        // We can't guarantee .version exists in every test environment,
        // so test the classification function directly.
        assert!(is_trellis_v05_or_later("0.5.19"));
        assert!(is_trellis_v05_or_later("v0.5.0"));
        assert!(is_trellis_v05_or_later("0.5.0"));
        assert!(is_trellis_v05_or_later("0.6.1"));
        assert!(is_trellis_v05_or_later("1.0.0"));
    }

    #[test]
    fn classify_pre_v05() {
        assert!(!is_trellis_v05_or_later("0.4.0"));
        assert!(!is_trellis_v05_or_later("v0.4.9"));
        assert!(!is_trellis_v05_or_later("0.3.12"));
        assert!(!is_trellis_v05_or_later("0.4.15"));
    }

    #[test]
    fn classify_unparseable_version() {
        assert!(!is_trellis_v05_or_later(""));
        assert!(!is_trellis_v05_or_later("not-a-version"));
        assert!(!is_trellis_v05_or_later("v"));
    }

    #[test]
    fn detect_trellis_version_reads_current_project() {
        // This test runs in the oh-my-versioning repo where .trellis/.version exists.
        // Use CARGO_MANIFEST_DIR to reliably locate the project root.
        let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let info = detect_trellis_version(project_root);
        assert!(
            info.is_some(),
            "expected .trellis/.version in current repo at {}",
            project_root.display()
        );
        let info = info.unwrap();
        assert!(!info.version.is_empty());
        // The current repo is at least v0.5.x.
        assert!(info.is_v05_or_later);
    }
}
