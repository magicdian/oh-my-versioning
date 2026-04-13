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
