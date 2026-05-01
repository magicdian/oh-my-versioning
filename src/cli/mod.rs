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
    Plan,
    Sync(SyncCommand),
    Current,
    Event(EventCommand),
    Adapter(AdapterCommand),
    Help,
    Version,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncCommand {
    pub check: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCommand {
    pub action: EventAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventAction {
    FinalizeTask(FinalizeTaskCommand),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FinalizeTaskCommand {
    pub task_id: Option<String>,
    pub change_type: Option<String>,
    pub status: Option<String>,
    pub tests: Option<String>,
    pub fingerprint: Option<String>,
    pub source: Option<String>,
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
                if let Some(value) = iter.next()
                    && let Some(mode) = OutputMode::parse(value)
                {
                    return mode;
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
            "plan" => command = Command::Plan,
            "sync" => command = Command::Sync(parse_sync_command(&mut args)?),
            "current" => command = Command::Current,
            "event" => command = Command::Event(parse_event_command(&mut args)?),
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

fn parse_sync_command(args: &mut std::vec::IntoIter<String>) -> Result<SyncCommand, OmvError> {
    let mut command = SyncCommand::default();

    for arg in args.by_ref() {
        match arg.as_str() {
            "--check" => command.check = true,
            other if other.starts_with("--") => return Err(CliError::UnknownOption(arg).into()),
            other => return Err(CliError::UnknownCommand(other.to_owned()).into()),
        }
    }

    Ok(command)
}

type GlobalFlags = (Vec<String>, Option<String>, Option<bool>, OutputMode);

fn strip_global_flags(args: Vec<String>) -> Result<GlobalFlags, OmvError> {
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
                    OutputMode::parse(&value).ok_or(CliError::InvalidOutputMode(value))?;
            }
            _ => remaining.push(arg),
        }
    }

    Ok((remaining, locale_override, ntp_override, output_mode))
}

fn parse_event_command(args: &mut std::vec::IntoIter<String>) -> Result<EventCommand, OmvError> {
    let action_value = args.next().ok_or(CliError::MissingEventAction)?;
    let action = match action_value.as_str() {
        "finalize-task" => EventAction::FinalizeTask(parse_finalize_task_command(args)?),
        other => return Err(CliError::UnknownEventAction(other.to_owned()).into()),
    };

    Ok(EventCommand { action })
}

fn parse_finalize_task_command(
    args: &mut std::vec::IntoIter<String>,
) -> Result<FinalizeTaskCommand, OmvError> {
    let mut command = FinalizeTaskCommand::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--task-id" => {
                command.task_id = Some(
                    args.next()
                        .ok_or_else(|| CliError::MissingEventFieldValue(arg.clone()))?,
                );
            }
            "--change-type" => {
                command.change_type = Some(
                    args.next()
                        .ok_or_else(|| CliError::MissingEventFieldValue(arg.clone()))?,
                );
            }
            "--status" => {
                command.status = Some(
                    args.next()
                        .ok_or_else(|| CliError::MissingEventFieldValue(arg.clone()))?,
                );
            }
            "--tests" => {
                command.tests = Some(
                    args.next()
                        .ok_or_else(|| CliError::MissingEventFieldValue(arg.clone()))?,
                );
            }
            "--fingerprint" => {
                command.fingerprint = Some(
                    args.next()
                        .ok_or_else(|| CliError::MissingEventFieldValue(arg.clone()))?,
                );
            }
            "--source" => {
                command.source = Some(
                    args.next()
                        .ok_or_else(|| CliError::MissingEventFieldValue(arg.clone()))?,
                );
            }
            other if other.starts_with("--") => return Err(CliError::UnknownOption(arg).into()),
            other => return Err(CliError::UnknownEventAction(other.to_owned()).into()),
        }
    }

    Ok(command)
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
    use crate::cli::{AdapterAction, Command, EventAction, OutputMode, parse_args};
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
    fn parses_plan_command() {
        let cli = parse_args(vec!["plan".to_owned(), "--json".to_owned()])
            .expect("plan command should parse");
        assert_eq!(cli.command, Command::Plan);
        assert_eq!(cli.output_mode, OutputMode::Json);
    }

    #[test]
    fn parses_sync_check() {
        let cli = parse_args(vec!["sync".to_owned(), "--check".to_owned()])
            .expect("sync check should parse");
        match cli.command {
            Command::Sync(command) => assert!(command.check),
            other => panic!("unexpected command: {other:?}"),
        }
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

    #[test]
    fn parses_event_finalize_task_with_all_fields() {
        let cli = parse_args(vec![
            "event".to_owned(),
            "finalize-task".to_owned(),
            "--task-id".to_owned(),
            "task-1".to_owned(),
            "--change-type".to_owned(),
            "bugfix".to_owned(),
            "--status".to_owned(),
            "done".to_owned(),
            "--tests".to_owned(),
            "passed".to_owned(),
            "--fingerprint".to_owned(),
            "task-1:v1".to_owned(),
            "--source".to_owned(),
            "trellis-finish-work".to_owned(),
        ])
        .expect("event finalize-task should parse");

        match cli.command {
            Command::Event(event) => match event.action {
                EventAction::FinalizeTask(command) => {
                    assert_eq!(command.task_id.as_deref(), Some("task-1"));
                    assert_eq!(command.change_type.as_deref(), Some("bugfix"));
                    assert_eq!(command.status.as_deref(), Some("done"));
                    assert_eq!(command.tests.as_deref(), Some("passed"));
                    assert_eq!(command.fingerprint.as_deref(), Some("task-1:v1"));
                    assert_eq!(command.source.as_deref(), Some("trellis-finish-work"));
                }
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_event_action() {
        let err = parse_args(vec!["event".to_owned(), "publish".to_owned()])
            .expect_err("unknown event action should fail");
        assert!(matches!(
            err,
            OmvError::Cli(CliError::UnknownEventAction(_))
        ));
    }
}
