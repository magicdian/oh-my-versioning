use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use omv::app::{AppRuntime, run_with_runtime};
use omv::cli::{Cli, Command as OmvCommand, OutputMode};
use omv::core::date::LogicalDate;
use omv::core::time::{LastTimeSource, TimeSource};
use omv::errors::OmvError;
use toml::Value;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static SCENARIO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug)]
struct Scenario {
    id: String,
    repo: String,
    tag: String,
    commit: String,
    expected_version: String,
    expected_drift: usize,
    expected_synced: usize,
    expected_ok: usize,
    assertions: Vec<ContainsAssertion>,
}

#[derive(Debug)]
struct ContainsAssertion {
    path: String,
    text: String,
}

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
#[ignore = "external scenario test clones pinned fixture repositories"]
fn wiremux_2604_30_3_syncs_declared_version_surfaces() {
    let _guard = scenario_guard();
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = repo_root.join("tests/external_scenarios/wiremux-2604.30.3");
    let scenario = load_scenario(&fixture_dir.join("scenario.toml"));
    let run_root = prepare_run_root(&repo_root, &scenario);

    let result = run_scenario(&repo_root, &fixture_dir, &scenario, &run_root);
    let keep = std::env::var("OMV_EXTERNAL_KEEP").ok().as_deref() == Some("1");

    match result {
        Ok(()) if keep => {
            eprintln!(
                "preserved external scenario workspace: {}",
                run_root.display()
            );
        }
        Ok(()) => {
            fs::remove_dir_all(&run_root)
                .unwrap_or_else(|err| panic!("failed to clean {}: {err}", run_root.display()));
        }
        Err(message) => {
            eprintln!(
                "preserved failed external scenario workspace: {}",
                run_root.display()
            );
            panic!("{message}");
        }
    }
}

#[test]
#[ignore = "external scenario test clones pinned fixture repositories"]
fn wiremux_2604_30_3_bumps_same_day_twice_then_resets_next_day() {
    let _guard = scenario_guard();
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = repo_root.join("tests/external_scenarios/wiremux-2604.30.3");
    let scenario = load_scenario(&fixture_dir.join("scenario.toml"));
    let run_root = prepare_named_run_root(&repo_root, &scenario, "bump-rollover");

    let result = run_bump_rollover_scenario(&repo_root, &fixture_dir, &scenario, &run_root);
    let keep = std::env::var("OMV_EXTERNAL_KEEP").ok().as_deref() == Some("1");

    match result {
        Ok(()) if keep => {
            eprintln!(
                "preserved external scenario workspace: {}",
                run_root.display()
            );
        }
        Ok(()) => {
            fs::remove_dir_all(&run_root)
                .unwrap_or_else(|err| panic!("failed to clean {}: {err}", run_root.display()));
        }
        Err(message) => {
            eprintln!(
                "preserved failed external scenario workspace: {}",
                run_root.display()
            );
            panic!("{message}");
        }
    }
}

