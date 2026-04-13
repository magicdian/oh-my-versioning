use crate::core::adapter::{AgentAdapter, SpecAdapter};
use crate::errors::{CliError, OmvError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Text,
    Json,
}

impl OutputMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Init,
    Bump,
    Sync,
    Current,
    Adapter(AdapterCommand),
    Help,
    Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterCommand {
    pub action: AdapterAction,
    pub agents: Vec<AgentAdapter>,
    pub specs: Vec<SpecAdapter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterAction {
    Install,
    Refresh,
    List,
    Status,
}

impl AdapterAction {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "install" => Some(Self::Install),
            "refresh" => Some(Self::Refresh),
            "list" => Some(Self::List),
            "status" => Some(Self::Status),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    pub command: Command,
    pub locale_override: Option<String>,
    pub ntp_override: Option<bool>,
    pub output_mode: OutputMode,
}

pub fn parse_from_env() -> Result<Cli, OmvError> {
    parse_args(std::env::args().skip(1).collect::<Vec<_>>())
}

pub fn detect_output_mode(raw_args: &[String]) -> OutputMode {
    let mut iter = raw_args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => return OutputMode::Json,
            "--output" => {
                if let Some(value) = iter.next() {
                    if let Some(mode) = OutputMode::parse(value) {
                        return mode;
                    }
                }
            }
            _ => {}
        }
    }
    OutputMode::Text
}

pub fn detect_locale_override(raw_args: &[String]) -> Option<String> {
    let mut iter = raw_args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--locale" {
            return iter.next().cloned();
        }
    }
    None
}

pub fn parse_args(args: Vec<String>) -> Result<Cli, OmvError> {
    let (remaining, locale_override, ntp_override, output_mode) = strip_global_flags(args)?;
    let mut args = remaining.into_iter();
    let mut command = Command::Help;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "init" => command = Command::Init,
            "bump" => command = Command::Bump,
            "sync" => command = Command::Sync,
            "current" => command = Command::Current,
            "adapter" => command = Command::Adapter(parse_adapter_command(&mut args)?),
            "version" | "--version" | "-V" => command = Command::Version,
            "help" | "--help" | "-h" => command = Command::Help,
            other if other.starts_with("--") => return Err(CliError::UnknownOption(arg).into()),
            other => return Err(CliError::UnknownCommand(other.to_owned()).into()),
        }
    }

    Ok(Cli {
        command,
        locale_override,
        ntp_override,
        output_mode,
    })
}

fn strip_global_flags(
    args: Vec<String>,
) -> Result<(Vec<String>, Option<String>, Option<bool>, OutputMode), OmvError> {
    let mut remaining = Vec::new();
    let mut locale_override = None;
    let mut ntp_override = None;
    let mut output_mode = OutputMode::Text;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--locale" => {
                let value = iter.next().ok_or(CliError::MissingLocaleValue)?;
                locale_override = Some(value);
            }
            "--no-ntp" => ntp_override = Some(false),
            "--json" => output_mode = OutputMode::Json,
            "--output" => {
                let value = iter.next().ok_or(CliError::MissingOutputValue)?;
                output_mode =
                    OutputMode::parse(&value).ok_or_else(|| CliError::InvalidOutputMode(value))?;
            }
            _ => remaining.push(arg),
        }
    }

    Ok((remaining, locale_override, ntp_override, output_mode))
}

