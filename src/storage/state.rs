use std::fs;
use std::path::{Path, PathBuf};

use crate::core::schema::OmvState;
use crate::core::time::LastTimeSource;
use crate::errors::{OmvError, StateError};

use super::{STATE_FILE, atomic};

pub fn path_for(root: &Path) -> PathBuf {
    root.join(STATE_FILE)
}

pub fn load_state(root: &Path) -> Result<OmvState, OmvError> {
    let path = path_for(root);
    let content = fs::read_to_string(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            StateError::MissingState { path: path.clone() }
        } else {
            StateError::Parse {
                path: path.clone(),
                reason: err.to_string(),
            }
        }
    })?;

    let mut state = OmvState::default();
    for (key, value) in parse_assignments(&content) {
        match key.as_str() {
            "schema_version" => {
                state.schema_version = value.parse::<u32>().map_err(|e| StateError::Parse {
                    path: path.clone(),
                    reason: format!("invalid schema_version: {e}"),
                })?
            }
            "logical_date" => state.logical_date = value,
            "build_number" => {
                state.build_number = value.parse::<u32>().map_err(|e| StateError::Parse {
                    path: path.clone(),
                    reason: format!("invalid build_number: {e}"),
                })?
            }
            "last_issued_version" => state.last_issued_version = value,
            "last_time_source" => {
                state.last_time_source =
                    LastTimeSource::parse(&value).ok_or_else(|| StateError::Parse {
                        path: path.clone(),
                        reason: format!("invalid last_time_source: {value}"),
                    })?
            }
            _ => {}
        }
    }

    Ok(state)
}

pub fn save_state(root: &Path, state: &OmvState) -> Result<(), OmvError> {
    let path = path_for(root);
    let content = format!(
        "schema_version = {}\nlogical_date = \"{}\"\nbuild_number = {}\nlast_issued_version = \"{}\"\nlast_time_source = \"{}\"\n",
        state.schema_version,
        state.logical_date,
        state.build_number,
        state.last_issued_version,
        state.last_time_source.as_str(),
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

    use crate::core::schema::OmvState;
    use crate::core::time::LastTimeSource;

    use super::{load_state, save_state};

    #[test]
    fn state_round_trip_preserves_all_fields() {
        let root = temp_omv_root("state-roundtrip");
        let state = OmvState {
            schema_version: 1,
            logical_date: "2026-04-13".to_owned(),
            build_number: 7,
            last_issued_version: "2604.13.7".to_owned(),
            last_time_source: LastTimeSource::ManualConfirmed,
        };

        save_state(&root, &state).expect("state should save");
        let loaded = load_state(&root).expect("state should load");
        assert_eq!(loaded, state);

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