fn run_scenario(
    repo_root: &Path,
    fixture_dir: &Path,
    scenario: &Scenario,
    run_root: &Path,
) -> Result<(), String> {
    eprintln!("external scenario: {}", scenario.id);
    eprintln!("  repo: {}", scenario.repo);
    eprintln!("  tag: {}", scenario.tag);
    eprintln!("  commit: {}", scenario.commit);
    eprintln!("  expected version: {}", scenario.expected_version);

    let source_cache = step("source cache ready", || {
        ensure_source_cache(repo_root, scenario)
    })?;
    eprintln!("  source cache: {}", source_cache.display());
    step("isolated worktree cloned", || {
        clone_worktree(&source_cache, run_root)
    })?;
    eprintln!("  worktree: {}", run_root.display());
    step("checked-out commit matches scenario pin", || {
        assert_git_commit(run_root, &scenario.commit)
    })?;
    step(".omv fixture overlaid", || {
        overlay_omv_fixture(fixture_dir, run_root)
    })?;

    let omv = std::env::var_os("CARGO_BIN_EXE_omv")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join("target/debug/omv"));
    if !omv.exists() {
        return Err(format!("OMV binary does not exist at {}", omv.display()));
    }

    let plan = step("omv plan --json", || {
        run_omv(&omv, run_root, &["plan", "--json"])
    })?;
    step("plan reports expected drift count", || {
        assert_stdout_contains(&plan, "\"command\": \"plan\"")?;
        assert_stdout_contains(&plan, &format!("\"drift\": {}", scenario.expected_drift))
    })?;

    let sync = step("omv sync --json", || {
        run_omv(&omv, run_root, &["sync", "--json"])
    })?;
    step("sync reports expected version and target count", || {
        assert_stdout_contains(&sync, "\"command\": \"sync\"")?;
        assert_stdout_contains(
            &sync,
            &format!("\"version\": \"{}\"", scenario.expected_version),
        )?;
        assert_stdout_contains(&sync, &format!("\"synced\": {}", scenario.expected_synced))
    })?;

    let updated_files = step("tracked file diff collected after sync", || {
        git_diff_name_only(run_root)
    })?;

    let check = step("omv sync --check --json", || {
        run_omv(&omv, run_root, &["sync", "--check", "--json"])
    })?;
    step("sync check reports clean target state", || {
        assert_stdout_contains(&check, "\"command\": \"sync.check\"")?;
        assert_stdout_contains(&check, &format!("\"ok\": {}", scenario.expected_ok))?;
        assert_stdout_contains(&check, "\"drift\": 0")
    })?;

    for assertion in &scenario.assertions {
        let label = format!("assert {}", assertion.path);
        step(&label, || assert_file_contains(run_root, assertion))?;
    }

    eprintln!("updated tracked files:");
    for path in &updated_files {
        eprintln!("  - {path}");
    }
    eprintln!(
        "summary: {} assertions passed; {} tracked files updated",
        scenario.assertions.len(),
        updated_files.len()
    );

    Ok(())
}

fn run_bump_rollover_scenario(
    repo_root: &Path,
    fixture_dir: &Path,
    scenario: &Scenario,
    run_root: &Path,
) -> Result<(), String> {
    eprintln!("external scenario: {} bump rollover", scenario.id);
    eprintln!("  repo: {}", scenario.repo);
    eprintln!("  tag: {}", scenario.tag);
    eprintln!("  commit: {}", scenario.commit);
    eprintln!("  initial version: 2604.30.3");
    eprintln!("  expected versions: 2604.30.4 -> 2604.30.5 -> 2605.1.1");

    let source_cache = step("source cache ready", || {
        ensure_source_cache(repo_root, scenario)
    })?;
    eprintln!("  source cache: {}", source_cache.display());
    step("isolated worktree cloned", || {
        clone_worktree(&source_cache, run_root)
    })?;
    eprintln!("  worktree: {}", run_root.display());
    step("checked-out commit matches scenario pin", || {
        assert_git_commit(run_root, &scenario.commit)
    })?;
    step(".omv fixture overlaid", || {
        overlay_omv_fixture(fixture_dir, run_root)
    })?;
    step("bump initial state seeded", || seed_bump_state(run_root))?;

    step("same-day bump 1 updates to 2604.30.4", || {
        run_bump_with_date(run_root, "2026-04-30", "2604.30.4")?;
        assert_file_contains_text(run_root, "VERSION", "2604.30.4")
    })?;
    step("same-day bump 2 updates to 2604.30.5", || {
        run_bump_with_date(run_root, "2026-04-30", "2604.30.5")?;
        assert_file_contains_text(run_root, "VERSION", "2604.30.5")
    })?;
    step("next-day bump resets to 2605.1.1", || {
        run_bump_with_date(run_root, "2026-05-01", &scenario.expected_version)?;
        assert_file_contains_text(run_root, "VERSION", &scenario.expected_version)
    })?;

    let omv = std::env::var_os("CARGO_BIN_EXE_omv")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join("target/debug/omv"));
    let check = step("omv sync --check --json after bump rollover", || {
        run_omv(&omv, run_root, &["sync", "--check", "--json"])
    })?;
    step(
        "sync check reports clean target state after rollover",
        || {
            assert_stdout_contains(&check, "\"command\": \"sync.check\"")?;
            assert_stdout_contains(&check, &format!("\"ok\": {}", scenario.expected_ok))?;
            assert_stdout_contains(&check, "\"drift\": 0")
        },
    )?;

    for assertion in &scenario.assertions {
        let label = format!("assert final {}", assertion.path);
        step(&label, || assert_file_contains(run_root, assertion))?;
    }

    let updated_files = step("tracked file diff collected after bump rollover", || {
        git_diff_name_only(run_root)
    })?;
    eprintln!("updated tracked files after bump rollover:");
    for path in &updated_files {
        eprintln!("  - {path}");
    }
    eprintln!(
        "summary: 3 bumps passed; {} final assertions passed; {} tracked files updated",
        scenario.assertions.len(),
        updated_files.len()
    );

    Ok(())
}

