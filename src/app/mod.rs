use std::io::IsTerminal;
use std::path::Path;

use crate::cli::{Cli, Command};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppOutput {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BumpExecution {
    version: String,
    time_source: LastTimeSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SyncExecution {
    version: String,
    synced: usize,
    skipped: usize,
}

pub fn run(cli: Cli) -> Result<AppOutput, OmvError> {
    let cwd = std::env::current_dir()?;
    let omv_root = storage::resolve_omv_root(&cwd)?;

    let locale = resolve_locale_for_root(&omv_root, cli.locale_override.as_deref())?;
    let catalog = i18n::load_catalog(&locale)?;

    match cli.command {
        Command::Init => run_init(&omv_root, &catalog, &locale),
        Command::Bump => run_bump(&omv_root, &catalog),
        Command::Sync => run_sync(&omv_root, &catalog),
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
        format!("  {}", catalog.t("cli.help.commands.help")),
        format!("  {}", catalog.t("cli.help.commands.version")),
        String::new(),
        catalog.t("cli.help.options.title"),
        format!("  {}", catalog.t("cli.help.options.help")),
        format!("  {}", catalog.t("cli.help.options.version")),
        format!("  {}", catalog.t("cli.help.options.locale")),
        String::new(),
        catalog.t("cli.help.examples.title"),
        format!("  {}", catalog.t("cli.help.examples.init")),
        format!("  {}", catalog.t("cli.help.examples.bump")),
        format!("  {}", catalog.t("cli.help.examples.sync")),
        format!("  {}", catalog.t("cli.help.examples.locale")),
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

fn run_init(omv_root: &Path, catalog: &Catalog, locale: &str) -> Result<AppOutput, OmvError> {
    let project_root = omv_root.parent().unwrap_or(omv_root);
    let discovery = crate::ui::discovery::discover_languages(project_root);

    let draft = if std::io::stdout().is_terminal() {
        crate::ui::runtime::run_init_tui(catalog, &discovery)?
    } else {
        init_ui_state::UiApp::from_discovery(&discovery).draft
    };

    persist_init_state(omv_root, locale, &draft)?;

    Ok(AppOutput {
        message: catalog.t("init.result.saved"),
    })
}

fn run_bump(omv_root: &Path, catalog: &Catalog) -> Result<AppOutput, OmvError> {
    let ntp = NtpTimeSource::default();
    let system = SystemTimeSource;
    let execution = execute_bump(omv_root, &ntp, &system)?;

    Ok(AppOutput {
        message: catalog.tf(
            "cli.bump.success",
            &[
                "version",
                execution.version.as_str(),
                "source",
                execution.time_source.as_str(),
            ],
        ),
    })
}

fn run_sync(omv_root: &Path, catalog: &Catalog) -> Result<AppOutput, OmvError> {
    let execution = execute_sync(omv_root)?;
    let synced = execution.synced.to_string();
    let skipped = execution.skipped.to_string();

    Ok(AppOutput {
        message: catalog.tf(
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
    })
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

fn execute_bump(
    omv_root: &Path,
    ntp_source: &dyn TimeSource,
    system_source: &dyn TimeSource,
) -> Result<BumpExecution, OmvError> {
    let config = storage::config::load_config(omv_root)?;
    let state = storage::state::load_state(omv_root)?;

    let validated =
        crate::core::time::validate_current_date(&config, &state, ntp_source, system_source)?;
    let next = engine::compute_next_version(&config, &state, validated.date)?;

    let mut next_state = state;
    next_state.logical_date = next.logical_date.to_iso_string();
    next_state.build_number = next.build_number;
    next_state.last_issued_version = next.value.clone();
    next_state.last_time_source = validated.source;

    storage::state::save_state(omv_root, &next_state)?;
    execute_sync(omv_root)?;

    Ok(BumpExecution {
        version: next.value,
        time_source: validated.source,
    })
}

fn execute_sync(omv_root: &Path) -> Result<SyncExecution, OmvError> {
    let state = storage::state::load_state(omv_root)?;
    let targets = load_targets_if_exists(omv_root)?;
    let project_root = omv_root.parent().unwrap_or(omv_root);

    let summary =
        crate::sync::sync_all_targets(project_root, &targets, &state.last_issued_version)?;
    crate::sync::skills::generate_skills(omv_root, &state.last_issued_version)?;

    Ok(SyncExecution {
        version: state.last_issued_version,
        synced: summary.synced,
        skipped: summary.skipped,
    })
}

fn persist_init_state(omv_root: &Path, locale: &str, draft: &InitDraft) -> Result<(), OmvError> {
    std::fs::create_dir_all(omv_root)?;

    let mut config = load_config_if_exists(omv_root)?.unwrap_or_default();
    config.locale = OperatorLocale::from_input(locale);
    storage::config::save_config(omv_root, &config)?;

    let targets = build_targets_from_draft(draft);
    storage::targets::save_targets(omv_root, &targets)?;

    ensure_state_exists(omv_root, &config)?;

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

    use crate::core::date::LogicalDate;
    use crate::core::locale::OperatorLocale;
    use crate::core::schema::{OmvConfig, OmvState};
    use crate::core::target::TargetLanguage;
    use crate::core::time::{LastTimeSource, TimeSource};
    use crate::errors::OmvError;
    use crate::storage;
    use crate::ui::state::draft::InitDraft;

    use super::{
        execute_bump, execute_sync, persist_init_state, render_help, render_version,
        resolve_locale_for_root,
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
    fn persist_init_state_writes_targets_and_initial_state() {
        let omv_root = temp_omv_root("persist-init");
        let draft = InitDraft::from_detected_languages(&[TargetLanguage::Rust, TargetLanguage::Go]);

        persist_init_state(&omv_root, "zh-CN", &draft).expect("init state should persist");

        let config = storage::config::load_config(&omv_root).expect("config should load");
        assert_eq!(config.locale, OperatorLocale::ZhCn);

        let targets = storage::targets::load_targets(&omv_root).expect("targets should load");
        assert_eq!(targets.targets.len(), TargetLanguage::all().len());
        assert!(
            targets
                .targets
                .iter()
                .any(|target| { target.language == TargetLanguage::Rust && target.enabled })
        );
        assert!(
            targets
                .targets
                .iter()
                .any(|target| { target.language == TargetLanguage::Go && target.enabled })
        );

        let state = storage::state::load_state(&omv_root).expect("state should load");
        assert_eq!(state.build_number, 1);

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn execute_sync_writes_manifests_runtime_exports_and_skills() {
        let omv_root = temp_omv_root("execute-sync");
        let draft = InitDraft::from_detected_languages(&[TargetLanguage::Rust]);
        persist_init_state(&omv_root, "en-US", &draft).expect("init state should persist");

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

        let runtime = fs::read_to_string(project_root.join("src/generated/version.rs"))
            .expect("runtime export should exist");
        assert!(runtime.contains("2604.13.9"));

        let guidance = fs::read_to_string(omv_root.join("skills/bump-guidance.md"))
            .expect("skills guidance should exist");
        assert!(guidance.contains("omv bump"));

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn execute_bump_updates_state_and_returns_version() {
        let omv_root = temp_omv_root("execute-bump-ok");

        let config = OmvConfig::default();
        storage::config::save_config(&omv_root, &config).expect("config should save");

        let state = OmvState {
            logical_date: "2026-04-13".to_owned(),
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
            date: LogicalDate::parse_iso("2026-04-12").expect("date should parse"),
        };

        let execution = execute_bump(&omv_root, &ntp, &system).expect("bump should succeed");
        assert_eq!(execution.version, "2604.13.2");
        assert_eq!(execution.time_source, LastTimeSource::Ntp);

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

        let err = execute_bump(&omv_root, &ntp, &system).expect_err("future date must fail");
        assert!(matches!(err, OmvError::Time(_)));

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn render_help_includes_structured_sections() {
        let catalog = crate::i18n::load_catalog("en-US").expect("catalog should load");
        let help = render_help(&catalog);

        assert!(help.contains("Usage:"));
        assert!(help.contains("Commands:"));
        assert!(help.contains("Options:"));
        assert!(help.contains("Examples:"));
        assert!(help.contains("omv init"));
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

    fn cleanup_omv_root(root: &std::path::Path) {
        if let Some(parent) = root.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
