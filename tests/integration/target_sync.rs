use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use omv::app;
use omv::cli::{Cli, Command};
use omv::core::date::LogicalDate;
use omv::core::locale::OperatorLocale;
use omv::core::schema::{OmvConfig, OmvState, OmvTargetRecord, OmvTargets};
use omv::core::target::{PreProjectStrategy, TargetLanguage};
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
        },
    )
    .expect("targets should save");

    with_cwd(&project_root, || {
        let output = app::run(Cli {
            command: Command::Sync,
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
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
        },
    )
    .expect("targets should save");

    with_cwd(&project_root, || {
        app::run(Cli {
            command: Command::Bump,
            locale_override: Some("en-US".to_owned()),
            ntp_override: None,
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
