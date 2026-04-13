use std::io::IsTerminal;
use std::path::Path;

use serde::Serialize;

use crate::adapter;
use crate::cli::{AdapterAction, AdapterCommand, Cli, Command, OutputMode};
use crate::core::date::LogicalDate;
use crate::core::locale::OperatorLocale;
use crate::core::schema::{OmvConfig, OmvState, OmvTargetRecord, OmvTargets};
use crate::core::target::TargetLanguage;
use crate::core::time::ntp::NtpTimeSource;
use crate::core::time::{LastTimeSource, SystemTimeSource, TimeSource};
use crate::core::versioning::engine;
use crate::errors::{ConfigError, OmvError, StateError, TargetError};
use crate::i18n::{self, Catalog};
use crate::storage;
use crate::ui::app as init_ui_state;
use crate::ui::state::draft::InitDraft;

const STRUCTURED_CONTRACT_VERSION: &str = "1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppOutput {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct BumpExecution {
    previous_version: String,
    version: String,
    logical_date: String,
    build_number: u32,
    time_source: String,
    synced: usize,
    skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SyncExecution {
    version: String,
    synced: usize,
    skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct CurrentExecution {
    version: String,
    logical_date: String,
    build_number: u32,
    build_policy: String,
    version_output: String,
    last_time_source: String,
    omv_root: String,
    enabled_targets: usize,
    total_targets: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct StructuredEnvelope<T: Serialize> {
    ok: bool,
    contract_version: &'static str,
    command: String,
    data: Option<T>,
    error: Option<crate::errors::StructuredError>,
}

pub fn run(cli: Cli) -> Result<AppOutput, OmvError> {
    let cwd = std::env::current_dir()?;
    let omv_root = storage::resolve_omv_root(&cwd)?;

    let locale = resolve_locale_for_root(&omv_root, cli.locale_override.as_deref())?;
    let catalog = i18n::load_catalog(&locale)?;
    let ntp_override = cli.ntp_override;

    match cli.command {
        Command::Init => run_init(&omv_root, &catalog, &locale, cli.output_mode),
        Command::Bump => run_bump(&omv_root, &catalog, ntp_override, cli.output_mode),
        Command::Sync => run_sync(&omv_root, &catalog, cli.output_mode),
        Command::Current => run_current(&omv_root, &catalog, cli.output_mode),
        Command::Adapter(adapter_command) => {
            run_adapter(&cwd, &omv_root, &catalog, cli.output_mode, adapter_command)
        }
        Command::Help => Ok(AppOutput {
            message: render_help(&catalog),
        }),
        Command::Version => Ok(AppOutput {
            message: render_version(&catalog),
        }),
    }
}

fn render_help(catalog: &Catalog) -> String {
    [
        catalog.t("cli.help.title"),
        String::new(),
        catalog.t("cli.help.description"),
        String::new(),
        catalog.t("cli.help.usage.title"),
        format!("  {}", catalog.t("cli.help.usage.value")),
        String::new(),
        catalog.t("cli.help.commands.title"),
        format!("  {}", catalog.t("cli.help.commands.init")),
        format!("  {}", catalog.t("cli.help.commands.bump")),
        format!("  {}", catalog.t("cli.help.commands.sync")),
        format!("  {}", catalog.t("cli.help.commands.current")),
        format!("  {}", catalog.t("cli.help.commands.adapter")),
        format!("  {}", catalog.t("cli.help.commands.help")),
        format!("  {}", catalog.t("cli.help.commands.version")),
        String::new(),
        catalog.t("cli.help.options.title"),
        format!("  {}", catalog.t("cli.help.options.help")),
        format!("  {}", catalog.t("cli.help.options.version")),
        format!("  {}", catalog.t("cli.help.options.locale")),
        format!("  {}", catalog.t("cli.help.options.no_ntp")),
        format!("  {}", catalog.t("cli.help.options.json")),
        format!("  {}", catalog.t("cli.help.options.output")),
        String::new(),
        catalog.t("cli.help.examples.title"),
        format!("  {}", catalog.t("cli.help.examples.init")),
        format!("  {}", catalog.t("cli.help.examples.bump")),
        format!("  {}", catalog.t("cli.help.examples.sync")),
        format!("  {}", catalog.t("cli.help.examples.current")),
        format!("  {}", catalog.t("cli.help.examples.adapter")),
        format!("  {}", catalog.t("cli.help.examples.locale")),
        format!("  {}", catalog.t("cli.help.examples.no_ntp")),
    ]
    .join("\n")
}

fn render_version(catalog: &Catalog) -> String {
    catalog.tf(
        "cli.version.output",
        &["version", env!("CARGO_PKG_VERSION")],
    )
}

pub fn render_error(locale: &str, err: &OmvError) -> String {
    let catalog = i18n::load_catalog(locale).or_else(|_| i18n::load_catalog("en-US"));

    let detail = err.to_string();
    match catalog {
        Ok(cat) => match err {
            OmvError::Cli(_) => cat.tf("error.cli", &["detail", detail.as_str()]),
            OmvError::Adapter(_) => cat.tf("error.adapter", &["detail", detail.as_str()]),
            OmvError::Config(_) => cat.tf("error.config", &["detail", detail.as_str()]),
            OmvError::State(_) => cat.tf("error.state", &["detail", detail.as_str()]),
            OmvError::Time(_) => cat.tf("error.time", &["detail", detail.as_str()]),
            OmvError::Ntp(_) => cat.tf("error.ntp", &["detail", detail.as_str()]),
            OmvError::Target(_) => cat.tf("error.target", &["detail", detail.as_str()]),
            OmvError::I18n(_) => cat.tf("error.i18n", &["detail", detail.as_str()]),
            OmvError::Storage(_) | OmvError::Io(_) => {
                cat.tf("error.storage", &["detail", detail.as_str()])
            }
        },
        Err(_) => format!("omv error: {detail}"),
    }
}

pub fn render_structured_error(command: &str, err: &OmvError) -> String {
    let envelope = StructuredEnvelope::<serde_json::Value> {
        ok: false,
        contract_version: STRUCTURED_CONTRACT_VERSION,
        command: command.to_owned(),
        data: None,
        error: Some(err.structured_error()),
    };
    serde_json::to_string_pretty(&envelope).expect("structured error should serialize")
}

fn run_init(
    omv_root: &Path,
    catalog: &Catalog,
    locale: &str,
    output_mode: OutputMode,
) -> Result<AppOutput, OmvError> {
    let project_root = omv_root.parent().unwrap_or(omv_root);
    let discovery = crate::ui::discovery::discover_languages(project_root);

    let draft = if std::io::stdout().is_terminal() {
        crate::ui::runtime::run_init_tui(catalog, &discovery, locale)?
    } else {
        let mut draft = init_ui_state::UiApp::from_discovery(&discovery).draft;
        draft.set_locale(OperatorLocale::from_input(locale));
        draft
    };

    persist_init_state(omv_root, &draft)?;

    let message = match output_mode {
        OutputMode::Text => catalog.t("init.result.saved"),
        OutputMode::Json => render_structured_success(
            "init",
            serde_json::json!({
                "saved": true,
                "omv_root": omv_root.display().to_string()
            }),
        ),
    };

    Ok(AppOutput { message })
}

fn run_bump(
    omv_root: &Path,
    catalog: &Catalog,
    ntp_override: Option<bool>,
    output_mode: OutputMode,
) -> Result<AppOutput, OmvError> {
    let ntp = NtpTimeSource::default();
    let system = SystemTimeSource;
    let execution = execute_bump(omv_root, &ntp, &system, ntp_override)?;

    let message = match output_mode {
        OutputMode::Text => catalog.tf(
            "cli.bump.success",
            &[
                "version",
                execution.version.as_str(),
                "source",
                execution.time_source.as_str(),
            ],
        ),
        OutputMode::Json => render_structured_success("bump", &execution),
    };

    Ok(AppOutput { message })
}

fn run_sync(
    omv_root: &Path,
    catalog: &Catalog,
    output_mode: OutputMode,
) -> Result<AppOutput, OmvError> {
    let execution = execute_sync(omv_root)?;
    let synced = execution.synced.to_string();
    let skipped = execution.skipped.to_string();

    let message = match output_mode {
        OutputMode::Text => catalog.tf(
            "cli.sync.success",
            &[
                "version",
                execution.version.as_str(),
                "synced",
                synced.as_str(),
                "skipped",
                skipped.as_str(),
            ],
        ),
        OutputMode::Json => render_structured_success("sync", &execution),
    };

    Ok(AppOutput { message })
}

fn run_current(
    omv_root: &Path,
    catalog: &Catalog,
    output_mode: OutputMode,
) -> Result<AppOutput, OmvError> {
    let execution = execute_current(omv_root)?;
    let build = execution.build_number.to_string();
    let message = match output_mode {
        OutputMode::Text => catalog.tf(
            "cli.current.success",
            &[
                "version",
                execution.version.as_str(),
                "build",
                build.as_str(),
                "date",
                execution.logical_date.as_str(),
            ],
        ),
        OutputMode::Json => render_structured_success("current", &execution),
    };

    Ok(AppOutput { message })
}

fn run_adapter(
    cwd: &Path,
    omv_root: &Path,
    catalog: &Catalog,
    output_mode: OutputMode,
    command: AdapterCommand,
) -> Result<AppOutput, OmvError> {
    let project_root = storage::resolve_project_root(cwd)?;
    let selection = adapter::AdapterSelection {
        agents: command.agents,
        specs: command.specs,
    };

    let message = match command.action {
        AdapterAction::Install => {
            let summary = adapter::install_selected(omv_root, &project_root, &selection)?;
            let count = summary.installed.len().to_string();
            match output_mode {
                OutputMode::Text => {
                    catalog.tf("cli.adapter.install.success", &["count", count.as_str()])
                }
                OutputMode::Json => render_structured_success("adapter.install", &summary),
            }
        }
        AdapterAction::Refresh => {
            let summary = adapter::refresh_selected(omv_root, &project_root, &selection)?;
            let count = summary.installed.len().to_string();
            match output_mode {
                OutputMode::Text => {
                    catalog.tf("cli.adapter.refresh.success", &["count", count.as_str()])
                }
                OutputMode::Json => render_structured_success("adapter.refresh", &summary),
            }
        }
        AdapterAction::List => {
            let summary = adapter::status(omv_root)?;
            match output_mode {
                OutputMode::Text => render_adapter_list_text(catalog, &summary.available),
                OutputMode::Json => render_structured_success("adapter.list", &summary.available),
            }
        }
        AdapterAction::Status => {
            let summary = adapter::status(omv_root)?;
            match output_mode {
                OutputMode::Text => render_adapter_status_text(catalog, &summary),
                OutputMode::Json => render_structured_success("adapter.status", &summary),
            }
        }
    };

    Ok(AppOutput { message })
}

fn render_structured_success<T: Serialize>(command: &str, data: T) -> String {
    let envelope = StructuredEnvelope {
        ok: true,
        contract_version: STRUCTURED_CONTRACT_VERSION,
        command: command.to_owned(),
        data: Some(data),
        error: None,
    };
    serde_json::to_string_pretty(&envelope).expect("structured success should serialize")
}

fn render_adapter_list_text(catalog: &Catalog, available: &adapter::AdapterCatalog) -> String {
    let agents = available.agents.join(", ");
    let specs = available.specs.join(", ");
    [
        catalog.t("cli.adapter.list.header"),
        catalog.tf("cli.adapter.list.agents", &["value", agents.as_str()]),
        catalog.tf("cli.adapter.list.specs", &["value", specs.as_str()]),
    ]
    .join("\n")
}

fn render_adapter_status_text(catalog: &Catalog, status: &adapter::AdapterStatusSummary) -> String {
    if status.installed.is_empty() {
        return [
            catalog.t("cli.adapter.status.header"),
            catalog.t("cli.adapter.status.none"),
        ]
        .join("\n");
    }

    let mut lines = vec![catalog.t("cli.adapter.status.header")];
    for installation in &status.installed {
        lines.push(catalog.tf(
            "cli.adapter.status.item",
            &[
                "kind",
                installation.kind.as_str(),
                "name",
                installation.name.as_str(),
                "mode",
                installation.install_mode.as_str(),
            ],
        ));
    }
    lines.join("\n")
}

fn resolve_locale_for_root(
    omv_root: &Path,
    locale_override: Option<&str>,
) -> Result<String, OmvError> {
    let config = load_config_if_exists(omv_root)?;

    if let Some(override_input) = locale_override {
        let normalized = i18n::normalize_operator_locale(override_input).to_owned();

        if let Some(mut config) = config {
            let locale = OperatorLocale::from_input(&normalized);
            if config.locale != locale {
                config.locale = locale;
                storage::config::save_config(omv_root, &config)?;
            }
        }

        return Ok(normalized);
    }

    if let Some(config) = config {
        return Ok(config.locale.as_str().to_owned());
    }

    Ok(String::from("en-US"))
}

fn load_config_if_exists(omv_root: &Path) -> Result<Option<OmvConfig>, OmvError> {
    match storage::config::load_config(omv_root) {
        Ok(config) => Ok(Some(config)),
        Err(OmvError::Config(ConfigError::Missing { .. })) => Ok(None),
        Err(err) => Err(err),
    }
}

fn load_targets_if_exists(omv_root: &Path) -> Result<OmvTargets, OmvError> {
    match storage::targets::load_targets(omv_root) {
        Ok(targets) => Ok(targets),
        Err(OmvError::Target(TargetError::Missing { .. })) => Ok(OmvTargets::default()),
        Err(err) => Err(err),
    }
}

fn execute_current(omv_root: &Path) -> Result<CurrentExecution, OmvError> {
    adapter::ensure_canonical_artifacts(omv_root)?;
    let config = storage::config::load_config(omv_root)?;
    let state = storage::state::load_state(omv_root)?;
    let targets = load_targets_if_exists(omv_root)?;
    let enabled_targets = targets
        .targets
        .iter()
        .filter(|target| target.enabled)
        .count();

    Ok(CurrentExecution {
        version: state.last_issued_version,
        logical_date: state.logical_date,
        build_number: state.build_number,
        build_policy: config.build_policy.as_str().to_owned(),
        version_output: config.version_output.as_str().to_owned(),
        last_time_source: state.last_time_source.as_str().to_owned(),
        omv_root: omv_root.display().to_string(),
        enabled_targets,
        total_targets: targets.targets.len(),
    })
}

fn execute_bump(
    omv_root: &Path,
    ntp_source: &dyn TimeSource,
    system_source: &dyn TimeSource,
    ntp_override: Option<bool>,
) -> Result<BumpExecution, OmvError> {
    let mut config = storage::config::load_config(omv_root)?;
    if let Some(enabled) = ntp_override {
        config.ntp_enabled = enabled;
    }
    let state = storage::state::load_state(omv_root)?;
    let previous_version = state.last_issued_version.clone();

    let validated =
        crate::core::time::validate_current_date(&config, &state, ntp_source, system_source)?;
    let next = engine::compute_next_version(&config, &state, validated.date)?;

    let mut next_state = state;
    next_state.logical_date = next.logical_date.to_iso_string();
    next_state.build_number = next.build_number;
    next_state.last_issued_version = next.value.clone();
    next_state.last_time_source = validated.source;

    storage::state::save_state(omv_root, &next_state)?;
    let sync = execute_sync(omv_root)?;

    Ok(BumpExecution {
        previous_version,
        version: next.value,
        logical_date: next.logical_date.to_iso_string(),
        build_number: next.build_number,
        time_source: validated.source.as_str().to_owned(),
        synced: sync.synced,
        skipped: sync.skipped,
    })
}

fn execute_sync(omv_root: &Path) -> Result<SyncExecution, OmvError> {
    let state = storage::state::load_state(omv_root)?;
    let targets = load_targets_if_exists(omv_root)?;
    let project_root = omv_root.parent().unwrap_or(omv_root);

    let summary =
        crate::sync::sync_all_targets(project_root, &targets, &state.last_issued_version)?;
    crate::sync::skills::generate_skills(omv_root, &state.last_issued_version)?;
    adapter::ensure_canonical_artifacts(omv_root)?;

    Ok(SyncExecution {
        version: state.last_issued_version,
        synced: summary.synced,
        skipped: summary.skipped,
    })
}

fn persist_init_state(omv_root: &Path, draft: &InitDraft) -> Result<(), OmvError> {
    std::fs::create_dir_all(omv_root)?;

    let mut config = load_config_if_exists(omv_root)?.unwrap_or_default();
    config.locale = draft.locale;
    config.timezone = draft.timezone_string();
    config.build_policy = draft.build_policy;
    storage::config::save_config(omv_root, &config)?;

    let targets = build_targets_from_draft(draft);
    storage::targets::save_targets(omv_root, &targets)?;

    ensure_state_exists(omv_root, &config)?;
    adapter::ensure_canonical_artifacts(omv_root)?;

    Ok(())
}

fn build_targets_from_draft(draft: &InitDraft) -> OmvTargets {
    let targets = draft
        .targets
        .iter()
        .map(|target| {
            let (manifest_path, runtime_export_path) = default_target_paths(target.language);
            OmvTargetRecord {
                id: format!("workspace-{}", target.language.as_str()),
                language: target.language,
                root: String::from("."),
                manifest_path: manifest_path.to_owned(),
                runtime_export_path: runtime_export_path.to_owned(),
                strategy: target.strategy,
                enabled: target.enabled,
            }
        })
        .collect();

    OmvTargets {
        schema_version: 1,
        targets,
    }
}

fn ensure_state_exists(omv_root: &Path, config: &OmvConfig) -> Result<(), OmvError> {
    match storage::state::load_state(omv_root) {
        Ok(_) => Ok(()),
        Err(OmvError::State(StateError::MissingState { .. })) => {
            let today = LogicalDate::today_from_system()?;
            let version = engine::format_version(today, 1, config.version_output);
            let state = OmvState {
                schema_version: 1,
                logical_date: today.to_iso_string(),
                build_number: 1,
                last_issued_version: version,
                last_time_source: LastTimeSource::System,
            };
            storage::state::save_state(omv_root, &state)
        }
        Err(err) => Err(err),
    }
}

fn default_target_paths(language: TargetLanguage) -> (&'static str, &'static str) {
    match language {
        TargetLanguage::Rust => ("Cargo.toml", "src/generated/version.rs"),
        TargetLanguage::Python => ("pyproject.toml", "omv_generated/version.py"),
        TargetLanguage::Go => ("go.mod", "internal/omv/version.go"),
        TargetLanguage::Java => ("pom.xml", "src/main/java/omv/Version.java"),
        TargetLanguage::CFamily => ("CMakeLists.txt", "include/omv_version.h"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::cli::{AdapterAction, AdapterCommand, OutputMode};
    use crate::core::date::LogicalDate;
    use crate::core::locale::OperatorLocale;
    use crate::core::schema::{OmvConfig, OmvState};
    use crate::core::target::TargetLanguage;
    use crate::core::time::{LastTimeSource, TimeSource};
    use crate::core::versioning::BuildPolicy;
    use crate::errors::{NtpError, OmvError, StateError};
    use crate::storage;
    use crate::ui::state::draft::InitDraft;

    use super::{
        execute_bump, execute_current, execute_sync, persist_init_state, render_help,
        render_structured_error, render_version, resolve_locale_for_root, run_adapter,
    };

    struct FixedSource {
        source: LastTimeSource,
        date: LogicalDate,
    }

    impl TimeSource for FixedSource {
        fn source(&self) -> LastTimeSource {
            self.source
        }

        fn today(&self) -> Result<LogicalDate, OmvError> {
            Ok(self.date)
        }
    }

    #[test]
    fn resolve_locale_prefers_saved_config_without_override() {
        let omv_root = temp_omv_root("locale-prefer-config");
        let config = OmvConfig {
            locale: OperatorLocale::ZhCn,
            ..OmvConfig::default()
        };
        storage::config::save_config(&omv_root, &config).expect("config should save");

        let locale = resolve_locale_for_root(&omv_root, None).expect("locale should resolve");
        assert_eq!(locale, "zh-CN");

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn resolve_locale_override_persists_to_config() {
        let omv_root = temp_omv_root("locale-persist-override");
        let config = OmvConfig::default();
        storage::config::save_config(&omv_root, &config).expect("config should save");

        let locale =
            resolve_locale_for_root(&omv_root, Some("zh-CN")).expect("locale should resolve");
        assert_eq!(locale, "zh-CN");

        let updated = storage::config::load_config(&omv_root).expect("config should reload");
        assert_eq!(updated.locale, OperatorLocale::ZhCn);

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn persist_init_state_writes_targets_initial_state_and_ai_artifacts() {
        let omv_root = temp_omv_root("persist-init");
        let mut draft =
            InitDraft::from_detected_languages(&[TargetLanguage::Rust, TargetLanguage::Go]);
        draft.set_locale(OperatorLocale::ZhCn);
        draft.set_timezone_offset_hours(8);
        draft.set_build_policy(BuildPolicy::Continuous);

        persist_init_state(&omv_root, &draft).expect("init state should persist");

        let config = storage::config::load_config(&omv_root).expect("config should load");
        assert_eq!(config.locale, OperatorLocale::ZhCn);
        assert_eq!(config.timezone, "UTC+8");
        assert_eq!(config.build_policy, BuildPolicy::Continuous);

        let targets = storage::targets::load_targets(&omv_root).expect("targets should load");
        assert_eq!(targets.targets.len(), TargetLanguage::all().len());

        let state = storage::state::load_state(&omv_root).expect("state should load");
        assert_eq!(state.build_number, 1);
        assert!(omv_root.join("ai/contract.json").exists());

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn execute_current_reports_truth_state() {
        let omv_root = temp_omv_root("current");
        storage::config::save_config(&omv_root, &OmvConfig::default()).expect("config should save");
        storage::state::save_state(
            &omv_root,
            &OmvState {
                schema_version: 1,
                logical_date: String::from("2026-04-13"),
                build_number: 2,
                last_issued_version: String::from("2604.13.2"),
                last_time_source: LastTimeSource::System,
            },
        )
        .expect("state should save");

        let current = execute_current(&omv_root).expect("current should work");
        assert_eq!(current.version, "2604.13.2");
        assert_eq!(current.build_number, 2);

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn execute_sync_writes_manifests_runtime_exports_and_skills() {
        let omv_root = temp_omv_root("execute-sync");
        let mut draft = InitDraft::from_detected_languages(&[TargetLanguage::Rust]);
        draft.set_locale(OperatorLocale::EnUs);
        persist_init_state(&omv_root, &draft).expect("init state should persist");

        let mut state = storage::state::load_state(&omv_root).expect("state should load");
        state.last_issued_version = "2604.13.9".to_owned();
        storage::state::save_state(&omv_root, &state).expect("state should save");

        let result = execute_sync(&omv_root).expect("sync should succeed");
        assert_eq!(result.version, "2604.13.9");
        assert!(result.synced >= 1);

        let project_root = omv_root.parent().expect("project root should exist");
        let cargo =
            fs::read_to_string(project_root.join("Cargo.toml")).expect("Cargo.toml should sync");
        assert!(cargo.contains("2604.13.9"));

        let guidance = fs::read_to_string(omv_root.join("skills/bump-guidance.md"))
            .expect("skills guidance should exist");
        assert!(guidance.contains("omv bump"));
        assert!(omv_root.join("ai/instructions.md").exists());

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn execute_bump_updates_state_and_sync_summary() {
        let omv_root = temp_omv_root("execute-bump-ok");

        let config = OmvConfig::default();
        storage::config::save_config(&omv_root, &config).expect("config should save");

        let state = OmvState {
            logical_date: "2026-04-13".to_owned(),
            build_number: 1,
            last_issued_version: "2604.13.1".to_owned(),
            ..OmvState::default()
        };
        storage::state::save_state(&omv_root, &state).expect("state should save");

        let ntp = FixedSource {
            source: LastTimeSource::Ntp,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };
        let system = FixedSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-12").expect("date should parse"),
        };

        let execution = execute_bump(&omv_root, &ntp, &system, None).expect("bump should succeed");
        assert_eq!(execution.version, "2604.13.2");
        assert_eq!(execution.time_source, LastTimeSource::Ntp.as_str());

        let updated = storage::state::load_state(&omv_root).expect("updated state should load");
        assert_eq!(updated.build_number, 2);
        assert_eq!(updated.logical_date, "2026-04-13");
        assert_eq!(updated.last_issued_version, "2604.13.2");
        assert_eq!(updated.last_time_source, LastTimeSource::Ntp);

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn execute_bump_blocks_future_stored_date() {
        let omv_root = temp_omv_root("execute-bump-future");

        let config = OmvConfig::default();
        storage::config::save_config(&omv_root, &config).expect("config should save");

        let state = OmvState {
            logical_date: "2026-04-15".to_owned(),
            build_number: 1,
            ..OmvState::default()
        };
        storage::state::save_state(&omv_root, &state).expect("state should save");

        let ntp = FixedSource {
            source: LastTimeSource::Ntp,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };
        let system = FixedSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };

        let err = execute_bump(&omv_root, &ntp, &system, None).expect_err("future date must fail");
        assert!(matches!(err, OmvError::Time(_)));

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn execute_bump_can_skip_ntp_via_runtime_override() {
        let omv_root = temp_omv_root("execute-bump-no-ntp");
        let config = OmvConfig {
            ntp_enabled: true,
            ..OmvConfig::default()
        };
        storage::config::save_config(&omv_root, &config).expect("config should save");
        let state = OmvState {
            logical_date: "2026-04-13".to_owned(),
            build_number: 1,
            ..OmvState::default()
        };
        storage::state::save_state(&omv_root, &state).expect("state should save");

        struct FailingNtp;
        impl TimeSource for FailingNtp {
            fn source(&self) -> LastTimeSource {
                LastTimeSource::Ntp
            }
            fn today(&self) -> Result<LogicalDate, OmvError> {
                Err(OmvError::Ntp(NtpError::Unavailable(
                    "forced failure".to_owned(),
                )))
            }
        }

        let ntp = FailingNtp;
        let system = FixedSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };

        let execution = execute_bump(&omv_root, &ntp, &system, Some(false))
            .expect("bump should succeed with no-ntp override");
        assert_eq!(execution.time_source, LastTimeSource::System.as_str());

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn run_adapter_status_json_reports_installed_entries() {
        let root = temp_project_root("adapter-status");
        let omv_root = root.join(".omv");
        fs::create_dir_all(&omv_root).expect("omv root should exist");
        let catalog = crate::i18n::load_catalog("en-US").expect("catalog should load");

        run_adapter(
            &root,
            &omv_root,
            &catalog,
            OutputMode::Text,
            AdapterCommand {
                action: AdapterAction::Install,
                agents: vec![crate::core::adapter::AgentAdapter::Codex],
                specs: Vec::new(),
            },
        )
        .expect("adapter install should work");

        let output = run_adapter(
            &root,
            &omv_root,
            &catalog,
            OutputMode::Json,
            AdapterCommand {
                action: AdapterAction::Status,
                agents: Vec::new(),
                specs: Vec::new(),
            },
        )
        .expect("adapter status should work");
        assert!(output.message.contains("\"ok\": true"));
        assert!(output.message.contains("\"installed\""));

        cleanup_project_root(&root);
    }

    #[test]
    fn render_structured_error_serializes_error_envelope() {
        let err = OmvError::State(StateError::MissingState {
            path: PathBuf::from(".omv/state.toml"),
        });
        let output = render_structured_error("current", &err);
        assert!(output.contains("\"ok\": false"));
        assert!(output.contains("\"command\": \"current\""));
        assert!(output.contains("\"code\": \"missing_state\""));
    }

    #[test]
    fn render_help_includes_structured_sections_and_new_commands() {
        let catalog = crate::i18n::load_catalog("en-US").expect("catalog should load");
        let help = render_help(&catalog);

        assert!(help.contains("Usage:"));
        assert!(help.contains("current"));
        assert!(help.contains("adapter"));
        assert!(help.contains("--json"));
        assert!(help.contains("--output <MODE>"));
    }

    #[test]
    fn render_help_localizes_section_headers() {
        let catalog = crate::i18n::load_catalog("zh-CN").expect("catalog should load");
        let help = render_help(&catalog);

        assert!(help.contains("用法："));
        assert!(help.contains("命令："));
        assert!(help.contains("选项："));
        assert!(help.contains("示例："));
    }

    #[test]
    fn render_version_uses_package_version() {
        let catalog = crate::i18n::load_catalog("en-US").expect("catalog should load");
        let version = render_version(&catalog);
        assert_eq!(version, format!("omv {}", env!("CARGO_PKG_VERSION")));
    }

    fn temp_omv_root(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let root = std::env::temp_dir()
            .join(format!("omv-{prefix}-{stamp}"))
            .join(".omv");
        fs::create_dir_all(&root).expect("temp omv root should be created");
        root
    }

    fn temp_project_root(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("omv-app-{prefix}-{stamp}"));
        fs::create_dir_all(&root).expect("temp project root should be created");
        root
    }

    fn cleanup_project_root(root: &std::path::Path) {
        let _ = fs::remove_dir_all(root);
    }

    fn cleanup_omv_root(root: &std::path::Path) {
        if let Some(parent) = root.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
