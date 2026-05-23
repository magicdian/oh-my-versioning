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
