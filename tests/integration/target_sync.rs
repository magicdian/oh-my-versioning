use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use omv::app;
use omv::cli::{Cli, Command, IntegrateAction, IntegrateCommand, OutputMode, SyncCommand};
use omv::core::date::LogicalDate;
use omv::core::integration::{
    IntegrationCapability, IntegrationCapabilityStatus, IntegrationDetectionSnapshot,
    IntegrationProvider, OmvIntegrationCapabilityState, OmvIntegrationProviderState,
    OmvIntegrations,
};
use omv::core::locale::OperatorLocale;
use omv::core::schema::{
    CHeaderMacroTarget, CargoWorkspaceTarget, MarkdownManagedBlockTarget, OmvConfig, OmvState,
    OmvTargetRecord, OmvTargets, OmvV2TargetConfig, OmvV2TargetRecord, RegexReplaceTarget,
    TextScalarTarget, YamlScalarTarget,
};
use omv::core::target::{
    CargoLockfileStrategy, CargoMembers, CargoVersionLocation, CargoVersionPolicy,
    PreProjectStrategy, TargetKind, TargetLanguage, TargetMode,
};
use omv::core::time::LastTimeSource;
use omv::core::versioning::engine;
use omv::storage;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn sync_command_updates_all_v1_targets_and_skills_guidance() {
    let project_root = temp_project_root("sync-all");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");

    storage::config::save_config(&omv_root, &OmvConfig::default()).expect("config should save");
    storage::state::save_state(
        &omv_root,
        &OmvState {
            schema_version: 1,
            logical_date: "2026-04-13".to_owned(),
            build_number: 9,
            last_issued_version: "2604.13.9".to_owned(),
            last_time_source: LastTimeSource::System,
        },
    )
    .expect("state should save");
    storage::targets::save_targets(
        &omv_root,
        &OmvTargets {
            schema_version: 1,
            targets: vec![
                target(
                    "workspace-rust",
                    TargetLanguage::Rust,
                    "Cargo.toml",
                    "src/generated/version.rs",
                ),
                target(
                    "workspace-python",
                    TargetLanguage::Python,
                    "pyproject.toml",
                    "omv_generated/version.py",
                ),
                target(
                    "workspace-go",
                    TargetLanguage::Go,
                    "go.mod",
                    "internal/omv/version.go",
                ),
                target(
                    "workspace-java",
                    TargetLanguage::Java,
                    "pom.xml",
                    "src/main/java/omv/Version.java",
                ),
                target(
                    "workspace-c-family",
                    TargetLanguage::CFamily,
                    "CMakeLists.txt",
                    "include/omv_version.h",
                ),
            ],
            v2_targets: Vec::new(),
            unsupported_targets: Vec::new(),
        },
    )
    .expect("targets should save");

    with_cwd(&project_root, || {
        let output = app::run(Cli {
            command: Command::Sync(SyncCommand::default()),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Text,
        })
        .expect("sync command should succeed");
        assert!(output.message.contains("2604.13.9"));
        assert!(output.message.contains("synced: 5"));
    });

    assert_file_contains(&project_root.join("Cargo.toml"), "version = \"2604.13.9\"");
    assert_file_contains(
        &project_root.join("src/generated/version.rs"),
        "OMV_VERSION: &str = \"2604.13.9\"",
    );

    assert_file_contains(
        &project_root.join("pyproject.toml"),
        "version = \"2604.13.9\"",
    );
    assert_file_contains(
        &project_root.join("omv_generated/version.py"),
        "OMV_VERSION = \"2604.13.9\"",
    );

    assert_file_contains(&project_root.join("go.mod"), "// omv-version: 2604.13.9");
    assert_file_contains(
        &project_root.join("internal/omv/version.go"),
        "const Version = \"2604.13.9\"",
    );

    assert_file_contains(
        &project_root.join("pom.xml"),
        "<version>2604.13.9</version>",
    );
    assert_file_contains(
        &project_root.join("src/main/java/omv/Version.java"),
        "public static final String VALUE = \"2604.13.9\";",
    );

    assert_file_contains(
        &project_root.join("CMakeLists.txt"),
        "set(OMV_VERSION \"2604.13.9\")",
    );
    assert_file_contains(
        &project_root.join("include/omv_version.h"),
        "#define OMV_VERSION \"2604.13.9\"",
    );

    assert_file_contains(&omv_root.join("skills/README.md"), "Use `omv bump`");
    assert_file_contains(
        &omv_root.join("skills/bump-guidance.md"),
        "Do not edit native manifest versions directly.",
    );

    cleanup_project_root(&project_root);
}