fn load_scenario(path: &Path) -> Scenario {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    let value = content
        .parse::<Value>()
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()));
    let table = value
        .as_table()
        .unwrap_or_else(|| panic!("{} root must be a TOML table", path.display()));

    let assertions = table
        .get("assertions")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{} must contain assertions array", path.display()))
        .iter()
        .map(|item| {
            let table = item
                .as_table()
                .unwrap_or_else(|| panic!("assertion entry must be a table"));
            ContainsAssertion {
                path: required_string(table, "path"),
                text: required_string(table, "text"),
            }
        })
        .collect();

    Scenario {
        id: required_string(table, "id"),
        repo: required_string(table, "repo"),
        tag: required_string(table, "tag"),
        commit: required_string(table, "commit"),
        expected_version: required_string(table, "expected_version"),
        expected_drift: required_usize(table, "expected_drift"),
        expected_synced: required_usize(table, "expected_synced"),
        expected_ok: required_usize(table, "expected_ok"),
        assertions,
    }
}

fn required_string(table: &toml::map::Map<String, Value>, key: &str) -> String {
    table
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("missing string field: {key}"))
        .to_owned()
}

fn required_usize(table: &toml::map::Map<String, Value>, key: &str) -> usize {
    let value = table
        .get(key)
        .and_then(Value::as_integer)
        .unwrap_or_else(|| panic!("missing integer field: {key}"));
    usize::try_from(value).unwrap_or_else(|_| panic!("field {key} must be non-negative"))
}

fn step<T>(label: &str, run: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    match run() {
        Ok(value) => {
            eprintln!("[PASS] {label}");
            Ok(value)
        }
        Err(err) => {
            eprintln!("[FAIL] {label}");
            Err(format!("{label}: {err}"))
        }
    }
}

fn prepare_run_root(repo_root: &Path, scenario: &Scenario) -> PathBuf {
    prepare_named_run_root(repo_root, scenario, "sync")
}

fn prepare_named_run_root(repo_root: &Path, scenario: &Scenario, name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    repo_root
        .join("target/external-scenarios/runs")
        .join(format!("{}-{name}-{stamp}", scenario.id))
}

fn ensure_source_cache(repo_root: &Path, scenario: &Scenario) -> Result<PathBuf, String> {
    let cache = repo_root
        .join("target/external-scenarios/source-cache")
        .join(&scenario.id);
    if cache.exists() {
        assert_git_commit(&cache, &scenario.commit)?;
        return Ok(cache);
    }

    if let Some(parent) = cache.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }

    let output = Command::new("git")
        .args(["clone", "--depth", "1", "--branch", &scenario.tag])
        .arg(&scenario.repo)
        .arg(&cache)
        .output()
        .map_err(|err| format!("failed to spawn git clone: {err}"))?;
    if !output.status.success() {
        return Err(format_output("git clone", &output));
    }
    assert_git_commit(&cache, &scenario.commit)?;
    Ok(cache)
}

fn clone_worktree(source_cache: &Path, run_root: &Path) -> Result<(), String> {
    if run_root.exists() {
        fs::remove_dir_all(run_root)
            .map_err(|err| format!("failed to remove {}: {err}", run_root.display()))?;
    }
    if let Some(parent) = run_root.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }

    let output = Command::new("git")
        .arg("clone")
        .arg(source_cache)
        .arg(run_root)
        .output()
        .map_err(|err| format!("failed to spawn git clone from cache: {err}"))?;
    if !output.status.success() {
        return Err(format_output("git clone from cache", &output));
    }
    Ok(())
}

