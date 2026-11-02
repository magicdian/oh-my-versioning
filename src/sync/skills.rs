use std::path::Path;

use crate::core::versioning::ReleaseMode;
use crate::errors::OmvError;

pub fn generate_skills(
    omv_root: &Path,
    version: &str,
    release_mode: ReleaseMode,
) -> Result<(), OmvError> {
    let skills_dir = omv_root.join("skills");

    let readme = match release_mode {
        ReleaseMode::Local => format!(
            "# OMV AI Skills\n\nUse `omv bump` to update project versions.\n\nCurrent recorded version: `{version}`\n"
        ),
        ReleaseMode::Pipeline => format!(
            "# OMV AI Skills\n\nDo not run `omv bump` or `omv sync` locally; the release pipeline owns version writes.\n\nCurrent recorded version: `{version}`\n"
        ),
    };
    crate::storage::atomic::write_atomically(&skills_dir.join("README.md"), readme.as_bytes())?;

    let guidance = match release_mode {
        ReleaseMode::Local => {
            "# Bump Guidance\n\n1. Do not edit native manifest versions directly.\n2. Run `omv bump` to advance version truth in `.omv/state.toml`.\n3. Let `omv` sync manifests and runtime exports from `.omv`.\n"
        }
        ReleaseMode::Pipeline => {
            "# Bump Guidance\n\n1. Do not edit native manifest versions directly.\n2. Do not run `omv bump` or `omv sync` locally; the release pipeline assigns versions.\n3. Use `omv current --json`, `omv plan --json`, and `omv sync --check --json` for reads.\n"
        }
    };
    crate::storage::atomic::write_atomically(
        &skills_dir.join("bump-guidance.md"),
        guidance.as_bytes(),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::generate_skills;
    use crate::core::versioning::ReleaseMode;

    #[test]
    fn generate_skills_writes_expected_files() {
        let root = temp_omv_root("skills");

        generate_skills(&root, "2604.13.2", ReleaseMode::Local).expect("skills should generate");

        let readme =
            fs::read_to_string(root.join("skills/README.md")).expect("README should exist");
        assert!(readme.contains("omv bump"));
        assert!(readme.contains("2604.13.2"));

        let guidance = fs::read_to_string(root.join("skills/bump-guidance.md"))
            .expect("guidance should exist");
        assert!(guidance.contains("Do not edit native manifest versions directly."));

        cleanup_root(&root);
    }

    #[test]
    fn generate_skills_pipeline_mode_marks_bump_as_release_only() {
        let root = temp_omv_root("skills-pipeline");

        generate_skills(&root, "2604.13.2", ReleaseMode::Pipeline).expect("skills should generate");

        let readme =
            fs::read_to_string(root.join("skills/README.md")).expect("README should exist");
        assert!(readme.contains("release pipeline owns version writes"));
        assert!(!readme.contains("Use `omv bump`"));

        let guidance = fs::read_to_string(root.join("skills/bump-guidance.md"))
            .expect("guidance should exist");
        assert!(guidance.contains("Do not run `omv bump`"));
        assert!(guidance.contains("omv sync --check --json"));

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

    fn cleanup_root(root: &std::path::Path) {
        if let Some(parent) = root.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
