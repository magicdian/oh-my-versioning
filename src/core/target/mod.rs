#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectProfile {
    #[default]
    Personal,
    Oss,
}

impl ProjectProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Oss => "oss",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "personal" => Some(Self::Personal),
            "oss" => Some(Self::Oss),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLanguage {
    CFamily,
    Java,
    Rust,
    Python,
    Go,
}

impl TargetLanguage {
    pub fn all() -> &'static [Self] {
        const ALL: [TargetLanguage; 5] = [
            TargetLanguage::CFamily,
            TargetLanguage::Java,
            TargetLanguage::Rust,
            TargetLanguage::Python,
            TargetLanguage::Go,
        ];
        &ALL
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::CFamily => "c-family",
            Self::Java => "java",
            Self::Rust => "rust",
            Self::Python => "python",
            Self::Go => "go",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "c-family" => Some(Self::CFamily),
            "java" => Some(Self::Java),
            "rust" => Some(Self::Rust),
            "python" => Some(Self::Python),
            "go" => Some(Self::Go),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    TextScalar,
    RegexReplace,
    MarkdownManagedBlock,
    YamlScalar,
    CHeaderMacro,
    CargoWorkspace,
}

impl TargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TextScalar => "text-scalar",
            Self::RegexReplace => "regex-replace",
            Self::MarkdownManagedBlock => "markdown-managed-block",
            Self::YamlScalar => "yaml-scalar",
            Self::CHeaderMacro => "c-header-macro",
            Self::CargoWorkspace => "cargo-workspace",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "text-scalar" => Some(Self::TextScalar),
            "regex-replace" => Some(Self::RegexReplace),
            "markdown-managed-block" => Some(Self::MarkdownManagedBlock),
            "yaml-scalar" => Some(Self::YamlScalar),
            "c-header-macro" => Some(Self::CHeaderMacro),
            "cargo-workspace" => Some(Self::CargoWorkspace),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TargetMode {
    #[default]
    Write,
    Check,
}

impl TargetMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::Check => "check",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "write" => Some(Self::Write),
            "check" => Some(Self::Check),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CargoMembers {
    #[default]
    All,
}

impl CargoMembers {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CargoVersionPolicy {
    #[default]
    Same,
}

impl CargoVersionPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Same => "same",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "same" => Some(Self::Same),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CargoVersionLocation {
    #[default]
    Auto,
    WorkspacePackage,
    MemberPackages,
}

impl CargoVersionLocation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::WorkspacePackage => "workspace-package",
            Self::MemberPackages => "member-packages",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "auto" => Some(Self::Auto),
            "workspace-package" => Some(Self::WorkspacePackage),
            "member-packages" => Some(Self::MemberPackages),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CargoLockfileStrategy {
    #[default]
    Check,
    Update,
    Ignore,
}

impl CargoLockfileStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Update => "update",
            Self::Ignore => "ignore",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "check" => Some(Self::Check),
            "update" => Some(Self::Update),
            "ignore" => Some(Self::Ignore),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreProjectStrategy {
    #[default]
    IntentOnly,
    InitExportTemplates,
    CreateMinimalScaffold,
}

impl PreProjectStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IntentOnly => "intent-only",
            Self::InitExportTemplates => "init-export-templates",
            Self::CreateMinimalScaffold => "create-minimal-scaffold",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "intent-only" => Some(Self::IntentOnly),
            "init-export-templates" => Some(Self::InitExportTemplates),
            "create-minimal-scaffold" => Some(Self::CreateMinimalScaffold),
            _ => None,
        }
    }
}
