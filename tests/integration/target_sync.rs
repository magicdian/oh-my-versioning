use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use omv::app;
use omv::cli::{Cli, Command, OutputMode, SyncCommand};
use omv::core::date::LogicalDate;
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
