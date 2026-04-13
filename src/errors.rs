use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug)]
pub enum OmvError {
    Cli(CliError),
    Adapter(AdapterError),
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
            Self::Adapter(err) => write!(f, "{err}"),
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
    MissingOutputValue,
    InvalidOutputMode(String),
    UnknownOption(String),
    MissingAdapterAction,
    UnknownAdapterAction(String),
    MissingAgentValue,
    MissingSpecValue,
    UnknownAgentAdapter(String),
    UnknownSpecAdapter(String),
    UserCancelled,
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCommand(cmd) => write!(f, "unknown command: {cmd}"),
            Self::MissingLocaleValue => write!(f, "missing value after --locale"),
            Self::MissingOutputValue => write!(f, "missing value after --output"),
            Self::InvalidOutputMode(mode) => write!(f, "invalid output mode: {mode}"),
            Self::UnknownOption(option) => write!(f, "unknown option: {option}"),
            Self::MissingAdapterAction => write!(f, "missing adapter action after `adapter`"),
            Self::UnknownAdapterAction(action) => write!(f, "unknown adapter action: {action}"),
            Self::MissingAgentValue => write!(f, "missing value after --agent"),
            Self::MissingSpecValue => write!(f, "missing value after --spec"),
            Self::UnknownAgentAdapter(name) => write!(f, "unknown agent adapter: {name}"),
            Self::UnknownSpecAdapter(name) => write!(f, "unknown spec adapter: {name}"),
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
pub enum AdapterError {
    Conflict { path: PathBuf, reason: String },
    Parse { path: PathBuf, reason: String },
    MissingRegistry { path: PathBuf },
    Unsupported { reason: String },
}

impl Display for AdapterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict { path, reason } => {
                write!(
                    f,
                    "adapter install conflict at {}: {reason}",
                    path.display()
                )
            }
            Self::Parse { path, reason } => {
                write!(
                    f,
                    "failed to parse adapter registry {}: {reason}",
                    path.display()
                )
            }
            Self::MissingRegistry { path } => {
                write!(f, "missing adapter registry: {}", path.display())
            }
            Self::Unsupported { reason } => write!(f, "unsupported adapter operation: {reason}"),
        }
    }
}

impl std::error::Error for AdapterError {}

