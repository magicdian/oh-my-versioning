use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::errors::{OmvError, StorageError};

pub fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), OmvError> {
    let parent = path
        .parent()
        .ok_or_else(|| StorageError::AtomicWriteFailed {
            path: path.to_path_buf(),
            reason: String::from("target path has no parent directory"),
        })?;

    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("omv-tmp");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = parent.join(format!(".{file_name}.{stamp}.tmp"));

    fs::write(&tmp_path, bytes).map_err(|err| StorageError::AtomicWriteFailed {
        path: path.to_path_buf(),
        reason: err.to_string(),
    })?;

    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(StorageError::AtomicWriteFailed {
            path: path.to_path_buf(),
            reason: err.to_string(),
        }
        .into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::errors::OmvError;

    use super::write_atomically;

    #[test]
    fn atomic_write_replaces_existing_content() {
        let dir = temp_dir("atomic-success");
        let target = dir.join("state.toml");

        fs::write(&target, "old").expect("should write initial file");
        write_atomically(&target, b"new").expect("atomic write should succeed");

        let content = fs::read_to_string(&target).expect("target should exist");
        assert_eq!(content, "new");

        cleanup_dir(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_failure_does_not_corrupt_existing_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("atomic-failure");
        let target = dir.join("config.toml");
        fs::write(&target, "stable").expect("should write initial file");

        let mut perms = fs::metadata(&dir)
            .expect("dir metadata should exist")
            .permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&dir, perms).expect("set read-only permissions");

        let result = write_atomically(&target, b"mutated");

        let mut restore = fs::metadata(&dir)
            .expect("dir metadata should exist")
            .permissions();
        restore.set_mode(0o755);
        fs::set_permissions(&dir, restore).expect("restore writable permissions");

        assert!(
            result.is_err(),
            "write should fail when parent dir is read-only"
        );
        let content = fs::read_to_string(&target).expect("original file should still exist");
        assert_eq!(content, "stable");
        assert!(matches!(
            result,
            Err(OmvError::Storage(_)) | Err(OmvError::Io(_))
        ));

        cleanup_dir(&dir);
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("omv-{prefix}-{stamp}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn cleanup_dir(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }
}
