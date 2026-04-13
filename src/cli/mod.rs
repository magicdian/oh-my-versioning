use crate::errors::{CliError, OmvError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Init,
    Bump,
    Sync,
    Help,
    Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    pub command: Command,
    pub locale_override: Option<String>,
}

pub fn parse_from_env() -> Result<Cli, OmvError> {
    parse_args(std::env::args().skip(1))
}

fn parse_args<I>(args: I) -> Result<Cli, OmvError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let mut command = Command::Help;
    let mut locale_override = None;

    while let Some(arg) = args.next() {
        if arg == "--locale" {
            let value = args.next().ok_or(CliError::MissingLocaleValue)?;
            locale_override = Some(value);
            continue;
        }

        command = match arg.as_str() {
            "init" => Command::Init,
            "bump" => Command::Bump,
            "sync" => Command::Sync,
            "version" | "--version" | "-V" => Command::Version,
            "help" | "--help" | "-h" => Command::Help,
            other => return Err(CliError::UnknownCommand(other.to_owned()).into()),
        };
    }

    Ok(Cli {
        command,
        locale_override,
    })
}

#[cfg(test)]
mod tests {
    use crate::cli::{Command, parse_args};
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
    }

    #[test]
    fn defaults_to_help_without_args() {
        let cli = parse_args(Vec::<String>::new()).expect("empty args should parse");
        assert_eq!(cli.command, Command::Help);
    }

    #[test]
    fn returns_error_when_locale_value_missing() {
        let err = parse_args(vec!["--locale".to_owned()]).expect_err("missing locale must fail");
        assert!(matches!(err, OmvError::Cli(CliError::MissingLocaleValue)));
    }
}
