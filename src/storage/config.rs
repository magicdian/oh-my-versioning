use std::fs;
use std::path::{Path, PathBuf};

use crate::core::locale::OperatorLocale;
use crate::core::schema::OmvConfig;
use crate::core::target::ProjectProfile;
use crate::core::versioning::capability;
use crate::core::versioning::{
    BuildPolicy, GateOutcome, ReleaseMode, VersionOutput, gate_declared_requirement,
};
use crate::errors::{ConfigError, OmvError};

use super::{CONFIG_FILE, atomic};

pub fn path_for(root: &Path) -> PathBuf {
    root.join(CONFIG_FILE)
}

pub fn load_config(root: &Path) -> Result<OmvConfig, OmvError> {
    let path = path_for(root);
    let content = fs::read_to_string(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ConfigError::Missing { path: path.clone() }
        } else {
            ConfigError::Parse {
                path: path.clone(),
                reason: err.to_string(),
            }
        }
    })?;

    let assignments = parse_assignments(&content);
    ensure_required_omv(&assignments)?;
    let declared_assignments = assignments.clone();

    let mut config = OmvConfig::default();
    for (key, value) in assignments {
        match key.as_str() {
            "schema_version" => {
                config.schema_version = value.parse::<u32>().map_err(|e| ConfigError::Parse {
                    path: path.clone(),
                    reason: format!("invalid schema_version: {e}"),
                })?;
            }
            // Checked up front by `ensure_required_omv`; kept out of the
            // typed config because it describes the config, not the project.
            "requires_omv" => {}
            "locale" => {
                if !OperatorLocale::is_supported(&value) {
                    return Err(ConfigError::InvalidLocale(value).into());
                }
                config.locale = OperatorLocale::from_input(&value);
            }
            "timezone" => config.timezone = value,
            "project_profile" => {
                config.project_profile =
                    ProjectProfile::parse(&value).ok_or_else(|| ConfigError::Parse {
                        path: path.clone(),
                        reason: format!("invalid project_profile: {value}"),
                    })?
            }
            "release_mode" => {
                config.release_mode = ReleaseMode::parse(&value)
                    .ok_or_else(|| ConfigError::InvalidReleaseMode(value.clone()))?
            }
            "version_output" => {
                config.version_output =
                    VersionOutput::parse(&value).ok_or_else(|| ConfigError::Parse {
                        path: path.clone(),
                        reason: format!("invalid version_output: {value}"),
                    })?
            }
            "build_policy" => {
                config.build_policy = BuildPolicy::parse(&value)
                    .ok_or_else(|| ConfigError::InvalidBuildPolicy(value.clone()))?
            }
            "ntp_enabled" => {
                config.ntp_enabled = value.parse::<bool>().map_err(|e| ConfigError::Parse {
                    path: path.clone(),
                    reason: format!("invalid ntp_enabled: {e}"),
                })?
            }
            _ => {}
        }
    }

    ensure_declaration_matches(&config, &declared_assignments)?;

    Ok(config)
}

pub fn save_config(root: &Path, config: &OmvConfig) -> Result<(), OmvError> {
    let path = path_for(root);
    let content = format!(
        "schema_version = {}\nrequires_omv = \"{}\"\nlocale = \"{}\"\ntimezone = \"{}\"\nproject_profile = \"{}\"\nrelease_mode = \"{}\"\nversion_output = \"{}\"\nbuild_policy = \"{}\"\nntp_enabled = {}\n",
        config.schema_version,
        crate::core::versioning::required_omv_for(config),
        config.locale.as_str(),
        config.timezone,
        config.project_profile.as_str(),
        config.release_mode.as_str(),
        config.version_output.as_str(),
        config.build_policy.as_str(),
        config.ntp_enabled,
    );

    atomic::write_atomically(&path, content.as_bytes())
}