fn parse_adapter_command(
    args: &mut std::vec::IntoIter<String>,
) -> Result<AdapterCommand, OmvError> {
    let action_value = args.next().ok_or(CliError::MissingAdapterAction)?;
    let action = AdapterAction::parse(&action_value)
        .ok_or_else(|| CliError::UnknownAdapterAction(action_value.clone()))?;

    let mut agents = Vec::new();
    let mut specs = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--agent" => {
                let value = args.next().ok_or(CliError::MissingAgentValue)?;
                let adapter = AgentAdapter::parse(&value)
                    .ok_or_else(|| CliError::UnknownAgentAdapter(value.clone()))?;
                agents.push(adapter);
            }
            "--spec" => {
                let value = args.next().ok_or(CliError::MissingSpecValue)?;
                let adapter = SpecAdapter::parse(&value)
                    .ok_or_else(|| CliError::UnknownSpecAdapter(value.clone()))?;
                specs.push(adapter);
            }
            other if other.starts_with("--") => return Err(CliError::UnknownOption(arg).into()),
            other => return Err(CliError::UnknownAdapterAction(other.to_owned()).into()),
        }
    }

    Ok(AdapterCommand {
        action,
        agents,
        specs,
    })
}

#[cfg(test)]
mod tests {
    use crate::cli::{AdapterAction, Command, OutputMode, parse_args};
    use crate::errors::{CliError, OmvError};

    #[test]
    fn parses_version_short_flag() {
        let cli = parse_args(vec!["-V".to_owned()]).expect("version flag should parse");
        assert_eq!(cli.command, Command::Version);
    }

    #[test]
    fn parses_locale_override_with_command() {
        let cli = parse_args(vec![
            "init".to_owned(),
            "--locale".to_owned(),
            "zh-CN".to_owned(),
        ])
        .expect("locale override should parse");

        assert_eq!(cli.command, Command::Init);
        assert_eq!(cli.locale_override.as_deref(), Some("zh-CN"));
        assert_eq!(cli.ntp_override, None);
    }

    #[test]
    fn defaults_to_help_without_args() {
        let cli = parse_args(Vec::<String>::new()).expect("empty args should parse");
        assert_eq!(cli.command, Command::Help);
        assert_eq!(cli.ntp_override, None);
        assert_eq!(cli.output_mode, OutputMode::Text);
    }

    #[test]
    fn returns_error_when_locale_value_missing() {
        let err = parse_args(vec!["--locale".to_owned()]).expect_err("missing locale must fail");
        assert!(matches!(err, OmvError::Cli(CliError::MissingLocaleValue)));
    }

    #[test]
    fn parses_no_ntp_flag() {
        let cli = parse_args(vec!["bump".to_owned(), "--no-ntp".to_owned()])
            .expect("no-ntp flag should parse");
        assert_eq!(cli.command, Command::Bump);
        assert_eq!(cli.ntp_override, Some(false));
    }

    #[test]
    fn parses_json_shortcut() {
        let cli = parse_args(vec!["current".to_owned(), "--json".to_owned()])
            .expect("json flag should parse");
        assert_eq!(cli.command, Command::Current);
        assert_eq!(cli.output_mode, OutputMode::Json);
    }

    #[test]
    fn parses_output_mode_json() {
        let cli = parse_args(vec![
            "bump".to_owned(),
            "--output".to_owned(),
            "json".to_owned(),
        ])
        .expect("output mode should parse");
        assert_eq!(cli.output_mode, OutputMode::Json);
    }

    #[test]
    fn parses_adapter_install_with_agent_and_spec() {
        let cli = parse_args(vec![
            "adapter".to_owned(),
            "install".to_owned(),
            "--agent".to_owned(),
            "codex".to_owned(),
            "--spec".to_owned(),
            "openspec".to_owned(),
        ])
        .expect("adapter install should parse");

        match cli.command {
            Command::Adapter(adapter) => {
                assert_eq!(adapter.action, AdapterAction::Install);
                assert_eq!(adapter.agents.len(), 1);
                assert_eq!(adapter.specs.len(), 1);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_agent_adapter() {
        let err = parse_args(vec![
            "adapter".to_owned(),
            "install".to_owned(),
            "--agent".to_owned(),
            "cursor".to_owned(),
        ])
        .expect_err("unknown adapter should fail");
        assert!(matches!(
            err,
            OmvError::Cli(CliError::UnknownAgentAdapter(_))
        ));
    }
}
