use std::fs;
use std::path::{Path, PathBuf};

use crate::core::locale::OperatorLocale;
use crate::core::schema::OmvConfig;
use crate::core::target::ProjectProfile;
use crate::core::versioning::{BuildPolicy, VersionOutput};
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

    let mut config = OmvConfig::default();
    for (key, value) in parse_assignments(&content) {
        match key.as_str() {
            "schema_version" => {
                config.schema_version = value.parse::<u32>().map_err(|e| ConfigError::Parse {
                    path: path.clone(),
                    reason: format!("invalid schema_version: {e}"),
                })?;
            }
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

    Ok(config)
}

pub fn save_config(root: &Path, config: &OmvConfig) -> Result<(), OmvError> {
    let path = path_for(root);
    let content = format!(
        "schema_version = {}\nlocale = \"{}\"\ntimezone = \"{}\"\nproject_profile = \"{}\"\nversion_output = \"{}\"\nbuild_policy = \"{}\"\nntp_enabled = {}\n",
        config.schema_version,
        config.locale.as_str(),
        config.timezone,
        config.project_profile.as_str(),
        config.version_output.as_str(),
        config.build_policy.as_str(),
        config.ntp_enabled,
    );

    atomic::write_atomically(&path, content.as_bytes())
}

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
    use crate::core::versioning::{BuildPolicy, VersionOutput};
    use crate::errors::{ConfigError, OmvError};

    use super::{load_config, path_for, save_config};

    #[test]
    fn config_round_trip_preserves_all_fields() {
        let root = temp_omv_root("config-roundtrip");

        let config = OmvConfig {
            schema_version: 1,
            locale: OperatorLocale::ZhCn,
            timezone: "UTC+8".to_owned(),
            project_profile: ProjectProfile::Oss,
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