/// Rejects a config whose `requires_omv` outranks the running binary.
///
/// `requires_omv` is the one check a binary can perform without having compiled
/// in knowledge of the newer value: the requirement lives in the file. It cannot
/// protect a binary that predates the key (that binary ignores unknown keys), but
/// it does protect every release from the one that introduced it onwards -- which
/// is what stops a machine silently falling back to an older `omv` on a newer
/// project.
///
/// A malformed value is rejected here rather than treated as "no requirement".
/// Fail-open would let a typo disable the check entirely, and this key is the
/// boundary that decides whether the file may be read at all.
fn ensure_required_omv(assignments: &[(String, String)]) -> Result<(), OmvError> {
    let Some(declared) = declared_requirement(assignments) else {
        // Absent declarations are the documented legacy path; nothing to check.
        return Ok(());
    };

    if capability::is_malformed_version(declared) {
        return Err(ConfigError::MalformedRequirement(declared.to_owned()).into());
    }

    let release_mode = assignments
        .iter()
        .find(|(key, _)| key == "release_mode")
        .map(|(_, value)| value.as_str());
    let culprit = release_mode.map(|value| ("release_mode", value));

    match gate_declared_requirement(culprit, Some(declared)) {
        GateOutcome::Supported => Ok(()),
        GateOutcome::UpgradeRequired(detail) => {
            Err(ConfigError::UnsupportedCapability(detail).into())
        }
    }
}

/// Rejects a declaration that is unparsable or weaker than the config's values.
///
/// `required_omv_for` lists gated fields by hand, so it can miss a field when one
/// is added. A declaration weaker than the config's real needs would let a binary
/// that lacks the newer value read a file it cannot honour, so it is rejected
/// rather than trusted. This catches drift observable to *this* binary; a field
/// gated by a future release is invisible until that release ships.
fn ensure_declaration_matches(
    config: &OmvConfig,
    assignments: &[(String, String)],
) -> Result<(), OmvError> {
    let Some(declared) = declared_requirement(assignments) else {
        return Ok(());
    };

    if capability::is_malformed_version(declared) {
        return Err(ConfigError::MalformedRequirement(declared.to_owned()).into());
    }

    let actual = crate::core::versioning::required_omv_for(config);
    if capability::requires_upgrade(declared, actual) {
        return Err(ConfigError::DeclarationMismatch {
            declared: declared.to_owned(),
            actual: actual.to_owned(),
        }
        .into());
    }

    Ok(())
}

/// Brings `requires_omv` in line with the config's values.
///
/// Called from the paths that establish a project's desired state (`omv init`
/// and `omv integrate apply`). Without this, a project created before the key
/// existed would never gain it, and the declaration would stay absent forever --
/// which is indistinguishable from "no requirement" to every reader.
///
/// Returns `true` when the file was rewritten.
pub fn reconcile_requirement(root: &Path) -> Result<bool, OmvError> {
    let path = path_for(root);
    if !path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&path).map_err(|err| ConfigError::Parse {
        path: path.clone(),
        reason: err.to_string(),
    })?;
    let assignments = parse_assignments(&content);
    let declared = declared_requirement(&assignments).map(str::to_owned);
    let config = load_config(root)?;
    let actual = crate::core::versioning::required_omv_for(&config);

    let needs_write = match declared.as_deref() {
        None => true,
        Some(declared) => declared != actual,
    };
    if !needs_write {
        return Ok(false);
    }

    save_config(root, &config)?;
    Ok(true)
}

fn declared_requirement(assignments: &[(String, String)]) -> Option<&str> {
    assignments
        .iter()
        .find(|(key, _)| key == REQUIRE_RELEASE_KEY)
        .map(|(_, value)| value.as_str())
}

/// Config key declaring the minimum `omv` release that can honour the file.
const REQUIRE_RELEASE_KEY: &str = "requires_omv";