#[test]
fn bump_command_updates_state_and_syncs_registered_targets() {
    let project_root = temp_project_root("bump-sync");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");

    let config = OmvConfig {
        locale: OperatorLocale::EnUs,
        ntp_enabled: false,
        ..OmvConfig::default()
    };
    storage::config::save_config(&omv_root, &config).expect("config should save");

    let today = LogicalDate::today_from_system().expect("system date should resolve");
    let initial_version = engine::format_version(today, 1, config.version_output);
    storage::state::save_state(
        &omv_root,
        &OmvState {
            schema_version: 1,
            logical_date: today.to_iso_string(),
            build_number: 1,
            last_issued_version: initial_version,
            last_time_source: LastTimeSource::System,
        },
    )
    .expect("state should save");

    storage::targets::save_targets(
        &omv_root,
        &OmvTargets {
            schema_version: 1,
            targets: vec![target(
                "workspace-rust",
                TargetLanguage::Rust,
                "Cargo.toml",
                "src/generated/version.rs",
            )],
            v2_targets: Vec::new(),
            unsupported_targets: Vec::new(),
        },
    )
    .expect("targets should save");

    with_cwd(&project_root, || {
        app::run(Cli {
            command: Command::Bump,
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Text,
        })
        .expect("bump command should succeed");
    });

    let state = storage::state::load_state(&omv_root).expect("updated state should load");
    assert_eq!(state.logical_date, today.to_iso_string());
    assert_eq!(state.build_number, 2);
    assert_eq!(state.last_time_source, LastTimeSource::System);
    let expected_version = engine::format_version(today, 2, config.version_output);
    assert_eq!(state.last_issued_version, expected_version);

    assert_file_contains(
        &project_root.join("Cargo.toml"),
        &format!("version = \"{}\"", state.last_issued_version),
    );
    assert_file_contains(
        &project_root.join("src/generated/version.rs"),
        &state.last_issued_version,
    );
    assert_file_contains(
        &omv_root.join("skills/README.md"),
        &state.last_issued_version,
    );

    cleanup_project_root(&project_root);
}

