pub mod engine;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildPolicy {
    #[default]
    DailyReset,
    Continuous,
}

impl BuildPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DailyReset => "daily-reset",
            Self::Continuous => "continuous",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "daily-reset" => Some(Self::DailyReset),
            "continuous" => Some(Self::Continuous),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VersionOutput {
    #[default]
    DateTriplet,
    Semver,
}

impl VersionOutput {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DateTriplet => "date-triplet",
            Self::Semver => "semver",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "date-triplet" => Some(Self::DateTriplet),
            "semver" => Some(Self::Semver),
            _ => None,
        }
    }
}