impl From<AdapterError> for OmvError {
    fn from(value: AdapterError) -> Self {
        Self::Adapter(value)
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StructuredError {
    pub code: String,
    pub message: String,
    pub details: Value,
}

impl OmvError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Cli(CliError::UnknownCommand(_)) => "unknown_command",
            Self::Cli(CliError::MissingLocaleValue) => "missing_locale_value",
            Self::Cli(CliError::MissingOutputValue) => "missing_output_value",
            Self::Cli(CliError::InvalidOutputMode(_)) => "invalid_output_mode",
            Self::Cli(CliError::UnknownOption(_)) => "unknown_option",
            Self::Cli(CliError::MissingAdapterAction) => "missing_adapter_action",
            Self::Cli(CliError::UnknownAdapterAction(_)) => "unknown_adapter_action",
            Self::Cli(CliError::MissingAgentValue) => "missing_agent_value",
            Self::Cli(CliError::MissingSpecValue) => "missing_spec_value",
            Self::Cli(CliError::UnknownAgentAdapter(_)) => "unknown_agent_adapter",
            Self::Cli(CliError::UnknownSpecAdapter(_)) => "unknown_spec_adapter",
            Self::Cli(CliError::UserCancelled) => "user_cancelled",
            Self::Adapter(AdapterError::Conflict { .. }) => "adapter_conflict",
            Self::Adapter(AdapterError::Parse { .. }) => "adapter_registry_parse_failed",
            Self::Adapter(AdapterError::MissingRegistry { .. }) => "missing_adapter_registry",
            Self::Adapter(AdapterError::Unsupported { .. }) => "unsupported_adapter_operation",
            Self::Config(ConfigError::RootResolution { .. }) => "root_resolution_failed",
            Self::Config(ConfigError::InvalidLocale(_)) => "invalid_locale",
            Self::Config(ConfigError::InvalidBuildPolicy(_)) => "invalid_build_policy",
            Self::Config(ConfigError::Parse { .. }) => "config_parse_failed",
            Self::Config(ConfigError::Missing { .. }) => "missing_config",
            Self::State(StateError::Parse { .. }) => "state_parse_failed",
            Self::State(StateError::MissingState { .. }) => "missing_state",
            Self::Time(TimeError::InvalidDateFormat(_)) => "invalid_date_format",
            Self::Time(TimeError::FutureStoredDate { .. }) => "future_stored_date",
            Self::Ntp(NtpError::Unavailable(_)) => "ntp_unavailable",
            Self::Target(TargetError::InvalidTargetRecord(_)) => "invalid_target_record",
            Self::Target(TargetError::Parse { .. }) => "targets_parse_failed",
            Self::Target(TargetError::Missing { .. }) => "missing_targets",
            Self::I18n(I18nError::ParseCatalog { .. }) => "catalog_parse_failed",
            Self::I18n(I18nError::MissingKey(_)) => "missing_i18n_key",
            Self::I18n(I18nError::CatalogParity { .. }) => "catalog_parity_mismatch",
            Self::Storage(StorageError::AtomicWriteFailed { .. }) => "atomic_write_failed",
            Self::Io(_) => "io_error",
        }
    }

    pub fn structured_error(&self) -> StructuredError {
        StructuredError {
            code: self.code().to_owned(),
            message: self.to_string(),
            details: self.details(),
        }
    }

    fn details(&self) -> Value {
        let mut map = Map::new();
        match self {
            Self::Cli(CliError::UnknownCommand(cmd)) => {
                map.insert(String::from("command"), Value::String(cmd.clone()));
            }
            Self::Cli(CliError::InvalidOutputMode(mode)) => {
                map.insert(String::from("output_mode"), Value::String(mode.clone()));
            }
            Self::Cli(CliError::UnknownOption(option)) => {
                map.insert(String::from("option"), Value::String(option.clone()));
            }
            Self::Cli(CliError::UnknownAdapterAction(action)) => {
                map.insert(String::from("action"), Value::String(action.clone()));
            }
            Self::Cli(CliError::UnknownAgentAdapter(name)) => {
                map.insert(String::from("agent"), Value::String(name.clone()));
            }
            Self::Cli(CliError::UnknownSpecAdapter(name)) => {
                map.insert(String::from("spec"), Value::String(name.clone()));
            }
            Self::Adapter(AdapterError::Conflict { path, reason })
            | Self::Adapter(AdapterError::Parse { path, reason }) => {
                map.insert(
                    String::from("path"),
                    Value::String(path.display().to_string()),
                );
                map.insert(String::from("reason"), Value::String(reason.clone()));
            }
            Self::Adapter(AdapterError::MissingRegistry { path }) => {
                map.insert(
                    String::from("path"),
                    Value::String(path.display().to_string()),
                );
            }
            Self::Adapter(AdapterError::Unsupported { reason }) => {
                map.insert(String::from("reason"), Value::String(reason.clone()));
            }
            Self::Config(ConfigError::RootResolution { cwd }) => {
                map.insert(String::from("cwd"), Value::String(cwd.clone()));
            }
            Self::Config(ConfigError::InvalidLocale(locale)) => {
                map.insert(String::from("locale"), Value::String(locale.clone()));
            }
            Self::Config(ConfigError::InvalidBuildPolicy(policy)) => {
                map.insert(String::from("build_policy"), Value::String(policy.clone()));
            }
            Self::Config(ConfigError::Parse { path, reason })
            | Self::State(StateError::Parse { path, reason })
            | Self::Target(TargetError::Parse { path, reason }) => {
                map.insert(
                    String::from("path"),
                    Value::String(path.display().to_string()),
                );
                map.insert(String::from("reason"), Value::String(reason.clone()));
            }
            Self::Config(ConfigError::Missing { path })
            | Self::State(StateError::MissingState { path })
            | Self::Target(TargetError::Missing { path }) => {
                map.insert(
                    String::from("path"),
                    Value::String(path.display().to_string()),
                );
            }
            Self::Time(TimeError::InvalidDateFormat(value)) => {
                map.insert(String::from("value"), Value::String(value.clone()));
            }
            Self::Time(TimeError::FutureStoredDate { stored, validated }) => {
                map.insert(String::from("stored"), Value::String(stored.clone()));
                map.insert(String::from("validated"), Value::String(validated.clone()));
            }
            Self::Ntp(NtpError::Unavailable(reason)) => {
                map.insert(String::from("reason"), Value::String(reason.clone()));
            }
            Self::Target(TargetError::InvalidTargetRecord(reason)) => {
                map.insert(String::from("reason"), Value::String(reason.clone()));
            }
            Self::I18n(I18nError::ParseCatalog { locale, reason }) => {
                map.insert(String::from("locale"), Value::String(locale.clone()));
                map.insert(String::from("reason"), Value::String(reason.clone()));
            }
            Self::I18n(I18nError::MissingKey(key)) => {
                map.insert(String::from("key"), Value::String(key.clone()));
            }
            Self::I18n(I18nError::CatalogParity {
                missing_in_en,
                missing_in_zh,
            }) => {
                map.insert(
                    String::from("missing_in_en"),
                    Value::Array(missing_in_en.iter().cloned().map(Value::String).collect()),
                );
                map.insert(
                    String::from("missing_in_zh"),
                    Value::Array(missing_in_zh.iter().cloned().map(Value::String).collect()),
                );
            }
            Self::Storage(StorageError::AtomicWriteFailed { path, reason }) => {
                map.insert(
                    String::from("path"),
                    Value::String(path.display().to_string()),
                );
                map.insert(String::from("reason"), Value::String(reason.clone()));
            }
            Self::Io(err) => {
                map.insert(String::from("reason"), Value::String(err.to_string()));
            }
            _ => {}
        }
        Value::Object(map)
    }
}