fn assert_git_commit(repo: &Path, expected: &str) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|err| format!("failed to spawn git rev-parse: {err}"))?;
    if !output.status.success() {
        return Err(format_output("git rev-parse HEAD", &output));
    }
    let actual = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if actual != expected {
        return Err(format!(
            "{} resolved to commit {actual}, expected {expected}",
            repo.display()
        ));
    }
    Ok(())
}

fn overlay_omv_fixture(fixture_dir: &Path, run_root: &Path) -> Result<(), String> {
    let source = fixture_dir.join("omv");
    let target = run_root.join(".omv");
    fs::create_dir_all(&target)
        .map_err(|err| format!("failed to create {}: {err}", target.display()))?;

    for file_name in ["config.toml", "state.toml", "targets.toml"] {
        fs::copy(source.join(file_name), target.join(file_name)).map_err(|err| {
            format!(
                "failed to copy {} to {}: {err}",
                source.join(file_name).display(),
                target.join(file_name).display()
            )
        })?;
    }
    Ok(())
}

fn seed_bump_state(run_root: &Path) -> Result<(), String> {
    let omv_root = run_root.join(".omv");
    fs::write(
        omv_root.join("config.toml"),
        r#"locale = "en-US"
timezone = "UTC+8"
project_profile = "oss"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = true
"#,
    )
    .map_err(|err| format!("failed to write bump config: {err}"))?;
    fs::write(
        omv_root.join("state.toml"),
        r#"logical_date = "2026-04-30"
build_number = 3
last_issued_version = "2604.30.3"
last_time_source = "ntp"
"#,
    )
    .map_err(|err| format!("failed to write bump state: {err}"))?;
    Ok(())
}

fn run_bump_with_date(run_root: &Path, date: &str, expected_version: &str) -> Result<(), String> {
    let logical_date =
        LogicalDate::parse_iso(date).map_err(|err| format!("invalid test date {date}: {err}"))?;
    let ntp = FixedSource {
        source: LastTimeSource::Ntp,
        date: logical_date,
    };
    let system = FixedSource {
        source: LastTimeSource::System,
        date: logical_date,
    };
    let runtime = AppRuntime {
        ntp_source: &ntp,
        system_source: &system,
    };
    let output = with_cwd(run_root, || {
        run_with_runtime(
            Cli {
                command: OmvCommand::Bump,
                locale_override: Some("en-US".to_owned()),
                ntp_override: None,
                output_mode: OutputMode::Json,
            },
            &runtime,
        )
    })
    .map_err(|err| err.to_string())?;
    let expected = format!("\"version\": \"{expected_version}\"");
    if output.message.contains(&expected) {
        Ok(())
    } else {
        Err(format!(
            "bump output did not contain {expected}\n{}",
            output.message
        ))
    }
}

fn git_diff_name_only(repo: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["diff", "--name-only"])
        .output()
        .map_err(|err| format!("failed to spawn git diff: {err}"))?;
    if !output.status.success() {
        return Err(format_output("git diff --name-only", &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

fn assert_file_contains(run_root: &Path, assertion: &ContainsAssertion) -> Result<(), String> {
    assert_file_contains_text(run_root, &assertion.path, &assertion.text)
}

fn assert_file_contains_text(
    run_root: &Path,
    relative_path: &str,
    expected: &str,
) -> Result<(), String> {
    let path = run_root.join(relative_path);
    let content = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read assertion file {}: {err}", path.display()))?;
    if content.contains(expected) {
        Ok(())
    } else {
        Err(format!(
            "{relative_path} did not contain expected text `{expected}`"
        ))
    }
}

fn with_cwd<T>(cwd: &Path, run: impl FnOnce() -> T) -> T {
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

fn scenario_guard() -> MutexGuard<'static, ()> {
    SCENARIO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn run_omv(omv: &Path, cwd: &Path, args: &[&str]) -> Result<Output, String> {
    let output = Command::new(omv)
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|err| format!("failed to spawn {} {args:?}: {err}", omv.display()))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format_output(&format!("omv {}", args.join(" ")), &output))
    }
}

fn assert_stdout_contains(output: &Output, expected: &str) -> Result<(), String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains(expected) {
        Ok(())
    } else {
        Err(format!(
            "stdout did not contain `{expected}`\nstdout:\n{stdout}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn format_output(label: &str, output: &Output) -> String {
    format!(
        "{label} failed with status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
