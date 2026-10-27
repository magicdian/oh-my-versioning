use std::path::Path;

use serde::{Deserialize, Serialize};

/// Host roots owned by spec/workflow frameworks that install competing
/// finish-work entrypoints. OMV must not project into both at once.
pub const SPEC_WORKFLOW_HOST_ROOTS: [(&str, IntegrationProvider); 2] = [
    (".trellis", IntegrationProvider::Trellis),
    (".phronia", IntegrationProvider::Phronia),
];

/// Return the spec/workflow host roots detected under `project_root` when more
/// than one is present.
///
/// OMV deliberately does not pick a winner, rewrite adapter records, or delete
/// framework files. The caller surfaces this as an actionable conflict so the
/// user decides which framework to keep.
pub fn detect_spec_framework_conflict(project_root: &Path) -> Option<Vec<&'static str>> {
    let detected: Vec<&'static str> = SPEC_WORKFLOW_HOST_ROOTS
        .iter()
        .filter(|(root, _)| project_root.join(root).exists())
        .map(|(root, _)| *root)
        .collect();
    if detected.len() > 1 {
        Some(detected)
    } else {
        None
    }
}

pub fn spec_framework_conflict_message(hosts: &[&str]) -> String {
    format!(
        "OMV detected multiple spec/workflow frameworks ({}). OMV will not choose between them, rewrite adapter records, or delete either framework. Remove or rename the framework host you do not want OMV to manage so exactly one remains, then rerun. If an old adapter record points at the removed framework, remove that record from `.omv/adapters.toml` first.",
        hosts.join(", ")
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrationProvider {
    Claude,
    Codex,
    Trellis,
    Phronia,
    OpenCode,
}

impl IntegrationProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Trellis => "trellis",
            Self::Phronia => "phronia",
            Self::OpenCode => "opencode",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            "trellis" => Some(Self::Trellis),
            "phronia" => Some(Self::Phronia),
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
            provider: IntegrationProvider::Claude,
            kind: IntegrationProviderKind::AgentHost,
            bootstrap_policy: IntegrationBootstrapPolicy::BootstrapLightweightHost,
            capabilities: vec![
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::ProjectInstructions,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![String::from("CLAUDE.md")],
                },
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::HostSkill,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![String::from(".claude/skills/omv-versioning/SKILL.md")],
                },
            ],
        },
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
        IntegrationProviderDescriptor {
            provider: IntegrationProvider::Phronia,
            kind: IntegrationProviderKind::SpecWorkflowHost,
            bootstrap_policy: IntegrationBootstrapPolicy::RequireExistingHost,
            capabilities: vec![
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::SpecGuide,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![String::from(
                        ".phronia/spec/guides/omv-versioning-guide.md",
                    )],
                },
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::SpecIndexSnippet,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![String::from(".phronia/spec/guides/index.md")],
                },
                IntegrationCapabilityDescriptor {
                    capability: IntegrationCapability::FinalizeBoundary,
                    default_selected: true,
                    recommended: true,
                    target_paths: vec![
                        String::from(".agents/skills/phronia-finish-work/SKILL.md"),
                        String::from(".claude/commands/phronia/finish-work.md"),
                        String::from(".opencode/commands/phronia/finish-work.md"),
                        String::from(".zcode/commands/phronia/finish-work.md"),
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
    fn detects_spec_framework_conflict_only_when_multiple_hosts_exist() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("omv-framework-conflict-{stamp}"));
        std::fs::create_dir_all(root.join(".trellis")).expect("create .trellis");

        assert_eq!(detect_spec_framework_conflict(&root), None);

        std::fs::create_dir_all(root.join(".phronia")).expect("create .phronia");
        assert_eq!(
            detect_spec_framework_conflict(&root),
            Some(vec![".trellis", ".phronia"])
        );

        std::fs::remove_dir_all(root.join(".trellis")).expect("remove .trellis");
        assert_eq!(detect_spec_framework_conflict(&root), None);

        let _ = std::fs::remove_dir_all(&root);
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
    fn detect_trellis_version_reads_version_file() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!("omv-trellis-version-{stamp}"));
        std::fs::create_dir_all(project_root.join(".trellis")).expect("create .trellis");
        std::fs::write(project_root.join(".trellis/.version"), "0.5.19\n").expect("write version");

        let info = detect_trellis_version(&project_root).expect("version should be detected");
        assert_eq!(info.version, "0.5.19");
        assert!(info.is_v05_or_later);

        let _ = std::fs::remove_dir_all(&project_root);
    }
}