#[test]
fn plan_command_reports_v1_target_status_without_mutation() {
    let project_root = temp_project_root("plan-json");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");

    storage::config::save_config(&omv_root, &OmvConfig::default()).expect("config should save");
    storage::state::save_state(
        &omv_root,
        &OmvState {
            schema_version: 1,
            logical_date: "2026-04-13".to_owned(),
            build_number: 9,
            last_issued_version: "2604.13.9".to_owned(),
            last_time_source: LastTimeSource::System,
        },
    )
    .expect("state should save");
    storage::targets::save_targets(
        &omv_root,
        &OmvTargets {
            schema_version: 1,
            targets: vec![target(
                "workspace-rust",
                TargetLanguage::Rust,
                "Cargo.toml",
                "src/generated/version.rs",
            )],
            v2_targets: Vec::new(),
            unsupported_targets: Vec::new(),
        },
    )
    .expect("targets should save");

    with_cwd(&project_root, || {
        let output = app::run(Cli {
            command: Command::Plan,
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("plan command should succeed");
        assert!(output.message.contains("\"command\": \"plan\""));
        assert!(output.message.contains("\"status\": \"missing\""));
        assert!(output.message.contains("\"workspace-rust\""));
    });

    assert!(!project_root.join("Cargo.toml").exists());
    cleanup_project_root(&project_root);
}

#[test]
fn sync_check_passes_when_targets_are_current_and_fails_on_drift_without_mutation() {
    let project_root = temp_project_root("sync-check");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");

    storage::config::save_config(&omv_root, &OmvConfig::default()).expect("config should save");
    storage::state::save_state(
        &omv_root,
        &OmvState {
            schema_version: 1,
            logical_date: "2026-04-13".to_owned(),
            build_number: 9,
            last_issued_version: "2604.13.9".to_owned(),
            last_time_source: LastTimeSource::System,
        },
    )
    .expect("state should save");
    storage::targets::save_targets(
        &omv_root,
        &OmvTargets {
            schema_version: 1,
            targets: vec![target(
                "workspace-rust",
                TargetLanguage::Rust,
                "Cargo.toml",
                "src/generated/version.rs",
            )],
            v2_targets: Vec::new(),
            unsupported_targets: Vec::new(),
        },
    )
    .expect("targets should save");

    with_cwd(&project_root, || {
        app::run(Cli {
            command: Command::Sync(SyncCommand::default()),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Text,
        })
        .expect("sync command should seed targets");

        let ok = app::run(Cli {
            command: Command::Sync(SyncCommand { check: true }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("sync check should pass");
        assert!(ok.message.contains("\"command\": \"sync.check\""));
        assert!(ok.message.contains("\"ok\": true"));

        fs::write(project_root.join("Cargo.toml"), "version = \"0.0.0\"\n")
            .expect("drift seed should write");
        let err = app::run(Cli {
            command: Command::Sync(SyncCommand { check: true }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect_err("sync check should fail on drift");
        assert_eq!(err.code(), "sync_check_failed");
    });

    assert_file_contains(&project_root.join("Cargo.toml"), "version = \"0.0.0\"");
    cleanup_project_root(&project_root);
}

#[test]
fn mixed_v2_targets_plan_check_and_sync_through_shared_engine() {
    let project_root = temp_project_root("v2-mixed");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");

    storage::config::save_config(&omv_root, &OmvConfig::default()).expect("config should save");
    storage::state::save_state(
        &omv_root,
        &OmvState {
            schema_version: 1,
            logical_date: "2026-05-01".to_owned(),
            build_number: 3,
            last_issued_version: "2605.1.3".to_owned(),
            last_time_source: LastTimeSource::System,
        },
    )
    .expect("state should save");

    seed_v2_files(&project_root);
    storage::targets::save_targets(
        &omv_root,
        &OmvTargets {
            schema_version: 2,
            targets: vec![target(
                "workspace-rust",
                TargetLanguage::Rust,
                "Cargo.toml",
                "src/generated/version.rs",
            )],
            v2_targets: v2_targets(),
            unsupported_targets: Vec::new(),
        },
    )
    .expect("targets should save");

    with_cwd(&project_root, || {
        let plan = app::run(Cli {
            command: Command::Plan,
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("plan should support mixed V2 targets");
        assert!(plan.message.contains("\"kind\": \"text-scalar\""));
        assert!(plan.message.contains("\"kind\": \"cargo-workspace\""));
        assert!(plan.message.contains("\"status\": \"drift\""));

        let err = app::run(Cli {
            command: Command::Sync(SyncCommand { check: true }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect_err("sync check should fail on mixed drift");
        assert_eq!(err.code(), "sync_check_failed");
        assert_file_contains(&project_root.join("VERSION"), "0.0.1");

        app::run(Cli {
            command: Command::Sync(SyncCommand::default()),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Text,
        })
        .expect("sync should apply mixed V2 targets");

        app::run(Cli {
            command: Command::Sync(SyncCommand { check: true }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("sync check should pass after sync");
    });

    assert_file_contains(&project_root.join("VERSION"), "2605.1.3");
    assert_file_contains(&project_root.join("README.md"), "version-2605.1.3-blue");
    assert_file_contains(
        &project_root.join("docs/version.md"),
        "Managed version: 2605.1.3",
    );
    assert_file_contains(&project_root.join("component.yml"), "version: 2605.1.3");
    assert_file_contains(
        &project_root.join("include/example_version.h"),
        "#define EXAMPLE_VERSION \"2605.1.3\"",
    );
    assert_file_contains(
        &project_root.join("tools/example/crates/a/Cargo.toml"),
        "version = \"2605.1.3\"",
    );
    assert_file_contains(
        &project_root.join("tools/example/Cargo.lock"),
        "version = \"2605.1.3\"",
    );

    cleanup_project_root(&project_root);
}

#[test]
fn kind_targets_are_not_gated_by_user_visible_schema_version() {
    let project_root = temp_project_root("kind-schema-capability");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");

    storage::config::save_config(&omv_root, &OmvConfig::default()).expect("config should save");
    storage::state::save_state(
        &omv_root,
        &OmvState {
            schema_version: 1,
            logical_date: "2026-05-01".to_owned(),
            build_number: 3,
            last_issued_version: "2605.1.3".to_owned(),
            last_time_source: LastTimeSource::System,
        },
    )
    .expect("state should save");
    fs::write(
        omv_root.join("targets.toml"),
        r#"schema_version = 1

[[targets]]
id = "root-version-file"
kind = "text-scalar"
path = "VERSION"
template = "{version}\n"

[[targets]]
id = "future-workspace"
kind = "future-workspace"
path = "future.toml"
"#,
    )
    .expect("targets fixture should write");

    with_cwd(&project_root, || {
        let plan = app::run(Cli {
            command: Command::Plan,
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("plan should not reject kind targets because schema_version is 1");
        assert!(plan.message.contains("\"id\": \"root-version-file\""));
        assert!(plan.message.contains("\"status\": \"missing\""));
        assert!(plan.message.contains("\"id\": \"future-workspace\""));
        assert!(plan.message.contains("\"status\": \"unsupported\""));
        assert!(plan.message.contains("update OMV"));

        let err = app::run(Cli {
            command: Command::Sync(SyncCommand { check: true }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect_err("sync check should fail on required unsupported target");
        assert_eq!(err.code(), "sync_check_failed");

        let err = app::run(Cli {
            command: Command::Sync(SyncCommand::default()),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect_err("sync should not apply while a required unsupported target exists");
        assert_eq!(err.code(), "invalid_target_record");
    });

    assert!(!project_root.join("VERSION").exists());
    assert!(!project_root.join("future.toml").exists());
    cleanup_project_root(&project_root);
}

#[test]
fn integrate_status_json_reports_matrix_without_integrations_file() {
    let project_root = temp_project_root("integrate-status-empty");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");

    with_cwd(&project_root, || {
        let output = app::run(Cli {
            command: Command::Integrate(IntegrateCommand {
                action: IntegrateAction::Status,
            }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("integrate status should succeed without integrations file");

        assert!(output.message.contains("\"command\": \"integrate.status\""));
        assert!(output.message.contains("\"provider\": \"codex\""));
        assert!(
            output
                .message
                .contains("\"capability\": \"project-instructions\"")
        );
        assert!(output.message.contains("\"provider\": \"trellis\""));
    });

    cleanup_project_root(&project_root);
}

#[test]
fn integrate_apply_bootstraps_codex_and_reports_json_envelope() {
    let project_root = temp_project_root("integrate-apply-codex");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");

    with_cwd(&project_root, || {
        let output = app::run(Cli {
            command: Command::Integrate(IntegrateCommand {
                action: IntegrateAction::Apply,
            }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("codex integrate apply should succeed");

        assert!(output.message.contains("\"command\": \"integrate.apply\""));
        assert!(output.message.contains("\"ok\": true"));
        assert!(output.message.contains("\"succeeded\": 2"));
    });

    assert_file_contains(&project_root.join("AGENTS.md"), "OMV Codex Adapter");
    assert_file_contains(
        &project_root.join(".codex/skills/omv-versioning/SKILL.md"),
        "omv-versioning",
    );
    let codex_skill =
        fs::read_to_string(project_root.join(".codex/skills/omv-versioning/SKILL.md"))
            .expect("codex skill host file should exist");
    assert!(codex_skill.starts_with("---\n"));
    assert!(codex_skill.contains("<!-- OMV-MANAGED-FILE"));
    assert_file_contains(
        &omv_root.join("integrations.toml"),
        "status = \"installed\"",
    );
    assert_file_contains(&omv_root.join("adapters.toml"), "name = \"codex\"");

    cleanup_project_root(&project_root);
}

#[test]
fn integrate_apply_preserves_successful_capability_when_later_capability_fails() {
    let project_root = temp_project_root("integrate-apply-partial");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");
    fs::create_dir_all(project_root.join(".codex/skills/omv-versioning"))
        .expect("codex skill dir should be created");
    fs::write(
        project_root.join(".codex/skills/omv-versioning/SKILL.md"),
        "unmanaged skill\n",
    )
    .expect("unmanaged skill should write");

    with_cwd(&project_root, || {
        let err = app::run(Cli {
            command: Command::Integrate(IntegrateCommand {
                action: IntegrateAction::Apply,
            }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect_err("partial apply should return non-zero error path");

        assert_eq!(err.code(), "integration_apply_failed");
        let structured = app::render_structured_error("integrate.apply", &err);
        assert!(structured.contains("\"capability\": \"project-instructions\""));
        assert!(structured.contains("\"status\": \"installed\""));
        assert!(structured.contains("\"capability\": \"host-skill\""));
        assert!(structured.contains("\"status\": \"failed\""));
    });

    assert_file_contains(&project_root.join("AGENTS.md"), "OMV Codex Adapter");
    assert_file_contains(
        &project_root.join(".codex/skills/omv-versioning/SKILL.md"),
        "unmanaged skill",
    );
    assert_file_contains(
        &omv_root.join("integrations.toml"),
        "status = \"installed\"",
    );
    assert_file_contains(&omv_root.join("integrations.toml"), "status = \"failed\"");

    cleanup_project_root(&project_root);
}

#[test]
fn integrate_apply_rejects_selected_trellis_when_not_detected() {
    let project_root = temp_project_root("integrate-apply-trellis-missing");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");
    storage::integrations::save_integrations(
        &omv_root,
        &OmvIntegrations {
            schema_version: 1,
            providers: vec![
                integration_provider(
                    IntegrationProvider::Codex,
                    false,
                    false,
                    &[
                        (
                            IntegrationCapability::ProjectInstructions,
                            false,
                            IntegrationCapabilityStatus::Selected,
                        ),
                        (
                            IntegrationCapability::HostSkill,
                            false,
                            IntegrationCapabilityStatus::Selected,
                        ),
                    ],
                ),
                integration_provider(
                    IntegrationProvider::Trellis,
                    true,
                    false,
                    &[(
                        IntegrationCapability::SpecGuide,
                        true,
                        IntegrationCapabilityStatus::Pending,
                    )],
                ),
            ],
        },
    )
    .expect("integrations state should write");

    with_cwd(&project_root, || {
        let err = app::run(Cli {
            command: Command::Integrate(IntegrateCommand {
                action: IntegrateAction::Apply,
            }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect_err("trellis apply without .trellis should fail");

        assert_eq!(err.code(), "integration_apply_failed");
        let structured = app::render_structured_error("integrate.apply", &err);
        assert!(structured.contains("provider-not-detected"));
        assert!(structured.contains("\"provider\": \"trellis\""));
    });

    assert!(
        !project_root
            .join(".trellis/spec/guides/omv-versioning-guide.md")
            .exists()
    );
    cleanup_project_root(&project_root);
}

#[test]
fn integrate_apply_installs_trellis_finalize_boundary_managed_block() {
    let project_root = temp_project_root("integrate-apply-finalize-boundary");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");
    fs::create_dir_all(project_root.join(".trellis/spec/guides"))
        .expect("trellis guides dir should be created");
    fs::create_dir_all(project_root.join(".agents/skills/finish-work"))
        .expect("finish-work skill dir should be created");
    fs::write(
        project_root.join(".trellis/spec/guides/index.md"),
        "# Thinking Guides\n",
    )
    .expect("trellis index should write");
    fs::write(
        project_root.join(".agents/skills/finish-work/SKILL.md"),
        "# Finish Work\n\n## Checklist\n\n## Quick Check Flow\n\nbody\n",
    )
    .expect("finish-work surface should write");
    storage::integrations::save_integrations(
        &omv_root,
        &OmvIntegrations {
            schema_version: 1,
            providers: vec![
                integration_provider(
                    IntegrationProvider::Codex,
                    false,
                    false,
                    &[
                        (
                            IntegrationCapability::ProjectInstructions,
                            false,
                            IntegrationCapabilityStatus::Selected,
                        ),
                        (
                            IntegrationCapability::HostSkill,
                            false,
                            IntegrationCapabilityStatus::Selected,
                        ),
                    ],
                ),
                integration_provider(
                    IntegrationProvider::Trellis,
                    true,
                    true,
                    &[(
                        IntegrationCapability::FinalizeBoundary,
                        true,
                        IntegrationCapabilityStatus::Pending,
                    )],
                ),
            ],
        },
    )
    .expect("integrations state should write through storage");

    with_cwd(&project_root, || {
        let output = app::run(Cli {
            command: Command::Integrate(IntegrateCommand {
                action: IntegrateAction::Apply,
            }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("trellis finalize-boundary apply should succeed");

        assert!(output.message.contains("\"command\": \"integrate.apply\""));
        assert!(
            output
                .message
                .contains("\"capability\": \"finalize-boundary\"")
        );
        assert!(output.message.contains("\"status\": \"installed\""));
    });

    let finish_work = fs::read_to_string(project_root.join(".agents/skills/finish-work/SKILL.md"))
        .expect("finish-work surface should exist");
    assert!(finish_work.contains("OMV-MANAGED-BEGIN:spec-trellis-finalize-boundary-finish-work"));
    assert!(finish_work.contains("omv event finalize-boundary --provider trellis"));
    let block = finish_work
        .find("OMV Finalize Boundary")
        .expect("finalize block should exist");
    let quick = finish_work
        .find("## Quick Check Flow")
        .expect("quick check should exist");
    assert!(block < quick);

    let loaded = storage::integrations::load_integrations(&omv_root)
        .expect("integrations should round-trip through storage");
    let trellis = loaded
        .providers
        .iter()
        .find(|provider| provider.provider == IntegrationProvider::Trellis)
        .expect("trellis provider should persist");
    let finalize = trellis
        .capabilities
        .iter()
        .find(|capability| capability.capability == IntegrationCapability::FinalizeBoundary)
        .expect("finalize-boundary should persist");
    assert_eq!(finalize.status, IntegrationCapabilityStatus::Installed);

    cleanup_project_root(&project_root);
}

#[test]
fn integrate_apply_prefers_trellis_05_finish_work_surface() {
    let project_root = temp_project_root("integrate-apply-finalize-boundary-05");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");
    fs::create_dir_all(project_root.join(".trellis/spec/guides"))
        .expect("trellis guides dir should be created");
    fs::create_dir_all(project_root.join(".agents/skills/trellis-finish-work"))
        .expect("trellis 0.5 finish-work skill dir should be created");
    fs::write(
        project_root.join(".trellis/spec/guides/index.md"),
        "# Thinking Guides\n",
    )
    .expect("trellis index should write");
    fs::write(
        project_root.join(".agents/skills/trellis-finish-work/SKILL.md"),
        "# Trellis Finish Work\n\n## Checklist\n\n## Quick Check Flow\n\nbody\n",
    )
    .expect("trellis 0.5 finish-work surface should write");
    storage::integrations::save_integrations(
        &omv_root,
        &OmvIntegrations {
            schema_version: 1,
            providers: vec![integration_provider(
                IntegrationProvider::Trellis,
                true,
                true,
                &[(
                    IntegrationCapability::FinalizeBoundary,
                    true,
                    IntegrationCapabilityStatus::Pending,
                )],
            )],
        },
    )
    .expect("integrations state should write through storage");

    with_cwd(&project_root, || {
        let output = app::run(Cli {
            command: Command::Integrate(IntegrateCommand {
                action: IntegrateAction::Apply,
            }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("trellis 0.5 finalize-boundary apply should succeed");

        assert!(output.message.contains("\"status\": \"installed\""));
        assert!(
            output
                .message
                .contains(".agents/skills/trellis-finish-work/SKILL.md")
        );
    });

    assert_file_contains(
        &project_root.join(".agents/skills/trellis-finish-work/SKILL.md"),
        "OMV-MANAGED-BEGIN:spec-trellis-finalize-boundary-finish-work",
    );
    assert!(
        !project_root
            .join(".agents/skills/finish-work/SKILL.md")
            .exists()
    );

    cleanup_project_root(&project_root);
}

#[test]
fn integrate_status_warns_when_trellis_finalize_block_is_only_in_legacy_path() {
    let project_root = temp_project_root("integrate-status-finalize-boundary-mixed");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");
    fs::create_dir_all(project_root.join(".trellis/spec/guides"))
        .expect("trellis guides dir should be created");
    fs::create_dir_all(project_root.join(".agents/skills/finish-work"))
        .expect("legacy finish-work skill dir should be created");
    fs::create_dir_all(project_root.join(".agents/skills/trellis-finish-work"))
        .expect("trellis 0.5 finish-work skill dir should be created");
    fs::write(
        project_root.join(".trellis/spec/guides/index.md"),
        "# Thinking Guides\n",
    )
    .expect("trellis index should write");
    fs::write(
        project_root.join(".agents/skills/finish-work/SKILL.md"),
        "# Finish Work\n\n<!-- OMV-MANAGED-BEGIN:spec-trellis-finalize-boundary-finish-work -->\n## OMV Finalize Boundary\n\n- [ ] stale legacy guidance\n<!-- OMV-MANAGED-END:spec-trellis-finalize-boundary-finish-work -->\n",
    )
    .expect("legacy finish-work surface should write");
    fs::write(
        project_root.join(".agents/skills/trellis-finish-work/SKILL.md"),
        "# Trellis Finish Work\n\n## Checklist\n\n## Quick Check Flow\n\nbody\n",
    )
    .expect("trellis 0.5 finish-work surface should write");
    storage::integrations::save_integrations(
        &omv_root,
        &OmvIntegrations {
            schema_version: 1,
            providers: vec![integration_provider(
                IntegrationProvider::Trellis,
                true,
                true,
                &[(
                    IntegrationCapability::FinalizeBoundary,
                    true,
                    IntegrationCapabilityStatus::Installed,
                )],
            )],
        },
    )
    .expect("integrations state should write through storage");

    with_cwd(&project_root, || {
        let output = app::run(Cli {
            command: Command::Integrate(IntegrateCommand {
                action: IntegrateAction::Status,
            }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("status should report mixed migration state without mutating");

        assert!(output.message.contains("\"status\": \"pending\""));
        assert!(output.message.contains("trellis-finish-work-path-mismatch"));
        assert!(output.message.contains("omv integrate apply"));
    });

    let trellis_05 =
        fs::read_to_string(project_root.join(".agents/skills/trellis-finish-work/SKILL.md"))
            .expect("trellis 0.5 finish-work surface should exist");
    assert!(!trellis_05.contains("OMV-MANAGED-BEGIN"));

    cleanup_project_root(&project_root);
}

#[test]
fn integrate_apply_repairs_trellis_finalize_block_from_legacy_to_05_path() {
    let project_root = temp_project_root("integrate-apply-finalize-boundary-mixed");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");
    fs::create_dir_all(project_root.join(".trellis/spec/guides"))
        .expect("trellis guides dir should be created");
    fs::create_dir_all(project_root.join(".agents/skills/finish-work"))
        .expect("legacy finish-work skill dir should be created");
    fs::create_dir_all(project_root.join(".agents/skills/trellis-finish-work"))
        .expect("trellis 0.5 finish-work skill dir should be created");
    fs::write(
        project_root.join(".trellis/spec/guides/index.md"),
        "# Thinking Guides\n",
    )
    .expect("trellis index should write");
    fs::write(
        project_root.join(".agents/skills/finish-work/SKILL.md"),
        "# Finish Work\n\n<!-- OMV-MANAGED-BEGIN:spec-trellis-finalize-boundary-finish-work -->\n## OMV Finalize Boundary\n\n- [ ] stale legacy guidance\n<!-- OMV-MANAGED-END:spec-trellis-finalize-boundary-finish-work -->\n",
    )
    .expect("legacy finish-work surface should write");
    fs::write(
        project_root.join(".agents/skills/trellis-finish-work/SKILL.md"),
        "# Trellis Finish Work\n\n## Checklist\n\n## Quick Check Flow\n\nbody\n",
    )
    .expect("trellis 0.5 finish-work surface should write");
    storage::integrations::save_integrations(
        &omv_root,
        &OmvIntegrations {
            schema_version: 1,
            providers: vec![integration_provider(
                IntegrationProvider::Trellis,
                true,
                true,
                &[(
                    IntegrationCapability::FinalizeBoundary,
                    true,
                    IntegrationCapabilityStatus::Installed,
                )],
            )],
        },
    )
    .expect("integrations state should write through storage");

    with_cwd(&project_root, || {
        let output = app::run(Cli {
            command: Command::Integrate(IntegrateCommand {
                action: IntegrateAction::Apply,
            }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("apply should repair the active Trellis 0.5 finish-work surface");

        assert!(output.message.contains("\"status\": \"installed\""));
        assert!(
            output
                .message
                .contains(".agents/skills/trellis-finish-work/SKILL.md")
        );
    });

    assert_file_contains(
        &project_root.join(".agents/skills/trellis-finish-work/SKILL.md"),
        "OMV-MANAGED-BEGIN:spec-trellis-finalize-boundary-finish-work",
    );
    assert_file_contains(
        &project_root.join(".agents/skills/finish-work/SKILL.md"),
        "stale legacy guidance",
    );

    cleanup_project_root(&project_root);
}

#[test]
fn integrate_status_warns_when_trellis_finalize_block_is_only_in_backup() {
    let project_root = temp_project_root("integrate-status-finalize-boundary-backup");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");
    fs::create_dir_all(project_root.join(".trellis/spec/guides"))
        .expect("trellis guides dir should be created");
    fs::create_dir_all(project_root.join(".agents/skills/trellis-finish-work"))
        .expect("trellis 0.5 finish-work skill dir should be created");
    fs::write(
        project_root.join(".trellis/spec/guides/index.md"),
        "# Thinking Guides\n",
    )
    .expect("trellis index should write");
    fs::write(
        project_root.join(".agents/skills/trellis-finish-work/SKILL.md"),
        "# Trellis Finish Work\n\n## Checklist\n\n## Quick Check Flow\n\nbody\n",
    )
    .expect("trellis 0.5 finish-work surface should write");
    fs::write(
        project_root.join(".agents/skills/trellis-finish-work/SKILL.md.backup"),
        "# Trellis Finish Work\n\n<!-- OMV-MANAGED-BEGIN:spec-trellis-finalize-boundary-finish-work -->\n## OMV Finalize Boundary\n\n- [ ] stale backup guidance\n<!-- OMV-MANAGED-END:spec-trellis-finalize-boundary-finish-work -->\n",
    )
    .expect("trellis 0.5 finish-work backup should write");
    storage::integrations::save_integrations(
        &omv_root,
        &OmvIntegrations {
            schema_version: 1,
            providers: vec![integration_provider(
                IntegrationProvider::Trellis,
                true,
                true,
                &[(
                    IntegrationCapability::FinalizeBoundary,
                    true,
                    IntegrationCapabilityStatus::Installed,
                )],
            )],
        },
    )
    .expect("integrations state should write through storage");

    with_cwd(&project_root, || {
        let output = app::run(Cli {
            command: Command::Integrate(IntegrateCommand {
                action: IntegrateAction::Status,
            }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("status should report backup-only migration state");

        assert!(output.message.contains("\"status\": \"pending\""));
        assert!(output.message.contains("trellis-finish-work-path-mismatch"));
        assert!(
            output
                .message
                .contains(".agents/skills/trellis-finish-work/SKILL.md.backup")
        );
        assert!(output.message.contains("omv integrate apply"));
    });

    let trellis_05 =
        fs::read_to_string(project_root.join(".agents/skills/trellis-finish-work/SKILL.md"))
            .expect("trellis 0.5 finish-work surface should exist");
    assert!(!trellis_05.contains("OMV-MANAGED-BEGIN"));

    cleanup_project_root(&project_root);
}

#[test]
fn integrate_apply_refreshes_installed_capabilities() {
    let project_root = temp_project_root("integrate-apply-refresh-installed");
    let omv_root = project_root.join(".omv");
    fs::create_dir_all(&omv_root).expect(".omv root should be created");
    fs::create_dir_all(project_root.join(".trellis/spec/guides"))
        .expect("trellis guides dir should be created");
    fs::create_dir_all(project_root.join(".agents/skills/finish-work"))
        .expect("finish-work skill dir should be created");
    fs::write(
        project_root.join(".trellis/spec/guides/index.md"),
        "# Thinking Guides\n",
    )
    .expect("trellis index should write");
    fs::write(
        project_root.join(".agents/skills/finish-work/SKILL.md"),
        "# Finish Work\n\n## Checklist\n\n<!-- OMV-MANAGED-BEGIN:spec-trellis-finalize-boundary-finish-work -->\n## OMV Finalize Boundary\n\n- [ ] stale finalize guidance\n<!-- OMV-MANAGED-END:spec-trellis-finalize-boundary-finish-work -->\n\n## Quick Check Flow\n\nbody\n",
    )
    .expect("finish-work surface should write");
    fs::write(
        project_root.join("AGENTS.md"),
        "# Existing Instructions\n\n<!-- OMV-MANAGED-BEGIN:integration-codex-project-instructions -->\nstale codex guidance\n<!-- OMV-MANAGED-END:integration-codex-project-instructions -->\n\nKeep this host-owned section.\n",
    )
    .expect("agents file should write");
    fs::create_dir_all(project_root.join(".codex/skills/omv-versioning"))
        .expect("codex skill dir should be created");
    fs::write(
        project_root.join(".codex/skills/omv-versioning/SKILL.md"),
        "<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/codex/SKILL.md contract=1 -->\nstale skill\n",
    )
    .expect("codex skill should write");
    storage::integrations::save_integrations(
        &omv_root,
        &OmvIntegrations {
            schema_version: 1,
            providers: vec![
                integration_provider(
                    IntegrationProvider::Codex,
                    true,
                    true,
                    &[
                        (
                            IntegrationCapability::ProjectInstructions,
                            true,
                            IntegrationCapabilityStatus::Installed,
                        ),
                        (
                            IntegrationCapability::HostSkill,
                            true,
                            IntegrationCapabilityStatus::Installed,
                        ),
                    ],
                ),
                integration_provider(
                    IntegrationProvider::Trellis,
                    true,
                    true,
                    &[
                        (
                            IntegrationCapability::SpecGuide,
                            false,
                            IntegrationCapabilityStatus::Selected,
                        ),
                        (
                            IntegrationCapability::SpecIndexSnippet,
                            false,
                            IntegrationCapabilityStatus::Selected,
                        ),
                        (
                            IntegrationCapability::FinalizeBoundary,
                            true,
                            IntegrationCapabilityStatus::Installed,
                        ),
                    ],
                ),
            ],
        },
    )
    .expect("integrations state should write through storage");

    with_cwd(&project_root, || {
        let output = app::run(Cli {
            command: Command::Integrate(IntegrateCommand {
                action: IntegrateAction::Apply,
            }),
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
            output_mode: OutputMode::Json,
        })
        .expect("installed capabilities should refresh");

        assert!(output.message.contains("\"succeeded\": 3"));
        assert!(
            output
                .message
                .contains("\"capability\": \"project-instructions\"")
        );
        assert!(
            output
                .message
                .contains("\"capability\": \"finalize-boundary\"")
        );
    });

    let finish_work = fs::read_to_string(project_root.join(".agents/skills/finish-work/SKILL.md"))
        .expect("finish-work surface should exist");
    assert!(!finish_work.contains("stale finalize guidance"));
    assert!(finish_work.contains("omv sync --check --json"));
    assert!(finish_work.contains("do not write target files"));

    let agents =
        fs::read_to_string(project_root.join("AGENTS.md")).expect("agents file should exist");
    assert!(agents.contains("# Existing Instructions"));
    assert!(agents.contains("Keep this host-owned section."));
    assert!(!agents.contains("stale codex guidance"));
    assert!(agents.contains("OMV Codex Adapter"));

    let codex_skill =
        fs::read_to_string(project_root.join(".codex/skills/omv-versioning/SKILL.md"))
            .expect("codex skill should exist");
    assert!(!codex_skill.contains("stale skill"));
    assert!(codex_skill.starts_with("---\n"));
    assert!(codex_skill.contains("<!-- OMV-MANAGED-FILE"));

    cleanup_project_root(&project_root);
}

fn integration_provider(
    provider: IntegrationProvider,
    selected: bool,
    detected: bool,
    capabilities: &[(IntegrationCapability, bool, IntegrationCapabilityStatus)],
) -> OmvIntegrationProviderState {
    OmvIntegrationProviderState {
        provider,
        selected,
        detection: IntegrationDetectionSnapshot {
            detected,
            recommended: detected,
        },
        capabilities: capabilities
            .iter()
            .map(
                |(capability, selected, status)| OmvIntegrationCapabilityState {
                    capability: *capability,
                    selected: *selected,
                    status: *status,
                    failure: None,
                },
            )
            .collect(),
    }
}

fn target(
    id: &str,
    language: TargetLanguage,
    manifest_path: &str,
    runtime_export_path: &str,
) -> OmvTargetRecord {
    OmvTargetRecord {
        id: id.to_owned(),
        language,
        root: ".".to_owned(),
        manifest_path: manifest_path.to_owned(),
        runtime_export_path: runtime_export_path.to_owned(),
        strategy: PreProjectStrategy::IntentOnly,
        enabled: true,
    }
}

fn v2_targets() -> Vec<OmvV2TargetRecord> {
    vec![
        OmvV2TargetRecord {
            id: "root-version-file".to_owned(),
            kind: TargetKind::TextScalar,
            adapter: "text".to_owned(),
            root: ".".to_owned(),
            enabled: true,
            mode: TargetMode::Write,
            config: OmvV2TargetConfig::TextScalar(TextScalarTarget {
                path: "VERSION".to_owned(),
                selector: "whole-file".to_owned(),
                template: "{version}\n".to_owned(),
            }),
        },
        OmvV2TargetRecord {
            id: "readme-version-badge".to_owned(),
            kind: TargetKind::RegexReplace,
            adapter: "markdown".to_owned(),
            root: ".".to_owned(),
            enabled: true,
            mode: TargetMode::Write,
            config: OmvV2TargetConfig::RegexReplace(RegexReplaceTarget {
                path: "README.md".to_owned(),
                pattern: "version-[0-9]+\\.[0-9]+\\.[0-9]+-blue".to_owned(),
                template: "version-{version}-blue".to_owned(),
                allow_multiple: false,
            }),
        },
        OmvV2TargetRecord {
            id: "readme-managed-version".to_owned(),
            kind: TargetKind::MarkdownManagedBlock,
            adapter: "markdown".to_owned(),
            root: ".".to_owned(),
            enabled: true,
            mode: TargetMode::Write,
            config: OmvV2TargetConfig::MarkdownManagedBlock(MarkdownManagedBlockTarget {
                path: "docs/version.md".to_owned(),
                begin_marker: "<!-- OMV:BEGIN version -->".to_owned(),
                end_marker: "<!-- OMV:END version -->".to_owned(),
                template: "Managed version: {version}".to_owned(),
            }),
        },
        OmvV2TargetRecord {
            id: "component-manifest".to_owned(),
            kind: TargetKind::YamlScalar,
            adapter: "yaml".to_owned(),
            root: ".".to_owned(),
            enabled: true,
            mode: TargetMode::Write,
            config: OmvV2TargetConfig::YamlScalar(YamlScalarTarget {
                path: "component.yml".to_owned(),
                key: "package.version".to_owned(),
                template: "{version}".to_owned(),
            }),
        },
        OmvV2TargetRecord {
            id: "public-header-version".to_owned(),
            kind: TargetKind::CHeaderMacro,
            adapter: "c-header".to_owned(),
            root: ".".to_owned(),
            enabled: true,
            mode: TargetMode::Write,
            config: OmvV2TargetConfig::CHeaderMacro(CHeaderMacroTarget {
                path: "include/example_version.h".to_owned(),
                macro_name: "EXAMPLE_VERSION".to_owned(),
                template: "\"{version}\"".to_owned(),
            }),
        },
        OmvV2TargetRecord {
            id: "rust-workspace".to_owned(),
            kind: TargetKind::CargoWorkspace,
            adapter: "cargo".to_owned(),
            root: "tools/example".to_owned(),
            enabled: true,
            mode: TargetMode::Write,
            config: OmvV2TargetConfig::CargoWorkspace(CargoWorkspaceTarget {
                root: "tools/example".to_owned(),
                members: CargoMembers::All,
                version_policy: CargoVersionPolicy::Same,
                version_location: CargoVersionLocation::MemberPackages,
                lockfile: CargoLockfileStrategy::Update,
            }),
        },
    ]
}

fn seed_v2_files(project_root: &Path) {
    fs::create_dir_all(project_root.join("include")).expect("include dir should exist");
    fs::create_dir_all(project_root.join("docs")).expect("docs dir should exist");
    fs::create_dir_all(project_root.join("tools/example/crates/a"))
        .expect("cargo member dir should exist");
    fs::write(project_root.join("VERSION"), "0.0.1\n").expect("version file should write");
    fs::write(
        project_root.join("README.md"),
        "# Example\n\n![version](https://img.shields.io/badge/version-0.0.1-blue)\n",
    )
    .expect("readme should write");
    fs::write(
        project_root.join("docs/version.md"),
        "<!-- OMV:BEGIN version -->\nold\n<!-- OMV:END version -->\n",
    )
    .expect("managed docs should write");
    fs::write(
        project_root.join("component.yml"),
        "package:\n  version: 0.0.1\n",
    )
    .expect("yaml should write");
    fs::write(
        project_root.join("include/example_version.h"),
        "#pragma once\n#define EXAMPLE_VERSION \"0.0.1\"\n",
    )
    .expect("header should write");
    fs::write(
        project_root.join("tools/example/Cargo.toml"),
        "[workspace]\nmembers = [\"crates/a\"]\n",
    )
    .expect("workspace manifest should write");
    fs::write(
        project_root.join("tools/example/crates/a/Cargo.toml"),
        "[package]\nname = \"example-a\"\nversion = \"0.0.1\"\nedition = \"2024\"\n",
    )
    .expect("member manifest should write");
    fs::write(
        project_root.join("tools/example/Cargo.lock"),
        "[[package]]\nname = \"example-a\"\nversion = \"0.0.1\"\n",
    )
    .expect("lockfile should write");
}

fn with_cwd<T>(cwd: &Path, run: impl FnOnce() -> T) -> T {
    let lock = CWD_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().expect("cwd lock should not be poisoned");

    let previous = std::env::current_dir().expect("current dir should resolve");
    std::env::set_current_dir(cwd).expect("set_current_dir should succeed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));
    std::env::set_current_dir(previous).expect("restore current dir should succeed");

    match result {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

fn assert_file_contains(path: &Path, needle: &str) {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("{} should exist: {err}", path.display()));
    assert!(
        content.contains(needle),
        "{} should contain `{}` but was: {}",
        path.display(),
        needle,
        content
    );
}

fn temp_project_root(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("omv-int-{prefix}-{stamp}"));
    fs::create_dir_all(&root).expect("temp project root should be created");
    root
}

fn cleanup_project_root(root: &Path) {
    let _ = fs::remove_dir_all(root);
}
