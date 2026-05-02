use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterKind {
    Agent,
    Spec,
}

impl AdapterKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Spec => "spec",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentAdapter {
    Claude,
    Codex,
}

impl AgentAdapter {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        const ALL: [AgentAdapter; 2] = [AgentAdapter::Claude, AgentAdapter::Codex];
        &ALL
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpecAdapter {
    OpenSpec,
    Trellis,
}

impl SpecAdapter {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenSpec => "openspec",
            Self::Trellis => "trellis",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "openspec" => Some(Self::OpenSpec),
            "trellis" => Some(Self::Trellis),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        const ALL: [SpecAdapter; 2] = [SpecAdapter::OpenSpec, SpecAdapter::Trellis];
        &ALL
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterInstallMode {
    Link,
    Materialize,
    Hybrid,
}

impl AdapterInstallMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::Materialize => "materialize",
            Self::Hybrid => "hybrid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterTargetMode {
    Link,
    Materialize,
    ManagedBlock,
}

impl AdapterTargetMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::Materialize => "materialize",
            Self::ManagedBlock => "managed-block",
        }
    }
}
