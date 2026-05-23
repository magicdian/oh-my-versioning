use std::collections::BTreeSet;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::adapter;
use crate::cli::{
    AdapterAction, AdapterCommand, Cli, Command, EventAction, EventCommand,
    FinalizeBoundaryCommand, FinalizeTaskCommand, IntegrateAction, IntegrateCommand, OutputMode,
};
use crate::contract::registry::STRUCTURED_JSON_CONTRACT_VERSION;
use crate::core::date::LogicalDate;
use crate::core::finalization::{
    self, ChangeType, FinalizationOutcome, FinalizationReason, TaskStatus, TestsStatus,
};
use crate::core::integration::{
    IntegrationCapability, IntegrationCapabilityDescriptor, IntegrationCapabilityStatus,
    IntegrationDetectionSnapshot, IntegrationFailure, IntegrationProvider,
    IntegrationProviderDescriptor, OmvIntegrationCapabilityState, OmvIntegrationProviderState,
    OmvIntegrations, mvp_provider_descriptors,
};
use crate::core::locale::OperatorLocale;
use crate::core::schema::{
    OmvAdapterInstallation, OmvAdapterTarget, OmvConfig, OmvFinalizationRecord, OmvState,
    OmvTargetRecord, OmvTargets,
};
use crate::core::target::TargetLanguage;
use crate::core::time::ntp::NtpTimeSource;
use crate::core::time::{LastTimeSource, SystemTimeSource, TimeSource};
use crate::core::versioning::engine;
use crate::errors::{
    AdapterError, ConfigError, FinalizationError, IntegrationError, OmvError, StateError,
    TargetError,
};
use crate::i18n::{self, Catalog};
use crate::storage;
use crate::ui::app as init_ui_state;
use crate::ui::state::draft::{InitDraft, integration_capability_target_files};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppOutput {
    pub message: String,
}