fn parse_assignments(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let key = key.trim().to_owned();
            let raw = value.trim();
            let value = raw.trim_matches('"').to_owned();
            Some((key, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::core::locale::OperatorLocale;
    use crate::core::schema::OmvConfig;
    use crate::core::target::ProjectProfile;
    use crate::core::versioning::{BuildPolicy, ReleaseMode, VersionOutput};
    use crate::errors::{ConfigError, OmvError};

    use super::{load_config, path_for, reconcile_requirement, save_config};

    #[test]
    fn config_round_trip_preserves_all_fields() {
        let root = temp_omv_root("config-roundtrip");

        let config = OmvConfig {
            schema_version: 1,
            locale: OperatorLocale::ZhCn,
            timezone: "UTC+8".to_owned(),
            project_profile: ProjectProfile::Oss,
            release_mode: ReleaseMode::Pipeline,
            version_output: VersionOutput::Semver,
            build_policy: BuildPolicy::Continuous,
            ntp_enabled: false,
        };

        save_config(&root, &config).expect("config should save");
        let loaded = load_config(&root).expect("config should load");
        assert_eq!(loaded, config);

        cleanup_root(&root);
    }

    #[test]
    fn config_rejects_unsupported_locale_value() {
        let root = temp_omv_root("config-invalid-locale");
        let path = path_for(&root);
        let raw = r#"schema_version = 1
locale = "fr-FR"
timezone = "UTC+0"
project_profile = "personal"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = true
"#;
        fs::write(&path, raw).expect("should write config fixture");

        let err = load_config(&root).expect_err("invalid locale must fail");
        match err {
            OmvError::Config(ConfigError::InvalidLocale(locale)) => assert_eq!(locale, "fr-FR"),
            other => panic!("unexpected error variant: {other}"),
        }

        cleanup_root(&root);
    }

    #[test]
    fn config_defaults_release_mode_to_local_when_absent() {
        let root = temp_omv_root("config-release-mode-absent");
        let path = path_for(&root);
        let raw = r#"schema_version = 1
locale = "en-US"
timezone = "UTC+0"
project_profile = "personal"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = true
"#;
        fs::write(&path, raw).expect("should write config fixture");

        let loaded = load_config(&root).expect("config should load");
        assert_eq!(loaded.release_mode, ReleaseMode::Local);

        cleanup_root(&root);
    }

    #[test]
    fn config_rejects_invalid_release_mode() {
        let root = temp_omv_root("config-invalid-release-mode");
        let path = path_for(&root);
        let raw = r#"schema_version = 1
locale = "en-US"
timezone = "UTC+0"
project_profile = "personal"
release_mode = "sometimes"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = true
"#;
        fs::write(&path, raw).expect("should write config fixture");

        let err = load_config(&root).expect_err("invalid release mode must fail");
        match err {
            OmvError::Config(ConfigError::InvalidReleaseMode(mode)) => {
                assert_eq!(mode, "sometimes")
            }
            other => panic!("unexpected error variant: {other}"),
        }

        cleanup_root(&root);
    }

    #[test]
    fn config_accepts_every_release_mode_this_binary_ships() {
        let root = temp_omv_root("config-release-mode-supported");
        let path = path_for(&root);
        // Both shipped values must load; a gate that rejects its own table
        // would make the feature unusable.
        for mode in ["local", "pipeline"] {
        let raw = format!(
            r#"schema_version = 1
requires_omv = "2611.2.1"
locale = "en-US"
timezone = "UTC+0"
project_profile = "personal"
release_mode = "{mode}"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = false
"#
        );
            fs::write(&path, raw).expect("should write config fixture");

            let loaded = load_config(&root).expect("supported release mode should load");
            assert_eq!(loaded.release_mode.as_str(), mode);
        }

        cleanup_root(&root);
    }

    #[test]
    fn config_reports_upgrade_requirement_with_a_stable_code() {
        let error = OmvError::Config(ConfigError::UnsupportedCapability(
            crate::core::versioning::capability::unsupported(
                "release_mode",
                Some("pipeline"),
                "9999.1.1",
                "2610.27.2",
            ),
        ));

        assert_eq!(error.code(), "unsupported_config_capability");
        let message = error.to_string();
        assert!(message.contains("9999.1.1"));
        assert!(message.contains("2610.27.2"));
        assert!(message.contains("upgrade omv"));
    }

    #[test]
    fn declaration_weaker_than_the_config_values_is_rejected() {
        let root = temp_omv_root("config-declaration-weaker");
        let path = path_for(&root);
        // Hand-edited: pipeline needs 2611.2.1 but the file claims the baseline.
        let raw = r#"schema_version = 1
requires_omv = "0.0.1"
locale = "en-US"
timezone = "UTC+0"
project_profile = "personal"
release_mode = "pipeline"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = false
"#;
        fs::write(&path, raw).expect("should write config fixture");

        let err = load_config(&root).expect_err("an understated declaration must be rejected");
        match err {
            OmvError::Config(ConfigError::DeclarationMismatch { declared, actual }) => {
                assert_eq!(declared, "0.0.1");
                assert_eq!(actual, "2611.2.1");
            }
            other => panic!("unexpected error variant: {other}"),
        }

        cleanup_root(&root);
    }

    #[test]
    fn declaration_stronger_than_needed_is_still_an_upgrade_request() {
        let root = temp_omv_root("config-declaration-stronger");
        let path = path_for(&root);
        // Over-declaring keeps the file internally consistent, but the running
        // binary still cannot honour a config that claims to need a later
        // release. That is the declaration doing its job.
        let raw = r#"schema_version = 1
requires_omv = "9999.1.1"
locale = "en-US"
timezone = "UTC+0"
project_profile = "personal"
release_mode = "local"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = false
"#;
        fs::write(&path, raw).expect("should write config fixture");

        let err = load_config(&root).expect_err("a newer declaration must still be rejected");
        match err {
            OmvError::Config(ConfigError::UnsupportedCapability(detail)) => {
                assert_eq!(detail.required_version, "9999.1.1");
            }
            other => panic!("unexpected error variant: {other}"),
        }

        cleanup_root(&root);
    }

    #[test]
    fn declaration_equal_to_the_actual_need_loads() {
        let root = temp_omv_root("config-declaration-exact");
        let path = path_for(&root);
        let raw = r#"schema_version = 1
requires_omv = "2611.2.1"
locale = "en-US"
timezone = "UTC+0"
project_profile = "personal"
release_mode = "pipeline"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = false
"#;
        fs::write(&path, raw).expect("should write config fixture");

        let loaded = load_config(&root).expect("an exact declaration should load");
        assert_eq!(loaded.release_mode, ReleaseMode::Pipeline);

        cleanup_root(&root);
    }

    #[test]
    fn malformed_requirement_is_rejected_rather_than_ignored() {
        let root = temp_omv_root("config-requires-omv-malformed");
        let path = path_for(&root);
        // Fail-open here would let a typo switch the version check off on the
        // one file that decides whether this project may be read at all.
        let raw = r#"schema_version = 1
requires_omv = "newest"
locale = "en-US"
timezone = "UTC+0"
project_profile = "personal"
release_mode = "pipeline"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = false
"#;
        fs::write(&path, raw).expect("should write config fixture");

        let err = load_config(&root).expect_err("a malformed declaration must be rejected");
        match err {
            OmvError::Config(ConfigError::MalformedRequirement(value)) => {
                assert_eq!(value, "newest");
            }
            other => panic!("unexpected error variant: {other}"),
        }

        cleanup_root(&root);
    }

    #[test]
    fn upgrade_error_names_the_key_that_forced_it() {
        let root = temp_omv_root("config-upgrade-culprit");
        let path = path_for(&root);
        let raw = r#"schema_version = 1
requires_omv = "9999.1.1"
locale = "en-US"
timezone = "UTC+0"
project_profile = "personal"
release_mode = "pipeline"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = false
"#;
        fs::write(&path, raw).expect("should write config fixture");

        let err = load_config(&root).expect_err("a newer declaration must be rejected");
        match err {
            // Reporting `release_mode = "pipeline"` is accurate here: it is why
            // the project needs a later release than the baseline.
            OmvError::Config(ConfigError::UnsupportedCapability(detail)) => {
                assert_eq!(detail.field, "release_mode");
                assert_eq!(detail.value.as_deref(), Some("pipeline"));
                assert_eq!(detail.required_version, "9999.1.1");
            }
            other => panic!("unexpected error variant: {other}"),
        }

        cleanup_root(&root);
    }

    #[test]
    fn reconcile_adds_a_missing_declaration() {
        let root = temp_omv_root("config-reconcile-missing");
        let path = path_for(&root);
        // A project created before the key existed.
        let raw = r#"schema_version = 1
locale = "en-US"
timezone = "UTC+0"
project_profile = "personal"
release_mode = "pipeline"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = false
"#;
        fs::write(&path, raw).expect("should write config fixture");

        assert!(reconcile_requirement(&root).expect("reconcile should succeed"));

        let raw = fs::read_to_string(&path).expect("config should exist");
        assert!(raw.contains("requires_omv = \"2611.2.1\""));

        cleanup_root(&root);
    }

    #[test]
    fn reconcile_leaves_a_correct_declaration_untouched() {
        let root = temp_omv_root("config-reconcile-idempotent");
        save_config(
            &root,
            &OmvConfig {
                release_mode: ReleaseMode::Pipeline,
                ..OmvConfig::default()
            },
        )
        .expect("config should save");

        assert!(!reconcile_requirement(&root).expect("reconcile should succeed"));

        cleanup_root(&root);
    }

    #[test]
    fn reconcile_skips_projects_without_a_config() {
        let root = temp_omv_root("config-reconcile-absent");

        assert!(!reconcile_requirement(&root).expect("reconcile should succeed"));

        cleanup_root(&root);
    }

    #[test]
    fn config_declaring_a_newer_required_omv_is_rejected() {
        let root = temp_omv_root("config-requires-omv-newer");
        let path = path_for(&root);
        let raw = r#"schema_version = 1
requires_omv = "9999.1.1"
locale = "en-US"
timezone = "UTC+0"
project_profile = "personal"
release_mode = "pipeline"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = false
"#;
        fs::write(&path, raw).expect("should write config fixture");

        let err = load_config(&root).expect_err("a newer requires_omv must be rejected");
        match err {
            OmvError::Config(ConfigError::UnsupportedCapability(detail)) => {
                assert_eq!(detail.required_version, "9999.1.1");
                assert_eq!(detail.value.as_deref(), Some("pipeline"));
            }
            other => panic!("unexpected error variant: {other}"),
        }

        cleanup_root(&root);
    }

    #[test]
    fn config_declaring_a_satisfiable_required_omv_loads() {
        let root = temp_omv_root("config-requires-omv-satisfied");
        let path = path_for(&root);
        let raw = r#"schema_version = 1
requires_omv = "2611.2.1"
locale = "en-US"
timezone = "UTC+0"
project_profile = "personal"
release_mode = "pipeline"
version_output = "date-triplet"
build_policy = "daily-reset"
ntp_enabled = false
"#;
        fs::write(&path, raw).expect("should write config fixture");

        let loaded = load_config(&root).expect("satisfiable requires_omv should load");
        assert_eq!(loaded.release_mode, ReleaseMode::Pipeline);

        cleanup_root(&root);
    }

    #[test]
    fn save_config_declares_the_minimum_omv_for_the_project() {
        let root = temp_omv_root("config-save-requires-omv");

        save_config(
            &root,
            &OmvConfig {
                release_mode: ReleaseMode::Pipeline,
                ..OmvConfig::default()
            },
        )
        .expect("config should save");

        let raw = fs::read_to_string(path_for(&root)).expect("config should exist");
        assert!(raw.contains("requires_omv = \"2611.2.1\""));

        // The declaration must round-trip: the file this version writes has to
        // load under the same version.
        let reloaded = load_config(&root).expect("written config should load");
        assert_eq!(reloaded.release_mode, ReleaseMode::Pipeline);

        cleanup_root(&root);
    }

    #[test]
    fn local_projects_declare_only_the_baseline_requirement() {
        let root = temp_omv_root("config-save-baseline");

        save_config(&root, &OmvConfig::default()).expect("config should save");

        let raw = fs::read_to_string(path_for(&root)).expect("config should exist");
        assert!(raw.contains("requires_omv = \"0.0.1\""));

        cleanup_root(&root);
    }

    fn temp_omv_root(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let root = std::env::temp_dir()
            .join(format!("omv-{prefix}-{stamp}"))
            .join(".omv");
        fs::create_dir_all(&root).expect("temp root should be created");
        root
    }

    fn cleanup_root(root: &Path) {
        if let Some(parent) = root.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
