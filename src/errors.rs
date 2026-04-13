use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug)]
pub enum OmvError {
    Cli(CliError),
    Config(ConfigError),
    State(StateError),
    Time(TimeError),
    Ntp(NtpError),
    Target(TargetError),
    I18n(I18nError),
    Storage(StorageError),
    Io(std::io::Error),
}

impl Display for OmvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cli(err) => write!(f, "{err}"),
            Self::Config(err) => write!(f, "{err}"),
            Self::State(err) => write!(f, "{err}"),
            Self::Time(err) => write!(f, "{err}"),
            Self::Ntp(err) => write!(f, "{err}"),
            Self::Target(err) => write!(f, "{err}"),
            Self::I18n(err) => write!(f, "{err}"),
            Self::Storage(err) => write!(f, "{err}"),
            Self::Io(err) => write!(f, "io error: {err}"),
        }
    }
}

impl std::error::Error for OmvError {}

impl From<std::io::Error> for OmvError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug)]
pub enum CliError {
    UnknownCommand(String),
    MissingLocaleValue,
    UserCancelled,
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCommand(cmd) => write!(f, "unknown command: {cmd}"),
            Self::MissingLocaleValue => write!(f, "missing value after --locale"),
            Self::UserCancelled => write!(f, "operation cancelled by user"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<CliError> for OmvError {
    fn from(value: CliError) -> Self {
        Self::Cli(value)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    RootResolution { cwd: String },
    InvalidLocale(String),
    InvalidBuildPolicy(String),
    Parse { path: PathBuf, reason: String },
    Missing { path: PathBuf },
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RootResolution { cwd } => {
                write!(f, "failed to resolve .omv root from cwd: {cwd}")
            }
            Self::InvalidLocale(locale) => write!(f, "invalid locale: {locale}"),
            Self::InvalidBuildPolicy(policy) => write!(f, "invalid build policy: {policy}"),
            Self::Parse { path, reason } => {
                write!(f, "failed to parse config {}: {reason}", path.display())
            }
            Self::Missing { path } => write!(f, "missing config file: {}", path.display()),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<ConfigError> for OmvError {
    fn from(value: ConfigError) -> Self {
        Self::Config(value)
    }
}

#[derive(Debug)]
pub enum StateError {
    Parse { path: PathBuf, reason: String },
    MissingState { path: PathBuf },
}

impl Display for StateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { path, reason } => {
                write!(f, "failed to parse state {}: {reason}", path.display())
            }
            Self::MissingState { path } => write!(f, "missing state file: {}", path.display()),
        }
    }
}

impl std::error::Error for StateError {}

impl From<StateError> for OmvError {
    fn from(value: StateError) -> Self {
        Self::State(value)
    }
}

#[derive(Debug)]
pub enum TimeError {
    InvalidDateFormat(String),
    FutureStoredDate { stored: String, validated: String },
}

impl Display for TimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDateFormat(value) => write!(f, "invalid date format: {value}"),
            Self::FutureStoredDate { stored, validated } => {
                write!(
                    f,
                    "stored logical date is in the future: stored={stored}, validated={validated}"
                )
            }
        }
    }
}

impl std::error::Error for TimeError {}

impl From<TimeError> for OmvError {
    fn from(value: TimeError) -> Self {
        Self::Time(value)
    }
}

#[derive(Debug)]
pub enum NtpError {
    Unavailable(String),
}

impl Display for NtpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(reason) => write!(f, "ntp unavailable: {reason}"),
        }
    }
}

impl std::error::Error for NtpError {}

impl From<NtpError> for OmvError {
    fn from(value: NtpError) -> Self {
        Self::Ntp(value)
    }
}

#[derive(Debug)]
pub enum TargetError {
    InvalidTargetRecord(String),
    Parse { path: PathBuf, reason: String },
    Missing { path: PathBuf },
}

impl Display for TargetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTargetRecord(reason) => write!(f, "invalid target record: {reason}"),
            Self::Parse { path, reason } => {
                write!(f, "failed to parse targets {}: {reason}", path.display())
            }
            Self::Missing { path } => write!(f, "missing targets file: {}", path.display()),
        }
    }
}

impl std::error::Error for TargetError {}

impl From<TargetError> for OmvError {
    fn from(value: TargetError) -> Self {
        Self::Target(value)
    }
}

#[derive(Debug)]
pub enum I18nError {
    ParseCatalog {
        locale: String,
        reason: String,
    },
    MissingKey(String),
    CatalogParity {
        missing_in_en: Vec<String>,
        missing_in_zh: Vec<String>,
    },
}

impl Display for I18nError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseCatalog { locale, reason } => {
                write!(f, "failed to parse {locale} catalog: {reason}")
            }
            Self::MissingKey(key) => write!(f, "missing i18n key: {key}"),
            Self::CatalogParity {
                missing_in_en,
                missing_in_zh,
            } => {
                write!(
                    f,
                    "catalog parity mismatch (missing_in_en={missing_in_en:?}, missing_in_zh={missing_in_zh:?})"
                )
            }
        }
    }
}

impl std::error::Error for I18nError {}

impl From<I18nError> for OmvError {
    fn from(value: I18nError) -> Self {
        Self::I18n(value)
    }
}

#[derive(Debug)]
pub enum StorageError {
    AtomicWriteFailed { path: PathBuf, reason: String },
}

impl Display for StorageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AtomicWriteFailed { path, reason } => {
                write!(f, "atomic write failed for {}: {reason}", path.display())
            }
        }
    }
}

impl std::error::Error for StorageError {}

impl From<StorageError> for OmvError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}