pub struct AppRuntime<'a> {
    pub ntp_source: &'a dyn TimeSource,
    pub system_source: &'a dyn TimeSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct InitPersistenceOutcome {
    integrations: InitIntegrationOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct InitIntegrationOutcome {
    selected_capabilities: usize,
    status: InitIntegrationApplyStatus,
    reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum InitIntegrationApplyStatus {
    NoSelection,
    Applied,
    Deferred,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct FinalizeTaskRequest {
    task_id: String,
    change_type: ChangeType,
    task_status: TaskStatus,
    tests_status: TestsStatus,
    fingerprint: String,
    source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct FinalizeTaskExecution {
    task_id: String,
    fingerprint: String,
    change_type: String,
    status: String,
    tests: String,
    source: String,
    outcome: String,
    reason: String,
    duplicate: bool,
    recovered: bool,
    version_before: String,
    version_after: String,
    synced: usize,
    skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct FinalizeBoundaryExecution {
    provider: String,
    boundary: String,
    source: String,
    task_id: String,
    change_type: Option<String>,
    status: String,
    tests: String,
    fingerprint: String,
    workspace_snapshot_hash: String,
    outcome: String,
    reason: String,
    manual_action_required: bool,
    finalize_task: Option<FinalizeTaskExecution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct StructuredEnvelope<T: Serialize> {
    ok: bool,
    contract_version: &'static str,
    command: String,
    data: Option<T>,
    error: Option<crate::errors::StructuredError>,
}

const MANAGED_BEGIN_PREFIX: &str = "<!-- OMV-MANAGED-BEGIN:";
const MANAGED_END_PREFIX: &str = "<!-- OMV-MANAGED-END:";
const TRELLIS_FINISH_WORK_V05_PATH: &str = ".agents/skills/trellis-finish-work/SKILL.md";
const TRELLIS_FINISH_WORK_V04_PATH: &str = ".agents/skills/finish-work/SKILL.md";
const TRELLIS_FINISH_WORK_PATHS: [&str; 2] =
    [TRELLIS_FINISH_WORK_V05_PATH, TRELLIS_FINISH_WORK_V04_PATH];
const TRELLIS_FINISH_WORK_BACKUP_PATHS: [&str; 2] = [
    ".agents/skills/trellis-finish-work/SKILL.md.backup",
    ".agents/skills/finish-work/SKILL.md.backup",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct IntegrationProviderDetection {
    detected: bool,
    evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct IntegrationStatusSummary {
    providers: Vec<IntegrationProviderPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct IntegrationProviderPlan {
    provider: String,
    detected: bool,
    evidence: Vec<String>,
    bootstrap_policy: String,
    capabilities: Vec<IntegrationCapabilityPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct IntegrationCapabilityPlan {
    provider: String,
    capability: String,
    selected: bool,
    status: String,
    target_paths: Vec<String>,
    failure: Option<IntegrationFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IntegrationCapabilityProbe {
    installed: bool,
    failure: Option<IntegrationFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct IntegrationApplySummary {
    before: IntegrationStatusSummary,
    results: Vec<IntegrationApplyCapabilityResult>,
    succeeded: usize,
    failed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct IntegrationApplyCapabilityResult {
    provider: String,
    capability: String,
    status: String,
    target_paths: Vec<String>,
    failure: Option<IntegrationFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntegrationTargetBehavior {
    FullFileOrManagedBlock,
    DedicatedFile,
    ManagedBlockOnly,
    TrellisFinalizeBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IntegrationTarget {
    source_rel: &'static str,
    host_rel: &'static str,
    behavior: IntegrationTargetBehavior,
}

pub fn run(cli: Cli) -> Result<AppOutput, OmvError> {
    let ntp = NtpTimeSource::default();
    let system = SystemTimeSource;
    let runtime = AppRuntime {
        ntp_source: &ntp,
        system_source: &system,
    };
    run_with_runtime(cli, &runtime)
}

pub fn run_with_runtime(cli: Cli, runtime: &AppRuntime<'_>) -> Result<AppOutput, OmvError> {
    let cwd = std::env::current_dir()?;
    let omv_root = storage::resolve_omv_root(&cwd)?;

    let locale = resolve_locale_for_root(&omv_root, cli.locale_override.as_deref())?;
    let catalog = i18n::load_catalog(&locale)?;
    let ntp_override = cli.ntp_override;

    match cli.command {
        Command::Init => run_init(&omv_root, &catalog, &locale, cli.output_mode),
        Command::Bump => run_bump(&omv_root, &catalog, runtime, ntp_override, cli.output_mode),
        Command::Plan => run_plan(&omv_root, &catalog, cli.output_mode),
        Command::Sync(command) => run_sync(&omv_root, &catalog, cli.output_mode, command.check),
        Command::Current => run_current(&omv_root, &catalog, cli.output_mode),
        Command::Event(event_command) => run_event(
            &omv_root,
            &catalog,
            runtime,
            ntp_override,
            cli.output_mode,
            event_command,
        ),
        Command::Adapter(adapter_command) => {
            run_adapter(&cwd, &omv_root, &catalog, cli.output_mode, adapter_command)
        }
        Command::Integrate(integrate_command) => run_integrate(
            &cwd,
            &omv_root,
            &catalog,
            cli.output_mode,
            integrate_command,
        ),
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
        format!("  {}", catalog.t("cli.help.commands.plan")),
        format!("  {}", catalog.t("cli.help.commands.sync")),
        format!("  {}", catalog.t("cli.help.commands.current")),
        format!("  {}", catalog.t("cli.help.commands.event")),
        format!("  {}", catalog.t("cli.help.commands.adapter")),
        format!("  {}", catalog.t("cli.help.commands.integrate")),
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
        format!("  {}", catalog.t("cli.help.examples.plan")),
        format!("  {}", catalog.t("cli.help.examples.sync")),
        format!("  {}", catalog.t("cli.help.examples.current")),
        format!("  {}", catalog.t("cli.help.examples.event")),
        format!("  {}", catalog.t("cli.help.examples.adapter")),
        format!("  {}", catalog.t("cli.help.examples.integrate")),
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
            OmvError::Finalization(_) => cat.tf("error.finalization", &["detail", detail.as_str()]),
            OmvError::Integration(_) => cat.tf("error.integration", &["detail", detail.as_str()]),
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
        contract_version: STRUCTURED_JSON_CONTRACT_VERSION,
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

    let outcome = persist_init_state(omv_root, &draft)?;

    let message = match output_mode {
        OutputMode::Text => render_init_result_text(catalog, &outcome),
        OutputMode::Json => render_structured_success(
            "init",
            serde_json::json!({
                "saved": true,
                "omv_root": omv_root.display().to_string(),
                "integrations": outcome.integrations
            }),
        ),
    };

    Ok(AppOutput { message })
}

fn render_init_result_text(catalog: &Catalog, outcome: &InitPersistenceOutcome) -> String {
    match outcome.integrations.status {
        InitIntegrationApplyStatus::NoSelection => catalog.t("init.result.saved"),
        InitIntegrationApplyStatus::Applied => {
            let count = outcome.integrations.selected_capabilities.to_string();
            catalog.tf(
                "init.result.saved_integrations_applied",
                &["count", count.as_str()],
            )
        }
        InitIntegrationApplyStatus::Deferred => {
            let (reason, detail) = outcome
                .integrations
                .reason
                .split_once(':')
                .unwrap_or((outcome.integrations.reason.as_str(), ""));
            match reason {
                "unsupported-capability" => catalog.tf(
                    "init.result.saved_integrations_deferred_unsupported",
                    &["detail", detail],
                ),
                "unsafe-worktree" => catalog.tf(
                    "init.result.saved_integrations_deferred_unsafe",
                    &["detail", detail],
                ),
                _ => catalog.t("init.result.saved_integrations_deferred"),
            }
        }
    }
}

fn run_bump(
    omv_root: &Path,
    catalog: &Catalog,
    runtime: &AppRuntime<'_>,
    ntp_override: Option<bool>,
    output_mode: OutputMode,
) -> Result<AppOutput, OmvError> {
    let execution = execute_bump(
        omv_root,
        runtime.ntp_source,
        runtime.system_source,
        ntp_override,
    )?;

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
    check: bool,
) -> Result<AppOutput, OmvError> {
    if check {
        return run_sync_check(omv_root, catalog, output_mode);
    }

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

fn run_plan(
    omv_root: &Path,
    catalog: &Catalog,
    output_mode: OutputMode,
) -> Result<AppOutput, OmvError> {
    let plan = execute_plan(omv_root)?;
    let message = match output_mode {
        OutputMode::Text => render_plan_text(catalog, &plan),
        OutputMode::Json => render_structured_success("plan", &plan),
    };

    Ok(AppOutput { message })
}

fn run_sync_check(
    omv_root: &Path,
    catalog: &Catalog,
    output_mode: OutputMode,
) -> Result<AppOutput, OmvError> {
    let plan = execute_plan(omv_root)?;
    if plan.has_required_drift() {
        let plan_value = serde_json::to_value(&plan).expect("plan should serialize");
        return Err(TargetError::CheckFailed {
            reason: String::from("required target drift or missing target detected"),
            plan: Box::new(plan_value),
        }
        .into());
    }

    let message = match output_mode {
        OutputMode::Text => {
            let targets = plan.targets.len().to_string();
            catalog.tf(
                "cli.sync.check.success",
                &[
                    "version",
                    plan.version.as_str(),
                    "targets",
                    targets.as_str(),
                ],
            )
        }
        OutputMode::Json => render_structured_success("sync.check", &plan),
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

fn run_event(
    omv_root: &Path,
    catalog: &Catalog,
    runtime: &AppRuntime<'_>,
    ntp_override: Option<bool>,
    output_mode: OutputMode,
    command: EventCommand,
) -> Result<AppOutput, OmvError> {
    let message = match command.action {
        EventAction::FinalizeTask(finalize_command) => {
            let execution = execute_finalize_task(
                omv_root,
                runtime.ntp_source,
                runtime.system_source,
                ntp_override,
                finalize_command,
            )?;
            match output_mode {
                OutputMode::Text => render_finalize_task_text(catalog, &execution),
                OutputMode::Json => render_structured_success("event.finalize-task", &execution),
            }
        }
        EventAction::FinalizeBoundary(finalize_command) => {
            let execution = execute_finalize_boundary(
                omv_root,
                runtime.ntp_source,
                runtime.system_source,
                ntp_override,
                finalize_command,
            )?;
            match output_mode {
                OutputMode::Text => render_finalize_boundary_text(catalog, &execution),
                OutputMode::Json => {
                    render_structured_success("event.finalize-boundary", &execution)
                }
            }
        }
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

fn run_integrate(
    cwd: &Path,
    omv_root: &Path,
    catalog: &Catalog,
    output_mode: OutputMode,
    command: IntegrateCommand,
) -> Result<AppOutput, OmvError> {
    let project_root = storage::resolve_project_root(cwd)?;
    let message = match command.action {
        IntegrateAction::Status => {
            let status = execute_integrate_status(omv_root, &project_root)?;
            match output_mode {
                OutputMode::Text => render_integrate_status_text(catalog, &status),
                OutputMode::Json => render_structured_success("integrate.status", &status),
            }
        }
        IntegrateAction::Apply => {
            let apply = execute_integrate_apply(omv_root, &project_root)?;
            match output_mode {
                OutputMode::Text => render_integrate_apply_text(catalog, &apply),
                OutputMode::Json => render_structured_success("integrate.apply", &apply),
            }
        }
    };

    Ok(AppOutput { message })
}

fn render_finalize_task_text(catalog: &Catalog, execution: &FinalizeTaskExecution) -> String {
    if execution.duplicate {
        return catalog.tf(
            "cli.event.finalize_task.duplicate",
            &[
                "task_id",
                execution.task_id.as_str(),
                "version",
                execution.version_after.as_str(),
            ],
        );
    }

    if execution.recovered {
        return catalog.tf(
            "cli.event.finalize_task.recovered",
            &[
                "task_id",
                execution.task_id.as_str(),
                "version",
                execution.version_after.as_str(),
            ],
        );
    }

    if execution.outcome == FinalizationOutcome::Bumped.as_str() {
        return catalog.tf(
            "cli.event.finalize_task.bumped",
            &[
                "task_id",
                execution.task_id.as_str(),
                "version",
                execution.version_after.as_str(),
            ],
        );
    }

    let reason = finalization_reason_text(catalog, execution.reason.as_str());
    catalog.tf(
        "cli.event.finalize_task.noop",
        &[
            "task_id",
            execution.task_id.as_str(),
            "reason",
            reason.as_str(),
        ],
    )
}

fn render_finalize_boundary_text(
    catalog: &Catalog,
    execution: &FinalizeBoundaryExecution,
) -> String {
    if execution.manual_action_required {
        return catalog.tf(
            "cli.event.finalize_boundary.pending_change_type",
            &[
                "task_id",
                execution.task_id.as_str(),
                "provider",
                execution.provider.as_str(),
                "boundary",
                execution.boundary.as_str(),
            ],
        );
    }

    if let Some(finalize_task) = &execution.finalize_task {
        return render_finalize_task_text(catalog, finalize_task);
    }

    catalog.tf(
        "cli.event.finalize_boundary.pending_change_type",
        &[
            "task_id",
            execution.task_id.as_str(),
            "provider",
            execution.provider.as_str(),
            "boundary",
            execution.boundary.as_str(),
        ],
    )
}

fn finalization_reason_text(catalog: &Catalog, reason: &str) -> String {
    let key = match reason {
        "semantic-change" => "cli.event.finalize_task.reason.semantic_change",
        "tests-not-passed" => "cli.event.finalize_task.reason.tests_not_passed",
        "status-not-done" => "cli.event.finalize_task.reason.status_not_done",
        "non-semantic-change" => "cli.event.finalize_task.reason.non_semantic_change",
        "duplicate-fingerprint" => "cli.event.finalize_task.reason.duplicate_fingerprint",
        "pending-recovered" => "cli.event.finalize_task.reason.pending_recovered",
        _ => return reason.to_owned(),
    };

    catalog.t(key)
}

fn render_structured_success<T: Serialize>(command: &str, data: T) -> String {
    let envelope = StructuredEnvelope {
        ok: true,
        contract_version: STRUCTURED_JSON_CONTRACT_VERSION,
        command: command.to_owned(),
        data: Some(data),
        error: None,
    };
    serde_json::to_string_pretty(&envelope).expect("structured success should serialize")
}

fn render_plan_text(catalog: &Catalog, plan: &crate::sync::PlanSummary) -> String {
    let mut lines = vec![catalog.tf(
        "cli.plan.header",
        &[
            "version",
            plan.version.as_str(),
            "status",
            plan.project_status.as_str(),
        ],
    )];

    for target in &plan.targets {
        let paths = target.paths.join(", ");
        lines.push(catalog.tf(
            "cli.plan.target",
            &[
                "id",
                target.id.as_str(),
                "status",
                target.status.as_str(),
                "paths",
                paths.as_str(),
            ],
        ));
        for diagnostic in &target.diagnostics {
            lines.push(catalog.tf("cli.plan.diagnostic", &["detail", diagnostic.as_str()]));
        }
    }

    let ok = plan.totals.ok.to_string();
    let drift = plan.totals.drift.to_string();
    let missing = plan.totals.missing.to_string();
    let skipped = plan.totals.skipped.to_string();
    lines.push(catalog.tf(
        "cli.plan.totals",
        &[
            "ok",
            ok.as_str(),
            "drift",
            drift.as_str(),
            "missing",
            missing.as_str(),
            "skipped",
            skipped.as_str(),
        ],
    ));

    lines.join("\n")
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

fn render_integrate_status_text(catalog: &Catalog, status: &IntegrationStatusSummary) -> String {
    let mut lines = vec![catalog.t("cli.integrate.status.header")];
    for provider in &status.providers {
        lines.push(catalog.tf(
            "cli.integrate.status.provider",
            &[
                "provider",
                provider.provider.as_str(),
                "detected",
                bool_text(provider.detected),
            ],
        ));
        for capability in &provider.capabilities {
            lines.push(catalog.tf(
                "cli.integrate.status.capability",
                &[
                    "capability",
                    capability.capability.as_str(),
                    "status",
                    capability.status.as_str(),
                    "selected",
                    bool_text(capability.selected),
                ],
            ));
        }
    }
    lines.join("\n")
}

fn render_integrate_apply_text(catalog: &Catalog, apply: &IntegrationApplySummary) -> String {
    let succeeded = apply.succeeded.to_string();
    let failed = apply.failed.to_string();
    let mut lines = vec![catalog.tf(
        "cli.integrate.apply.summary",
        &["succeeded", succeeded.as_str(), "failed", failed.as_str()],
    )];

    for result in &apply.results {
        lines.push(catalog.tf(
            "cli.integrate.apply.item",
            &[
                "provider",
                result.provider.as_str(),
                "capability",
                result.capability.as_str(),
                "status",
                result.status.as_str(),
            ],
        ));
        if let Some(failure) = &result.failure {
            lines.push(catalog.tf(
                "cli.integrate.apply.failure",
                &[
                    "code",
                    failure.reason_code.as_str(),
                    "message",
                    failure.display_message.as_str(),
                ],
            ));
        }
    }

    lines.join("\n")
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn execute_integrate_status(
    omv_root: &Path,
    project_root: &Path,
) -> Result<IntegrationStatusSummary, OmvError> {
    adapter::ensure_canonical_artifacts(omv_root)?;
    let state = load_integration_state(omv_root)?;
    build_integration_status(project_root, &state)
}

fn execute_integrate_apply(
    omv_root: &Path,
    project_root: &Path,
) -> Result<IntegrationApplySummary, OmvError> {
    adapter::ensure_canonical_artifacts(omv_root)?;
    let mut state = load_integration_state(omv_root)?;
    let before = build_integration_status(project_root, &state)?;
    update_state_from_status(&mut state, &before);
    save_integration_state(omv_root, &state)?;

    let mut results = Vec::new();
    for plan in selected_integration_capabilities(&before) {
        let result = apply_integration_capability(omv_root, project_root, &plan);
        update_state_capability(&mut state, &result);
        save_integration_state(omv_root, &state)?;
        results.push(result);
    }

    let succeeded = results
        .iter()
        .filter(|result| result.status == "installed")
        .count();
    let failed = results
        .iter()
        .filter(|result| result.status == "failed")
        .count();
    let summary = IntegrationApplySummary {
        before,
        results,
        succeeded,
        failed,
    };

    if failed > 0 {
        let result = serde_json::to_value(&summary).expect("integration summary should serialize");
        return Err(IntegrationError::ApplyFailed {
            reason: String::from("one or more selected integration capabilities failed"),
            result: Box::new(result),
        }
        .into());
    }

    Ok(summary)
}

fn selected_integration_capabilities(
    status: &IntegrationStatusSummary,
) -> Vec<IntegrationCapabilityPlan> {
    status
        .providers
        .iter()
        .flat_map(|provider| provider.capabilities.iter())
        .filter(|capability| capability.selected && capability.status != "failed")
        .cloned()
        .collect()
}

fn apply_integration_capability(
    omv_root: &Path,
    project_root: &Path,
    plan: &IntegrationCapabilityPlan,
) -> IntegrationApplyCapabilityResult {
    let Some(provider) = IntegrationProvider::parse(&plan.provider) else {
        return failed_integration_result(plan, "unknown-provider", "unknown integration provider");
    };
    let Some(capability) = IntegrationCapability::parse(&plan.capability) else {
        return failed_integration_result(
            plan,
            "unknown-capability",
            "unknown integration capability",
        );
    };

    if provider == IntegrationProvider::Trellis
        && !detect_integration_provider(project_root, provider).detected
    {
        return failed_integration_result(
            plan,
            "provider-not-detected",
            "trellis integration requires an existing .trellis installation",
        );
    }

    let Some(target) = integration_target(provider, capability) else {
        return failed_integration_result(
            plan,
            "unsupported-capability",
            "capability is not installable in this worker scope",
        );
    };
    let target = resolve_integration_target(project_root, target);

    if let Err(err) = check_integration_target_safety(project_root, &target) {
        let message = err.to_string();
        return failed_integration_result(plan, "unsafe-worktree", message.as_str());
    }

    match install_integration_target(omv_root, project_root, &target, plan) {
        Ok(mode) => {
            if let Err(err) = record_adapter_target(omv_root, &target, plan, mode) {
                let message = err.to_string();
                return failed_integration_result(plan, "registry-write-failed", message.as_str());
            }
            IntegrationApplyCapabilityResult {
                provider: plan.provider.clone(),
                capability: plan.capability.clone(),
                status: String::from("installed"),
                target_paths: plan.target_paths.clone(),
                failure: None,
            }
        }
        Err(err) => {
            let message = err.to_string();
            failed_integration_result(plan, "install-failed", message.as_str())
        }
    }
}

fn failed_integration_result(
    plan: &IntegrationCapabilityPlan,
    code: &str,
    message: &str,
) -> IntegrationApplyCapabilityResult {
    IntegrationApplyCapabilityResult {
        provider: plan.provider.clone(),
        capability: plan.capability.clone(),
        status: String::from("failed"),
        target_paths: plan.target_paths.clone(),
        failure: Some(IntegrationFailure {
            reason_code: code.to_owned(),
            display_message: message.to_owned(),
        }),
    }
}

fn load_integration_state(omv_root: &Path) -> Result<OmvIntegrations, OmvError> {
    storage::integrations::load_integrations(omv_root).map(normalize_integration_state)
}

fn save_integration_state(omv_root: &Path, state: &OmvIntegrations) -> Result<(), OmvError> {
    storage::integrations::save_integrations(omv_root, state)
}

fn normalize_integration_state(mut state: OmvIntegrations) -> OmvIntegrations {
    state.schema_version = 1;
    for descriptor in mvp_provider_descriptors() {
        let default_selected = default_integration_selected(&descriptor);
        if !state
            .providers
            .iter()
            .any(|existing| existing.provider == descriptor.provider)
        {
            state.providers.push(OmvIntegrationProviderState {
                provider: descriptor.provider,
                selected: default_selected,
                detection: IntegrationDetectionSnapshot {
                    detected: false,
                    recommended: descriptor.capabilities.iter().any(|cap| cap.recommended),
                },
                capabilities: Vec::new(),
            });
        }

        let provider_state = state
            .providers
            .iter_mut()
            .find(|existing| existing.provider == descriptor.provider)
            .expect("provider was inserted");
        if provider_state.capabilities.is_empty() {
            provider_state.selected = default_selected;
        }

        for capability in &descriptor.capabilities {
            if !provider_state
                .capabilities
                .iter()
                .any(|existing| existing.capability == capability.capability)
            {
                provider_state
                    .capabilities
                    .push(OmvIntegrationCapabilityState {
                        capability: capability.capability,
                        selected: default_selected,
                        status: if default_selected {
                            IntegrationCapabilityStatus::Pending
                        } else {
                            IntegrationCapabilityStatus::Selected
                        },
                        failure: None,
                    });
            }
        }
    }
    state
}

fn build_integration_status(
    project_root: &Path,
    state: &OmvIntegrations,
) -> Result<IntegrationStatusSummary, OmvError> {
    let providers = mvp_provider_descriptors()
        .into_iter()
        .map(|descriptor| {
            let state_provider = state
                .providers
                .iter()
                .find(|candidate| candidate.provider == descriptor.provider);
            let detection = detect_integration_provider(project_root, descriptor.provider);
            let capabilities = descriptor
                .capabilities
                .iter()
                .map(|capability_descriptor| {
                    let state_capability = state_provider.and_then(|candidate| {
                        candidate
                            .capabilities
                            .iter()
                            .find(|item| item.capability == capability_descriptor.capability)
                    });
                    let selected = state_capability
                        .map(|candidate| candidate.selected)
                        .unwrap_or(default_capability_selected(
                            &descriptor,
                            capability_descriptor,
                        ));
                    let probe = probe_integration_capability(
                        project_root,
                        descriptor.provider,
                        capability_descriptor.capability,
                    );
                    let status = if probe.installed && probe.failure.is_none() {
                        IntegrationCapabilityStatus::Installed.as_str().to_owned()
                    } else if probe.failure.is_some() && selected {
                        IntegrationCapabilityStatus::Pending.as_str().to_owned()
                    } else if state_capability
                        .and_then(|candidate| candidate.failure.as_ref())
                        .is_some()
                        && selected
                    {
                        IntegrationCapabilityStatus::Failed.as_str().to_owned()
                    } else if selected {
                        IntegrationCapabilityStatus::Pending.as_str().to_owned()
                    } else {
                        IntegrationCapabilityStatus::Selected.as_str().to_owned()
                    };
                    IntegrationCapabilityPlan {
                        provider: descriptor.provider.as_str().to_owned(),
                        capability: capability_descriptor.capability.as_str().to_owned(),
                        selected,
                        status,
                        target_paths: capability_descriptor.target_paths.clone(),
                        failure: probe.failure.or_else(|| {
                            state_capability.and_then(|candidate| candidate.failure.clone())
                        }),
                    }
                })
                .collect();

            IntegrationProviderPlan {
                provider: descriptor.provider.as_str().to_owned(),
                detected: detection.detected,
                evidence: detection.evidence,
                bootstrap_policy: descriptor.bootstrap_policy.as_str().to_owned(),
                capabilities,
            }
        })
        .collect();

    Ok(IntegrationStatusSummary { providers })
}

fn default_integration_selected(descriptor: &IntegrationProviderDescriptor) -> bool {
    descriptor.provider == IntegrationProvider::Codex
}

fn default_capability_selected(
    provider: &IntegrationProviderDescriptor,
    capability: &IntegrationCapabilityDescriptor,
) -> bool {
    default_integration_selected(provider) && capability.default_selected
}

fn update_state_from_status(state: &mut OmvIntegrations, status: &IntegrationStatusSummary) {
    for provider in &status.providers {
        let Some(provider_id) = IntegrationProvider::parse(&provider.provider) else {
            continue;
        };
        if let Some(state_provider) = state
            .providers
            .iter_mut()
            .find(|item| item.provider == provider_id)
        {
            state_provider.detection = IntegrationDetectionSnapshot {
                detected: provider.detected,
                recommended: provider.detected,
            };
            for capability in &provider.capabilities {
                let Some(capability_id) = IntegrationCapability::parse(&capability.capability)
                else {
                    continue;
                };
                if let Some(state_capability) = state_provider
                    .capabilities
                    .iter_mut()
                    .find(|item| item.capability == capability_id)
                {
                    state_capability.status =
                        parse_integration_status(&capability.status, state_capability.status);
                    state_capability.failure = capability.failure.clone();
                }
            }
        }
    }
}

fn update_state_capability(state: &mut OmvIntegrations, result: &IntegrationApplyCapabilityResult) {
    let Some(provider_id) = IntegrationProvider::parse(&result.provider) else {
        return;
    };
    let Some(capability_id) = IntegrationCapability::parse(&result.capability) else {
        return;
    };
    let Some(provider) = state
        .providers
        .iter_mut()
        .find(|item| item.provider == provider_id)
    else {
        return;
    };
    let Some(capability) = provider
        .capabilities
        .iter_mut()
        .find(|item| item.capability == capability_id)
    else {
        return;
    };
    capability.status = parse_integration_status(&result.status, capability.status);
    capability.failure = result.failure.clone();
}

fn parse_integration_status(
    value: &str,
    fallback: IntegrationCapabilityStatus,
) -> IntegrationCapabilityStatus {
    match value {
        "selected" => IntegrationCapabilityStatus::Selected,
        "pending" => IntegrationCapabilityStatus::Pending,
        "installed" => IntegrationCapabilityStatus::Installed,
        "failed" => IntegrationCapabilityStatus::Failed,
        _ => fallback,
    }
}

fn detect_integration_provider(
    project_root: &Path,
    provider: IntegrationProvider,
) -> IntegrationProviderDetection {
    match provider {
        IntegrationProvider::Codex => {
            let mut evidence = Vec::new();
            if project_root.join("AGENTS.md").exists() {
                evidence.push(String::from("AGENTS.md"));
            }
            if project_root.join(".codex").exists() {
                evidence.push(String::from(".codex"));
            }
            IntegrationProviderDetection {
                detected: !evidence.is_empty(),
                evidence,
            }
        }
        IntegrationProvider::Trellis => {
            let mut evidence = Vec::new();
            if project_root.join(".trellis").exists() {
                evidence.push(String::from(".trellis"));
            }
            if project_root.join(".trellis/spec/guides/index.md").exists() {
                evidence.push(String::from(".trellis/spec/guides/index.md"));
            }
            IntegrationProviderDetection {
                detected: !evidence.is_empty(),
                evidence,
            }
        }
        IntegrationProvider::OpenCode => {
            let mut evidence = Vec::new();
            if project_root.join(".opencode").exists() {
                evidence.push(String::from(".opencode"));
            }
            if project_root.join("AGENTS.md").exists() {
                evidence.push(String::from("AGENTS.md"));
            }
            IntegrationProviderDetection {
                detected: !evidence.is_empty(),
                evidence,
            }
        }
    }
}

fn integration_target(
    provider: IntegrationProvider,
    capability: IntegrationCapability,
) -> Option<IntegrationTarget> {
    match (provider, capability) {
        (IntegrationProvider::Codex, IntegrationCapability::ProjectInstructions) => {
            Some(IntegrationTarget {
                source_rel: "adapters/project-instructions.md",
                host_rel: "AGENTS.md",
                behavior: IntegrationTargetBehavior::FullFileOrManagedBlock,
            })
        }
        (IntegrationProvider::Codex, IntegrationCapability::HostSkill) => Some(IntegrationTarget {
            source_rel: "adapters/codex/SKILL.md",
            host_rel: ".codex/skills/omv-versioning/SKILL.md",
            behavior: IntegrationTargetBehavior::DedicatedFile,
        }),
        (IntegrationProvider::OpenCode, IntegrationCapability::ProjectInstructions) => {
            Some(IntegrationTarget {
                source_rel: "adapters/project-instructions.md",
                host_rel: "AGENTS.md",
                behavior: IntegrationTargetBehavior::FullFileOrManagedBlock,
            })
        }
        (IntegrationProvider::OpenCode, IntegrationCapability::HostSkill) => {
            Some(IntegrationTarget {
                source_rel: "adapters/opencode/SKILL.md",
                host_rel: ".opencode/skills/omv-versioning/SKILL.md",
                behavior: IntegrationTargetBehavior::DedicatedFile,
            })
        }
        (IntegrationProvider::Trellis, IntegrationCapability::SpecGuide) => {
            Some(IntegrationTarget {
                source_rel: "adapters/trellis/guide.md",
                host_rel: ".trellis/spec/guides/omv-versioning-guide.md",
                behavior: IntegrationTargetBehavior::DedicatedFile,
            })
        }
        (IntegrationProvider::Trellis, IntegrationCapability::SpecIndexSnippet) => {
            Some(IntegrationTarget {
                source_rel: "adapters/trellis/index-snippet.md",
                host_rel: ".trellis/spec/guides/index.md",
                behavior: IntegrationTargetBehavior::ManagedBlockOnly,
            })
        }
        (IntegrationProvider::Trellis, IntegrationCapability::FinalizeBoundary) => {
            Some(IntegrationTarget {
                source_rel: "contract.json",
                host_rel: TRELLIS_FINISH_WORK_V05_PATH,
                behavior: IntegrationTargetBehavior::TrellisFinalizeBoundary,
            })
        }
        _ => None,
    }
}

fn resolve_integration_target(project_root: &Path, target: IntegrationTarget) -> IntegrationTarget {
    if target.behavior != IntegrationTargetBehavior::TrellisFinalizeBoundary {
        return target;
    }

    IntegrationTarget {
        host_rel: resolve_trellis_finish_work_path(project_root).unwrap_or(target.host_rel),
        ..target
    }
}

fn resolve_trellis_finish_work_path(project_root: &Path) -> Option<&'static str> {
    let v05 = project_root.join(TRELLIS_FINISH_WORK_V05_PATH);
    let v04 = project_root.join(TRELLIS_FINISH_WORK_V04_PATH);

    if file_contains_trellis_finalize_block(&v05) {
        return Some(TRELLIS_FINISH_WORK_V05_PATH);
    }
    if file_contains_trellis_finalize_block(&v04) && !v05.exists() {
        return Some(TRELLIS_FINISH_WORK_V04_PATH);
    }
    if v05.exists() {
        return Some(TRELLIS_FINISH_WORK_V05_PATH);
    }
    if v04.exists() {
        return Some(TRELLIS_FINISH_WORK_V04_PATH);
    }
    None
}

fn probe_integration_capability(
    project_root: &Path,
    provider: IntegrationProvider,
    capability: IntegrationCapability,
) -> IntegrationCapabilityProbe {
    let Some(target) = integration_target(provider, capability) else {
        return IntegrationCapabilityProbe {
            installed: false,
            failure: None,
        };
    };
    if target.behavior == IntegrationTargetBehavior::TrellisFinalizeBoundary {
        return probe_trellis_finalize_boundary(project_root);
    }

    let host_path = project_root.join(target.host_rel);
    let Ok(content) = fs::read_to_string(host_path) else {
        return IntegrationCapabilityProbe {
            installed: false,
            failure: None,
        };
    };
    IntegrationCapabilityProbe {
        installed: is_omv_managed_integration_content(&content),
        failure: None,
    }
}

fn probe_trellis_finalize_boundary(project_root: &Path) -> IntegrationCapabilityProbe {
    let v05_path = project_root.join(TRELLIS_FINISH_WORK_V05_PATH);
    let v04_path = project_root.join(TRELLIS_FINISH_WORK_V04_PATH);
    let v05_exists = v05_path.exists();
    let v04_has_block = file_contains_trellis_finalize_block(&v04_path);
    let v05_has_block = file_contains_trellis_finalize_block(&v05_path);

    if v05_has_block {
        return IntegrationCapabilityProbe {
            installed: true,
            failure: None,
        };
    }

    if v04_has_block && v05_exists {
        return trellis_finalize_boundary_mismatch(
            TRELLIS_FINISH_WORK_V04_PATH,
            "Trellis may now run the new skill path",
        );
    }

    if let Some(backup_path) = trellis_finalize_backup_with_block(project_root) {
        return trellis_finalize_boundary_mismatch(
            backup_path,
            "Trellis update may have moved the previous OMV guidance into a backup file",
        );
    }

    IntegrationCapabilityProbe {
        installed: v04_has_block,
        failure: None,
    }
}

fn trellis_finalize_boundary_mismatch(
    source_path: &str,
    reason: &str,
) -> IntegrationCapabilityProbe {
    IntegrationCapabilityProbe {
        installed: false,
        failure: Some(IntegrationFailure {
            reason_code: String::from("trellis-finish-work-path-mismatch"),
            display_message: format!(
                "OMV finalize-boundary guidance is installed only in {source_path}; {reason}. Run `omv integrate apply` to refresh the active Trellis finish-work surface."
            ),
        }),
    }
}

fn trellis_finalize_backup_with_block(project_root: &Path) -> Option<&'static str> {
    TRELLIS_FINISH_WORK_BACKUP_PATHS
        .iter()
        .copied()
        .find(|path| file_contains_trellis_finalize_block(&project_root.join(path)))
}

fn file_contains_trellis_finalize_block(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|content| content.contains(adapter::TRELLIS_FINISH_WORK_BLOCK_NAME))
        .unwrap_or(false)
}

fn check_integration_target_safety(
    project_root: &Path,
    target: &IntegrationTarget,
) -> Result<(), OmvError> {
    let host_path = project_root.join(target.host_rel);
    if !host_path.exists() {
        return Ok(());
    }

    match target.behavior {
        IntegrationTargetBehavior::DedicatedFile => {
            let content = fs::read_to_string(&host_path).unwrap_or_default();
            if is_omv_managed_integration_content(&content) {
                Ok(())
            } else {
                Err(AdapterError::Conflict {
                    path: host_path,
                    reason: String::from("existing integration target is not OMV-managed"),
                }
                .into())
            }
        }
        IntegrationTargetBehavior::FullFileOrManagedBlock
        | IntegrationTargetBehavior::ManagedBlockOnly
        | IntegrationTargetBehavior::TrellisFinalizeBoundary => Ok(()),
    }
}

fn install_integration_target(
    omv_root: &Path,
    project_root: &Path,
    target: &IntegrationTarget,
    plan: &IntegrationCapabilityPlan,
) -> Result<crate::core::adapter::AdapterTargetMode, OmvError> {
    let host_path = project_root.join(target.host_rel);

    match target.behavior {
        IntegrationTargetBehavior::TrellisFinalizeBoundary => {
            if !host_path.exists() {
                return Err(AdapterError::Unsupported {
                    reason: format!(
                        "no Trellis finish-work skill surface found; expected one of {}. Run Trellis update/init, then rerun `omv integrate apply`.",
                        TRELLIS_FINISH_WORK_PATHS.join(", ")
                    ),
                }
                .into());
            }
            let existing = fs::read_to_string(&host_path)?;
            let content = adapter::upsert_trellis_finish_work_finalize_block(&existing);
            storage::atomic::write_atomically(&host_path, content.as_bytes())?;
            Ok(crate::core::adapter::AdapterTargetMode::ManagedBlock)
        }
        IntegrationTargetBehavior::ManagedBlockOnly => {
            let source_path = omv_root.join(adapter::AI_DIR).join(target.source_rel);
            let rendered = fs::read_to_string(source_path)?;
            write_integration_managed_block(&host_path, plan, &rendered)?;
            Ok(crate::core::adapter::AdapterTargetMode::ManagedBlock)
        }
        IntegrationTargetBehavior::DedicatedFile => {
            let source_path = omv_root.join(adapter::AI_DIR).join(target.source_rel);
            let rendered = fs::read_to_string(source_path)?;
            write_integration_managed_file(&host_path, target.source_rel, &rendered)?;
            Ok(crate::core::adapter::AdapterTargetMode::Materialize)
        }
        IntegrationTargetBehavior::FullFileOrManagedBlock => {
            let source_path = omv_root.join(adapter::AI_DIR).join(target.source_rel);
            let rendered = fs::read_to_string(source_path)?;
            if host_path.exists() {
                let content = fs::read_to_string(&host_path).unwrap_or_default();
                if is_omv_managed_integration_file(&content) {
                    write_integration_managed_file(&host_path, target.source_rel, &rendered)?;
                    Ok(crate::core::adapter::AdapterTargetMode::Materialize)
                } else {
                    write_integration_managed_block(&host_path, plan, &rendered)?;
                    Ok(crate::core::adapter::AdapterTargetMode::ManagedBlock)
                }
            } else {
                write_integration_managed_file(&host_path, target.source_rel, &rendered)?;
                Ok(crate::core::adapter::AdapterTargetMode::Materialize)
            }
        }
    }
}

fn write_integration_managed_file(
    host_path: &Path,
    source_rel: &str,
    rendered: &str,
) -> Result<(), OmvError> {
    let content = adapter::wrap_managed_file(source_rel, rendered);
    storage::atomic::write_atomically(host_path, content.as_bytes())
}

fn write_integration_managed_block(
    host_path: &Path,
    plan: &IntegrationCapabilityPlan,
    rendered: &str,
) -> Result<(), OmvError> {
    let block_name = if plan.capability == "project-instructions" {
        String::from("integration-project-instructions")
    } else {
        format!("integration-{}-{}", plan.provider, plan.capability)
    };
    let begin = format!("{MANAGED_BEGIN_PREFIX}{block_name} -->");
    let end = format!("{MANAGED_END_PREFIX}{block_name} -->");
    let block = format!("{begin}\n{rendered}\n{end}\n");
    let content = match fs::read_to_string(host_path) {
        Ok(existing) => replace_or_append_integration_block(&existing, &begin, &end, &block),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => block,
        Err(err) => return Err(err.into()),
    };
    storage::atomic::write_atomically(host_path, content.as_bytes())?;

    // Migration: remove old provider-prefixed managed block
    if plan.capability == "project-instructions" {
        let old_codex_block = "integration-codex-project-instructions";
        let old_begin = format!("{MANAGED_BEGIN_PREFIX}{old_codex_block} -->");
        let old_end = format!("{MANAGED_END_PREFIX}{old_codex_block} -->");
        if let Ok(existing) = fs::read_to_string(host_path)
            && existing.find(&old_begin).is_some()
        {
            let cleaned = remove_managed_block_by_marker(&existing, &old_begin, &old_end);
            storage::atomic::write_atomically(host_path, cleaned.as_bytes())?;
        }
    }

    Ok(())
}

fn remove_managed_block_by_marker(existing: &str, begin: &str, end: &str) -> String {
    if let Some(start) = existing.find(begin)
        && let Some(end_idx) = existing[start..].find(end)
    {
        let absolute_end = start + end_idx + end.len();
        let mut output = String::new();
        output.push_str(existing[..start].trim_end());
        if absolute_end < existing.len() {
            output.push_str("\n\n");
            output.push_str(existing[absolute_end..].trim_start_matches('\n'));
        }
        if !output.ends_with('\n') {
            output.push('\n');
        }
        return output;
    }
    existing.to_owned()
}

fn replace_or_append_integration_block(
    existing: &str,
    begin: &str,
    end: &str,
    replacement: &str,
) -> String {
    if let Some(start) = existing.find(begin)
        && let Some(end_idx) = existing[start..].find(end)
    {
        let absolute_end = start + end_idx + end.len();
        let mut output = String::with_capacity(existing.len() + replacement.len());
        output.push_str(&existing[..start]);
        if !output.ends_with('\n') && !output.is_empty() {
            output.push('\n');
        }
        output.push_str(replacement);
        if absolute_end < existing.len() {
            let tail = &existing[absolute_end..];
            if !tail.starts_with('\n') && !tail.is_empty() {
                output.push('\n');
            }
            output.push_str(tail.trim_start_matches('\n'));
        }
        if !output.ends_with('\n') {
            output.push('\n');
        }
        return output;
    }

    let mut output = existing.trim_end().to_owned();
    if !output.is_empty() {
        output.push_str("\n\n");
    }
    output.push_str(replacement);
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn is_omv_managed_integration_content(content: &str) -> bool {
    content.contains("<!-- OMV-MANAGED-FILE") || content.contains(MANAGED_BEGIN_PREFIX)
}

fn is_omv_managed_integration_file(content: &str) -> bool {
    content.trim_start().starts_with("<!-- OMV-MANAGED-FILE")
}

fn record_adapter_target(
    omv_root: &Path,
    target: &IntegrationTarget,
    plan: &IntegrationCapabilityPlan,
    mode: crate::core::adapter::AdapterTargetMode,
) -> Result<(), OmvError> {
    let mut registry = storage::adapters::load_adapters_if_exists(omv_root)?;
    let kind = if plan.provider == "codex" || plan.provider == "opencode" {
        crate::core::adapter::AdapterKind::Agent
    } else {
        crate::core::adapter::AdapterKind::Spec
    };
    let adapter_target = OmvAdapterTarget {
        path: target.host_rel.to_owned(),
        source_path: format!(".omv/{}/{}", adapter::AI_DIR, target.source_rel),
        mode,
    };

    if let Some(installation) = registry
        .installations
        .iter_mut()
        .find(|item| item.kind == kind && item.name == plan.provider)
    {
        if let Some(existing) = installation
            .targets
            .iter_mut()
            .find(|item| item.path == adapter_target.path)
        {
            *existing = adapter_target;
        } else {
            installation.targets.push(adapter_target);
        }
        installation.install_mode = derive_integration_install_mode(&installation.targets);
        installation.source_contract_version = adapter::CONTRACT_VERSION;
    } else {
        registry.installations.push(OmvAdapterInstallation {
            kind,
            name: plan.provider.clone(),
            install_mode: derive_integration_install_mode(std::slice::from_ref(&adapter_target)),
            source_contract_version: adapter::CONTRACT_VERSION,
            targets: vec![adapter_target],
        });
    }

    storage::adapters::save_adapters(omv_root, &registry)
}

fn derive_integration_install_mode(
    targets: &[OmvAdapterTarget],
) -> crate::core::adapter::AdapterInstallMode {
    if targets
        .iter()
        .all(|target| target.mode == crate::core::adapter::AdapterTargetMode::Link)
    {
        crate::core::adapter::AdapterInstallMode::Link
    } else if targets.iter().all(|target| {
        matches!(
            target.mode,
            crate::core::adapter::AdapterTargetMode::Materialize
                | crate::core::adapter::AdapterTargetMode::ManagedBlock
        )
    }) {
        crate::core::adapter::AdapterInstallMode::Materialize
    } else {
        crate::core::adapter::AdapterInstallMode::Hybrid
    }
}

fn execute_finalize_boundary(
    omv_root: &Path,
    ntp_source: &dyn TimeSource,
    system_source: &dyn TimeSource,
    ntp_override: Option<bool>,
    command: FinalizeBoundaryCommand,
) -> Result<FinalizeBoundaryExecution, OmvError> {
    let provider = require_finalize_task_field(command.provider, "provider")?;
    let boundary = require_finalize_task_field(command.boundary, "boundary")?;
    let source = finalization::flatten_boundary_source(&provider, &boundary).ok_or_else(|| {
        FinalizationError::InvalidField {
            field: String::from("boundary_identity"),
            value: format!("{provider}/{boundary}"),
        }
    })?;
    let project_root = omv_root.parent().unwrap_or(omv_root);
    let task_id = resolve_finalize_boundary_task_id(project_root, command.task_id.as_deref())?;
    let snapshot_hash = workspace_snapshot_hash(project_root, omv_root)?;
    let fingerprint = format!("finalize-boundary:{task_id}:{source}:{snapshot_hash}");

    let change_type = command
        .change_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let Some(change_type) = change_type else {
        return Ok(FinalizeBoundaryExecution {
            provider,
            boundary,
            source,
            task_id,
            change_type: None,
            status: TaskStatus::Done.as_str().to_owned(),
            tests: TestsStatus::Passed.as_str().to_owned(),
            fingerprint,
            workspace_snapshot_hash: snapshot_hash,
            outcome: String::from("pending"),
            reason: String::from("manual-action-missing-change-type"),
            manual_action_required: true,
            finalize_task: None,
        });
    };

    if ChangeType::parse(change_type).is_none() {
        return Err(FinalizationError::InvalidField {
            field: String::from("change_type"),
            value: change_type.to_owned(),
        }
        .into());
    }

    let finalize_task = execute_finalize_task(
        omv_root,
        ntp_source,
        system_source,
        ntp_override,
        FinalizeTaskCommand {
            task_id: Some(task_id.clone()),
            change_type: Some(change_type.to_owned()),
            status: Some(TaskStatus::Done.as_str().to_owned()),
            tests: Some(TestsStatus::Passed.as_str().to_owned()),
            fingerprint: Some(fingerprint.clone()),
            source: Some(source.clone()),
        },
    )?;

    Ok(FinalizeBoundaryExecution {
        provider,
        boundary,
        source,
        task_id,
        change_type: Some(change_type.to_owned()),
        status: TaskStatus::Done.as_str().to_owned(),
        tests: TestsStatus::Passed.as_str().to_owned(),
        fingerprint,
        workspace_snapshot_hash: snapshot_hash,
        outcome: finalize_task.outcome.clone(),
        reason: finalize_task.reason.clone(),
        manual_action_required: false,
        finalize_task: Some(finalize_task),
    })
}

fn execute_finalize_task(
    omv_root: &Path,
    ntp_source: &dyn TimeSource,
    system_source: &dyn TimeSource,
    ntp_override: Option<bool>,
    command: FinalizeTaskCommand,
) -> Result<FinalizeTaskExecution, OmvError> {
    let request = parse_finalize_task_request(command)?;
    let mut finalizations = storage::finalizations::load_finalizations_if_exists(omv_root)?;
    let current_state = storage::state::load_state(omv_root)?;

    if let Some(index) = finalizations
        .entries
        .iter()
        .position(|entry| entry.fingerprint == request.fingerprint)
    {
        let existing = finalizations.entries[index].clone();
        if existing.outcome == FinalizationOutcome::Pending
            && current_state.last_issued_version != existing.version_before
        {
            let sync = execute_sync(omv_root)?;
            let recovered = OmvFinalizationRecord {
                task_id: existing.task_id.clone(),
                fingerprint: existing.fingerprint.clone(),
                change_type: existing.change_type,
                task_status: existing.task_status,
                tests_status: existing.tests_status,
                source: existing.source.clone(),
                outcome: FinalizationOutcome::Bumped,
                reason: FinalizationReason::PendingRecovered,
                version_before: existing.version_before.clone(),
                version_after: sync.version.clone(),
                recorded_at: current_timestamp_string(),
            };
            upsert_finalization_record(&mut finalizations, recovered);
            storage::finalizations::save_finalizations(omv_root, &finalizations)?;

            return Ok(FinalizeTaskExecution {
                task_id: existing.task_id,
                fingerprint: existing.fingerprint,
                change_type: existing.change_type.as_str().to_owned(),
                status: existing.task_status.as_str().to_owned(),
                tests: existing.tests_status.as_str().to_owned(),
                source: existing.source,
                outcome: FinalizationOutcome::Bumped.as_str().to_owned(),
                reason: FinalizationReason::PendingRecovered.as_str().to_owned(),
                duplicate: false,
                recovered: true,
                version_before: existing.version_before,
                version_after: sync.version,
                synced: sync.synced,
                skipped: sync.skipped,
            });
        }

        if existing.outcome != FinalizationOutcome::Pending {
            let version_after = if existing.version_after.is_empty() {
                current_state.last_issued_version.clone()
            } else {
                existing.version_after.clone()
            };

            return Ok(FinalizeTaskExecution {
                task_id: existing.task_id,
                fingerprint: existing.fingerprint,
                change_type: existing.change_type.as_str().to_owned(),
                status: existing.task_status.as_str().to_owned(),
                tests: existing.tests_status.as_str().to_owned(),
                source: existing.source,
                outcome: existing.outcome.as_str().to_owned(),
                reason: FinalizationReason::DuplicateFingerprint.as_str().to_owned(),
                duplicate: true,
                recovered: false,
                version_before: existing.version_before,
                version_after,
                synced: 0,
                skipped: 0,
            });
        }
    }

    let decision = finalization::decide(
        request.change_type,
        request.task_status,
        request.tests_status,
    );
    let version_before = current_state.last_issued_version.clone();

    if !decision.should_bump() {
        let noop_record = OmvFinalizationRecord {
            task_id: request.task_id.clone(),
            fingerprint: request.fingerprint.clone(),
            change_type: request.change_type,
            task_status: request.task_status,
            tests_status: request.tests_status,
            source: request.source.clone(),
            outcome: FinalizationOutcome::NoOp,
            reason: decision.reason,
            version_before: version_before.clone(),
            version_after: version_before.clone(),
            recorded_at: current_timestamp_string(),
        };
        upsert_finalization_record(&mut finalizations, noop_record);
        storage::finalizations::save_finalizations(omv_root, &finalizations)?;

        return Ok(FinalizeTaskExecution {
            task_id: request.task_id,
            fingerprint: request.fingerprint,
            change_type: request.change_type.as_str().to_owned(),
            status: request.task_status.as_str().to_owned(),
            tests: request.tests_status.as_str().to_owned(),
            source: request.source,
            outcome: FinalizationOutcome::NoOp.as_str().to_owned(),
            reason: decision.reason.as_str().to_owned(),
            duplicate: false,
            recovered: false,
            version_before: version_before.clone(),
            version_after: version_before,
            synced: 0,
            skipped: 0,
        });
    }

    let pending_record = OmvFinalizationRecord {
        task_id: request.task_id.clone(),
        fingerprint: request.fingerprint.clone(),
        change_type: request.change_type,
        task_status: request.task_status,
        tests_status: request.tests_status,
        source: request.source.clone(),
        outcome: FinalizationOutcome::Pending,
        reason: decision.reason,
        version_before: version_before.clone(),
        version_after: String::new(),
        recorded_at: current_timestamp_string(),
    };
    upsert_finalization_record(&mut finalizations, pending_record);
    storage::finalizations::save_finalizations(omv_root, &finalizations)?;

    let bump = execute_bump(omv_root, ntp_source, system_source, ntp_override)?;

    let completed_record = OmvFinalizationRecord {
        task_id: request.task_id.clone(),
        fingerprint: request.fingerprint.clone(),
        change_type: request.change_type,
        task_status: request.task_status,
        tests_status: request.tests_status,
        source: request.source.clone(),
        outcome: FinalizationOutcome::Bumped,
        reason: decision.reason,
        version_before: version_before.clone(),
        version_after: bump.version.clone(),
        recorded_at: current_timestamp_string(),
    };
    upsert_finalization_record(&mut finalizations, completed_record);
    storage::finalizations::save_finalizations(omv_root, &finalizations)?;

    Ok(FinalizeTaskExecution {
        task_id: request.task_id,
        fingerprint: request.fingerprint,
        change_type: request.change_type.as_str().to_owned(),
        status: request.task_status.as_str().to_owned(),
        tests: request.tests_status.as_str().to_owned(),
        source: request.source,
        outcome: FinalizationOutcome::Bumped.as_str().to_owned(),
        reason: decision.reason.as_str().to_owned(),
        duplicate: false,
        recovered: false,
        version_before,
        version_after: bump.version,
        synced: bump.synced,
        skipped: bump.skipped,
    })
}

fn parse_finalize_task_request(
    command: FinalizeTaskCommand,
) -> Result<FinalizeTaskRequest, OmvError> {
    let task_id = require_finalize_task_field(command.task_id, "task_id")?;
    let change_type_value = require_finalize_task_field(command.change_type, "change_type")?;
    let status_value = require_finalize_task_field(command.status, "status")?;
    let tests_value = require_finalize_task_field(command.tests, "tests")?;
    let fingerprint = require_finalize_task_field(command.fingerprint, "fingerprint")?;
    let source = require_finalize_task_field(command.source, "source")?;

    Ok(FinalizeTaskRequest {
        task_id,
        change_type: ChangeType::parse(&change_type_value).ok_or_else(|| {
            FinalizationError::InvalidField {
                field: "change_type".to_owned(),
                value: change_type_value.clone(),
            }
        })?,
        task_status: TaskStatus::parse(&status_value).ok_or_else(|| {
            FinalizationError::InvalidField {
                field: "status".to_owned(),
                value: status_value.clone(),
            }
        })?,
        tests_status: TestsStatus::parse(&tests_value).ok_or_else(|| {
            FinalizationError::InvalidField {
                field: "tests".to_owned(),
                value: tests_value.clone(),
            }
        })?,
        fingerprint,
        source,
    })
}

fn require_finalize_task_field(value: Option<String>, field: &str) -> Result<String, OmvError> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(FinalizationError::MissingField(field.to_owned()).into()),
    }
}

fn resolve_finalize_boundary_task_id(
    project_root: &Path,
    explicit: Option<&str>,
) -> Result<String, OmvError> {
    if let Some(task_id) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(task_id.to_owned());
    }

    let current_task_path = project_root.join(".trellis/.current-task");
    let current_task_ref = fs::read_to_string(&current_task_path)
        .map_err(|_| FinalizationError::MissingField(String::from("task_id")))?
        .trim()
        .to_owned();
    if current_task_ref.is_empty() {
        return Err(FinalizationError::MissingField(String::from("task_id")).into());
    }

    let task_dir = resolve_trellis_task_ref(project_root, &current_task_ref);
    let task_json_path = task_dir.join("task.json");
    if let Ok(content) = fs::read_to_string(&task_json_path)
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(id) = value.get("id").and_then(|id| id.as_str())
        && !id.trim().is_empty()
    {
        return Ok(id.trim().to_owned());
    }

    task_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| FinalizationError::MissingField(String::from("task_id")).into())
}

fn resolve_trellis_task_ref(project_root: &Path, task_ref: &str) -> PathBuf {
    let normalized = task_ref.trim().trim_start_matches("./").replace('\\', "/");
    let path = PathBuf::from(&normalized);
    if path.is_absolute() {
        return path;
    }
    if normalized.starts_with(".trellis/") {
        return project_root.join(path);
    }
    if normalized.starts_with("tasks/") {
        return project_root.join(".trellis").join(path);
    }
    project_root.join(".trellis/tasks").join(path)
}

fn workspace_snapshot_hash(project_root: &Path, omv_root: &Path) -> Result<String, OmvError> {
    let normalized_paths = snapshot_normalized_paths(project_root, omv_root)?;
    let mut parts = Vec::new();

    if git_stdout(
        project_root,
        &[
            String::from("rev-parse"),
            String::from("--is-inside-work-tree"),
        ],
    )?
    .is_none()
    {
        parts.push((String::from("git"), b"unavailable".to_vec()));
        return Ok(stable_hash(parts));
    }

    let head = git_stdout(
        project_root,
        &[String::from("rev-parse"), String::from("HEAD")],
    )?
    .unwrap_or_else(|| b"NO_HEAD\n".to_vec());
    parts.push((String::from("head"), head));

    let mut staged_args = vec![
        String::from("diff"),
        String::from("--cached"),
        String::from("--binary"),
        String::from("--"),
        String::from("."),
    ];
    append_git_excludes(&mut staged_args, &normalized_paths);
    let staged = git_stdout(project_root, &staged_args)?.unwrap_or_default();
    parts.push((String::from("staged"), staged));

    let mut unstaged_args = vec![
        String::from("diff"),
        String::from("--binary"),
        String::from("--"),
        String::from("."),
    ];
    append_git_excludes(&mut unstaged_args, &normalized_paths);
    let unstaged = git_stdout(project_root, &unstaged_args)?.unwrap_or_default();
    parts.push((String::from("unstaged"), unstaged));

    let untracked = git_stdout(
        project_root,
        &[
            String::from("ls-files"),
            String::from("--others"),
            String::from("--exclude-standard"),
            String::from("-z"),
        ],
    )?
    .unwrap_or_default();
    for raw_path in untracked.split(|byte| *byte == 0) {
        if raw_path.is_empty() {
            continue;
        }
        let rel_path = String::from_utf8_lossy(raw_path).to_string();
        if normalized_paths.contains(&rel_path) {
            continue;
        }
        let path = project_root.join(&rel_path);
        if path.is_file() {
            parts.push((format!("untracked:{rel_path}"), fs::read(path)?));
        }
    }

    Ok(stable_hash(parts))
}

fn snapshot_normalized_paths(
    project_root: &Path,
    omv_root: &Path,
) -> Result<BTreeSet<String>, OmvError> {
    let mut paths = BTreeSet::new();
    for rel in [
        ".omv/state.toml",
        ".omv/finalizations.toml",
        ".omv/skills/README.md",
    ] {
        paths.insert(rel.to_owned());
    }

    let targets = load_targets_if_exists(omv_root)?;
    for target in targets.targets.iter().filter(|target| target.enabled) {
        insert_project_relative(
            &mut paths,
            project_root,
            &project_root.join(&target.root).join(&target.manifest_path),
        );
        insert_project_relative(
            &mut paths,
            project_root,
            &project_root
                .join(&target.root)
                .join(&target.runtime_export_path),
        );
    }
    for target in targets
        .v2_targets
        .iter()
        .filter(|target| target.enabled)
        .filter_map(|target| target.path().map(|path| (&target.root, path)))
    {
        insert_project_relative(
            &mut paths,
            project_root,
            &project_root.join(target.0).join(target.1),
        );
    }

    Ok(paths)
}

fn insert_project_relative(paths: &mut BTreeSet<String>, project_root: &Path, path: &Path) {
    let relative = path.strip_prefix(project_root).unwrap_or(path);
    paths.insert(relative.to_string_lossy().replace('\\', "/"));
}

fn append_git_excludes(args: &mut Vec<String>, paths: &BTreeSet<String>) {
    for path in paths {
        args.push(format!(":(exclude){path}"));
    }
}

fn git_stdout(project_root: &Path, args: &[String]) -> Result<Option<Vec<u8>>, OmvError> {
    match ProcessCommand::new("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
    {
        Ok(output) if output.status.success() => Ok(Some(output.stdout)),
        Ok(_) => Ok(None),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn stable_hash(parts: Vec<(String, Vec<u8>)>) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for (label, bytes) in parts {
        hash = fnv1a(hash, label.as_bytes());
        hash = fnv1a(hash, &[0]);
        hash = fnv1a(hash, bytes.len().to_string().as_bytes());
        hash = fnv1a(hash, &[0]);
        hash = fnv1a(hash, &bytes);
        hash = fnv1a(hash, &[0xff]);
    }
    format!("{hash:016x}")
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn upsert_finalization_record(
    finalizations: &mut crate::core::schema::OmvFinalizations,
    record: OmvFinalizationRecord,
) {
    if let Some(existing) = finalizations
        .entries
        .iter_mut()
        .find(|entry| entry.fingerprint == record.fingerprint)
    {
        *existing = record;
        return;
    }

    finalizations.entries.push(record);
}

fn current_timestamp_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| String::from("0"))
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

fn execute_plan(omv_root: &Path) -> Result<crate::sync::PlanSummary, OmvError> {
    let state = storage::state::load_state(omv_root)?;
    let targets = load_targets_if_exists(omv_root)?;
    let project_root = omv_root.parent().unwrap_or(omv_root);
    let mut plan =
        crate::sync::plan_all_targets(project_root, &targets, &state.last_issued_version);

    let adapters = storage::adapters::load_adapters_if_exists(omv_root)?;
    if adapters
        .installations
        .iter()
        .any(|installation| installation.source_contract_version < adapter::CONTRACT_VERSION)
    {
        plan.migration_status
            .push(String::from("adapter-refresh-needed"));
        if plan.project_status == "current-project" {
            plan.project_status = String::from("adapter-refresh-needed");
        }
    }

    Ok(plan)
}

fn persist_init_state(
    omv_root: &Path,
    draft: &InitDraft,
) -> Result<InitPersistenceOutcome, OmvError> {
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
    let integrations = persist_and_apply_init_integrations(omv_root, draft)?;

    Ok(InitPersistenceOutcome { integrations })
}

fn persist_and_apply_init_integrations(
    omv_root: &Path,
    draft: &InitDraft,
) -> Result<InitIntegrationOutcome, OmvError> {
    let project_root = omv_root.parent().unwrap_or(omv_root);
    let selected_capabilities = draft.selected_integrations();
    let selected_count = selected_capabilities.len();

    persist_init_integration_state(
        project_root,
        omv_root,
        draft,
        IntegrationCapabilityStatus::Pending,
    )?;

    if selected_count == 0 {
        return Ok(InitIntegrationOutcome {
            selected_capabilities: 0,
            status: InitIntegrationApplyStatus::NoSelection,
            reason: String::new(),
        });
    }

    let unsupported = selected_capabilities
        .iter()
        .filter(|(provider, capability)| integration_target(*provider, *capability).is_none())
        .map(|(provider, capability)| format!("{}:{}", provider.as_str(), capability.as_str()))
        .collect::<Vec<_>>();
    if !unsupported.is_empty() {
        return Ok(InitIntegrationOutcome {
            selected_capabilities: selected_count,
            status: InitIntegrationApplyStatus::Deferred,
            reason: format!("unsupported-capability:{}", unsupported.join(", ")),
        });
    }

    let target_paths = selected_capabilities
        .iter()
        .flat_map(|(_, capability)| integration_capability_target_files(*capability))
        .collect::<Vec<_>>();
    let dirty_targets = dirty_integration_targets(project_root, &target_paths);
    if !dirty_targets.is_empty() {
        return Ok(InitIntegrationOutcome {
            selected_capabilities: selected_count,
            status: InitIntegrationApplyStatus::Deferred,
            reason: format!("unsafe-worktree:{}", dirty_targets.join(", ")),
        });
    }

    execute_integrate_apply(omv_root, project_root)?;
    Ok(InitIntegrationOutcome {
        selected_capabilities: selected_count,
        status: InitIntegrationApplyStatus::Applied,
        reason: String::new(),
    })
}

fn persist_init_integration_state(
    project_root: &Path,
    omv_root: &Path,
    draft: &InitDraft,
    selected_status: IntegrationCapabilityStatus,
) -> Result<(), OmvError> {
    let providers = draft
        .integrations
        .iter()
        .map(|provider| {
            let selected = provider
                .capabilities
                .iter()
                .any(|capability| capability.selected);
            let detection = detect_integration_provider(project_root, provider.provider);
            OmvIntegrationProviderState {
                provider: provider.provider,
                selected,
                detection: IntegrationDetectionSnapshot {
                    detected: detection.detected,
                    recommended: provider.recommended,
                },
                capabilities: provider
                    .capabilities
                    .iter()
                    .map(|capability| OmvIntegrationCapabilityState {
                        capability: capability.capability,
                        selected: capability.selected,
                        status: if capability.selected {
                            selected_status
                        } else {
                            IntegrationCapabilityStatus::Selected
                        },
                        failure: None,
                    })
                    .collect(),
            }
        })
        .collect();

    storage::integrations::save_integrations(
        omv_root,
        &OmvIntegrations {
            schema_version: 1,
            providers,
        },
    )
}

fn dirty_integration_targets(project_root: &Path, target_files: &[String]) -> Vec<String> {
    if target_files.is_empty() || !project_root.join(".git").exists() {
        return Vec::new();
    }

    let output = ProcessCommand::new("git")
        .arg("-C")
        .arg(project_root)
        .arg("status")
        .arg("--porcelain")
        .arg("--")
        .args(target_files)
        .output();

    let Ok(output) = output else {
        return target_files.to_owned();
    };
    if !output.status.success() {
        return target_files.to_owned();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let path = line.get(3..)?.trim();
            if path.is_empty() {
                None
            } else {
                Some(path.to_owned())
            }
        })
        .collect()
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
        v2_targets: Vec::new(),
        unsupported_targets: Vec::new(),
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
    use std::process::Command as ProcessCommand;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::cli::{
        AdapterAction, AdapterCommand, Cli, Command, FinalizeBoundaryCommand, FinalizeTaskCommand,
        OutputMode,
    };
    use crate::core::date::LogicalDate;
    use crate::core::finalization::FinalizationOutcome;
    use crate::core::locale::OperatorLocale;
    use crate::core::schema::{OmvConfig, OmvState};
    use crate::core::target::TargetLanguage;
    use crate::core::time::{LastTimeSource, TimeSource};
    use crate::core::versioning::BuildPolicy;
    use crate::errors::{NtpError, OmvError, StateError};
    use crate::storage;
    use crate::ui::state::draft::InitDraft;

    use super::{
        AppRuntime, execute_bump, execute_current, execute_finalize_boundary,
        execute_finalize_task, execute_sync, persist_init_state, render_help,
        render_structured_error, render_version, resolve_finalize_boundary_task_id,
        resolve_locale_for_root, run_adapter, run_with_runtime,
    };

    static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

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
    fn run_with_runtime_uses_injected_ntp_for_same_day_bump() {
        let root = temp_project_root("runtime-same-day");
        let omv_root = root.join(".omv");
        fs::create_dir_all(&omv_root).expect("omv root should exist");
        storage::config::save_config(&omv_root, &OmvConfig::default()).expect("config should save");
        storage::state::save_state(
            &omv_root,
            &OmvState {
                logical_date: "2026-04-30".to_owned(),
                build_number: 3,
                last_issued_version: "2604.30.3".to_owned(),
                ..OmvState::default()
            },
        )
        .expect("state should save");

        let ntp = FixedSource {
            source: LastTimeSource::Ntp,
            date: LogicalDate::parse_iso("2026-04-30").expect("date should parse"),
        };
        let system = FixedSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-29").expect("date should parse"),
        };
        let runtime = AppRuntime {
            ntp_source: &ntp,
            system_source: &system,
        };

        with_cwd(&root, || {
            let output = run_with_runtime(
                Cli {
                    command: Command::Bump,
                    locale_override: Some("en-US".to_owned()),
                    ntp_override: None,
                    output_mode: OutputMode::Json,
                },
                &runtime,
            )
            .expect("bump should use injected NTP date");
            assert!(output.message.contains("\"version\": \"2604.30.4\""));
        });

        let state = storage::state::load_state(&omv_root).expect("state should load");
        assert_eq!(state.logical_date, "2026-04-30");
        assert_eq!(state.build_number, 4);
        assert_eq!(state.last_issued_version, "2604.30.4");

        cleanup_project_root(&root);
    }

    #[test]
    fn run_with_runtime_uses_injected_ntp_for_next_day_reset() {
        let root = temp_project_root("runtime-next-day");
        let omv_root = root.join(".omv");
        fs::create_dir_all(&omv_root).expect("omv root should exist");
        storage::config::save_config(&omv_root, &OmvConfig::default()).expect("config should save");
        storage::state::save_state(
            &omv_root,
            &OmvState {
                logical_date: "2026-04-30".to_owned(),
                build_number: 3,
                last_issued_version: "2604.30.3".to_owned(),
                ..OmvState::default()
            },
        )
        .expect("state should save");

        let ntp = FixedSource {
            source: LastTimeSource::Ntp,
            date: LogicalDate::parse_iso("2026-05-01").expect("date should parse"),
        };
        let system = FixedSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-30").expect("date should parse"),
        };
        let runtime = AppRuntime {
            ntp_source: &ntp,
            system_source: &system,
        };

        with_cwd(&root, || {
            let output = run_with_runtime(
                Cli {
                    command: Command::Bump,
                    locale_override: Some("en-US".to_owned()),
                    ntp_override: None,
                    output_mode: OutputMode::Json,
                },
                &runtime,
            )
            .expect("bump should use injected next-day NTP date");
            assert!(output.message.contains("\"version\": \"2605.1.1\""));
        });

        let state = storage::state::load_state(&omv_root).expect("state should load");
        assert_eq!(state.logical_date, "2026-05-01");
        assert_eq!(state.build_number, 1);
        assert_eq!(state.last_issued_version, "2605.1.1");

        cleanup_project_root(&root);
    }

    #[test]
    fn execute_finalize_task_bumps_once_for_semantic_change() {
        let omv_root = temp_omv_root("finalize-bump");
        storage::config::save_config(&omv_root, &OmvConfig::default()).expect("config should save");
        storage::state::save_state(
            &omv_root,
            &OmvState {
                logical_date: "2026-04-13".to_owned(),
                build_number: 1,
                last_issued_version: "2604.13.1".to_owned(),
                ..OmvState::default()
            },
        )
        .expect("state should save");

        let ntp = FixedSource {
            source: LastTimeSource::Ntp,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };
        let system = FixedSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };

        let execution = execute_finalize_task(
            &omv_root,
            &ntp,
            &system,
            None,
            finalize_task_command("bugfix", "task-1:v1"),
        )
        .expect("finalize-task should succeed");
        assert_eq!(execution.outcome, "bumped");
        assert!(!execution.duplicate);
        assert_eq!(execution.version_before, "2604.13.1");
        assert_eq!(execution.version_after, "2604.13.2");

        let state = storage::state::load_state(&omv_root).expect("state should load");
        assert_eq!(state.last_issued_version, "2604.13.2");

        let finalizations = storage::finalizations::load_finalizations_if_exists(&omv_root)
            .expect("finalizations should load");
        assert_eq!(finalizations.entries.len(), 1);
        assert_eq!(
            finalizations.entries[0].outcome,
            FinalizationOutcome::Bumped
        );

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn execute_finalize_task_returns_duplicate_without_second_bump() {
        let omv_root = temp_omv_root("finalize-duplicate");
        storage::config::save_config(&omv_root, &OmvConfig::default()).expect("config should save");
        storage::state::save_state(
            &omv_root,
            &OmvState {
                logical_date: "2026-04-13".to_owned(),
                build_number: 1,
                last_issued_version: "2604.13.1".to_owned(),
                ..OmvState::default()
            },
        )
        .expect("state should save");

        let ntp = FixedSource {
            source: LastTimeSource::Ntp,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };
        let system = FixedSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };
        let command = finalize_task_command("feature", "task-1:v1");

        execute_finalize_task(&omv_root, &ntp, &system, None, command.clone())
            .expect("first finalize-task should succeed");
        let duplicate = execute_finalize_task(&omv_root, &ntp, &system, None, command)
            .expect("duplicate finalize-task should succeed");

        assert!(duplicate.duplicate);
        assert_eq!(duplicate.outcome, "bumped");
        assert_eq!(duplicate.version_after, "2604.13.2");

        let state = storage::state::load_state(&omv_root).expect("state should load");
        assert_eq!(state.build_number, 2);
        assert_eq!(state.last_issued_version, "2604.13.2");

        let finalizations = storage::finalizations::load_finalizations_if_exists(&omv_root)
            .expect("finalizations should load");
        assert_eq!(finalizations.entries.len(), 1);

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn execute_finalize_task_records_noop_for_non_semantic_change() {
        let omv_root = temp_omv_root("finalize-noop");
        storage::config::save_config(&omv_root, &OmvConfig::default()).expect("config should save");
        storage::state::save_state(
            &omv_root,
            &OmvState {
                logical_date: "2026-04-13".to_owned(),
                build_number: 1,
                last_issued_version: "2604.13.1".to_owned(),
                ..OmvState::default()
            },
        )
        .expect("state should save");

        let ntp = FixedSource {
            source: LastTimeSource::Ntp,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };
        let system = FixedSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };

        let execution = execute_finalize_task(
            &omv_root,
            &ntp,
            &system,
            None,
            finalize_task_command("docs", "task-1:v1"),
        )
        .expect("finalize-task noop should succeed");
        assert_eq!(execution.outcome, "noop");
        assert_eq!(execution.version_before, "2604.13.1");
        assert_eq!(execution.version_after, "2604.13.1");

        let state = storage::state::load_state(&omv_root).expect("state should load");
        assert_eq!(state.build_number, 1);
        assert_eq!(state.last_issued_version, "2604.13.1");

        let finalizations = storage::finalizations::load_finalizations_if_exists(&omv_root)
            .expect("finalizations should load");
        assert_eq!(finalizations.entries.len(), 1);
        assert_eq!(finalizations.entries[0].outcome, FinalizationOutcome::NoOp);

        cleanup_omv_root(&omv_root);
    }

    #[test]
    fn finalize_boundary_missing_change_type_returns_pending_without_record() {
        let root = temp_project_root("boundary-pending");
        let omv_root = root.join(".omv");
        fs::create_dir_all(&omv_root).expect("omv root should exist");
        write_current_trellis_task(&root, "05-01-example-task", "example-task");

        let ntp = FixedSource {
            source: LastTimeSource::Ntp,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };
        let system = FixedSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };

        let execution = execute_finalize_boundary(
            &omv_root,
            &ntp,
            &system,
            None,
            FinalizeBoundaryCommand {
                provider: Some("trellis".to_owned()),
                boundary: Some("finish-work".to_owned()),
                task_id: None,
                change_type: None,
            },
        )
        .expect("missing change type should be pending, not an error");

        assert_eq!(execution.task_id, "example-task");
        assert_eq!(execution.outcome, "pending");
        assert!(execution.manual_action_required);
        assert!(execution.finalize_task.is_none());
        assert!(!omv_root.join("finalizations.toml").exists());

        cleanup_project_root(&root);
    }

    #[test]
    fn finalize_boundary_resolves_current_task_and_honors_explicit_override() {
        let root = temp_project_root("boundary-task-resolution");
        write_current_trellis_task(&root, "05-01-example-task", "example-task");

        let resolved =
            resolve_finalize_boundary_task_id(&root, None).expect("current task should resolve");
        let explicit = resolve_finalize_boundary_task_id(&root, Some("override-task"))
            .expect("explicit task should resolve");

        assert_eq!(resolved, "example-task");
        assert_eq!(explicit, "override-task");

        cleanup_project_root(&root);
    }

    #[test]
    fn finalize_boundary_duplicate_uses_normalized_workspace_fingerprint() {
        let root = temp_project_root("boundary-duplicate");
        init_git_repo(&root);
        let omv_root = root.join(".omv");
        let mut draft = InitDraft::from_detected_languages(&[TargetLanguage::Rust]);
        draft.set_locale(OperatorLocale::EnUs);
        persist_init_state(&omv_root, &draft).expect("init state should persist");
        storage::state::save_state(
            &omv_root,
            &OmvState {
                logical_date: "2026-04-13".to_owned(),
                build_number: 1,
                last_issued_version: "2604.13.1".to_owned(),
                ..OmvState::default()
            },
        )
        .expect("state should save");
        execute_sync(&omv_root).expect("initial sync should create managed outputs");
        write_current_trellis_task(&root, "05-01-example-task", "example-task");
        fs::create_dir_all(root.join("src")).expect("src dir should exist");
        fs::write(root.join("src/lib.rs"), "pub fn changed() {}\n").expect("work file");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "seed"]);
        fs::write(
            root.join("src/lib.rs"),
            "pub fn changed() -> bool { true }\n",
        )
        .expect("work file should update");

        let ntp = FixedSource {
            source: LastTimeSource::Ntp,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };
        let system = FixedSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };
        let command = FinalizeBoundaryCommand {
            provider: Some("trellis".to_owned()),
            boundary: Some("finish-work".to_owned()),
            task_id: None,
            change_type: Some("feature".to_owned()),
        };

        let first = execute_finalize_boundary(&omv_root, &ntp, &system, None, command.clone())
            .expect("first boundary should finalize");
        let second = execute_finalize_boundary(&omv_root, &ntp, &system, None, command)
            .expect("second boundary should be duplicate");

        assert_eq!(first.fingerprint, second.fingerprint);
        assert_eq!(first.outcome, "bumped");
        assert_eq!(second.outcome, "bumped");
        assert!(
            second
                .finalize_task
                .as_ref()
                .expect("finalize result")
                .duplicate
        );

        let state = storage::state::load_state(&omv_root).expect("state should load");
        assert_eq!(state.build_number, 2);

        cleanup_project_root(&root);
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
        assert!(help.contains("event"));
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

    fn with_cwd<T>(cwd: &std::path::Path, run: impl FnOnce() -> T) -> T {
        let lock = CWD_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

        let previous = std::env::current_dir().expect("current dir should resolve");
        std::env::set_current_dir(cwd).expect("set_current_dir should succeed");

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));
        std::env::set_current_dir(previous).expect("restore current dir should succeed");

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    fn write_current_trellis_task(root: &std::path::Path, dir_name: &str, task_id: &str) {
        let task_dir = root.join(".trellis/tasks").join(dir_name);
        fs::create_dir_all(&task_dir).expect("task dir should exist");
        fs::write(
            root.join(".trellis/.current-task"),
            format!(".trellis/tasks/{dir_name}"),
        )
        .expect("current task should write");
        fs::write(
            task_dir.join("task.json"),
            format!(r#"{{"id":"{task_id}","title":"Example"}}"#),
        )
        .expect("task json should write");
    }

    fn init_git_repo(root: &std::path::Path) {
        git(root, &["init"]);
        git(root, &["config", "user.email", "omv@example.invalid"]);
        git(root, &["config", "user.name", "OMV Test"]);
    }

    fn git(root: &std::path::Path, args: &[&str]) {
        let status = ProcessCommand::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .expect("git should run");
        assert!(status.success(), "git command failed: {args:?}");
    }

    fn finalize_task_command(change_type: &str, fingerprint: &str) -> FinalizeTaskCommand {
        FinalizeTaskCommand {
            task_id: Some("04-18-product-gaps-automation-hooks".to_owned()),
            change_type: Some(change_type.to_owned()),
            status: Some("done".to_owned()),
            tests: Some("passed".to_owned()),
            fingerprint: Some(fingerprint.to_owned()),
            source: Some("trellis-finish-work".to_owned()),
        }
    }
}
