use std::path::Path;

use crate::core::integration::{IntegrationProvider, mvp_provider_descriptors};
use crate::core::target::TargetLanguage;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiscoveryResult {
    pub detected: Vec<TargetLanguage>,
    pub has_any_manifest: bool,
    pub integrations: Vec<IntegrationProviderDiscovery>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegrationProviderDiscovery {
    pub provider: IntegrationProvider,
    pub detected: bool,
}

pub fn discover_languages(root: &Path) -> DiscoveryResult {
    let mut detected = Vec::new();

    if has_any(root, &["Cargo.toml"]) {
        detected.push(TargetLanguage::Rust);
    }
    if has_any(root, &["pyproject.toml", "requirements.txt"]) {
        detected.push(TargetLanguage::Python);
    }
    if has_any(root, &["go.mod"]) {
        detected.push(TargetLanguage::Go);
    }
    if has_any(root, &["pom.xml", "build.gradle", "build.gradle.kts"]) {
        detected.push(TargetLanguage::Java);
    }
    if has_any(root, &["CMakeLists.txt", "Makefile"]) {
        detected.push(TargetLanguage::CFamily);
    }

    DiscoveryResult {
        has_any_manifest: !detected.is_empty(),
        detected,
        integrations: discover_integrations(root),
    }
}

pub fn discover_integrations(root: &Path) -> Vec<IntegrationProviderDiscovery> {
    mvp_provider_descriptors()
        .into_iter()
        .map(|descriptor| IntegrationProviderDiscovery {
            provider: descriptor.provider,
            detected: match descriptor.provider {
                IntegrationProvider::Codex => {
                    root.join(".codex").exists() || root.join("AGENTS.md").exists()
                }
                IntegrationProvider::Trellis => root.join(".trellis").exists(),
            },
        })
        .collect()
}

fn has_any(root: &Path, file_names: &[&str]) -> bool {
    file_names.iter().any(|name| root.join(name).exists())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::core::target::TargetLanguage;

    use crate::core::integration::{IntegrationProvider, mvp_provider_descriptors};

    use super::{discover_integrations, discover_languages};

    #[test]
    fn discover_languages_detects_multiple_manifests() {
        let root = temp_dir("discovery-multi");
        fs::write(root.join("Cargo.toml"), "[package]\nname='x'\n").expect("write Cargo.toml");
        fs::write(root.join("pyproject.toml"), "[project]\nname='x'\n")
            .expect("write pyproject.toml");

        let result = discover_languages(&root);
        assert!(result.has_any_manifest);
        assert!(result.detected.contains(&TargetLanguage::Rust));
        assert!(result.detected.contains(&TargetLanguage::Python));
        assert_eq!(result.integrations.len(), mvp_provider_descriptors().len());

        cleanup_dir(&root);
    }

    #[test]
    fn discover_languages_returns_empty_when_no_manifest_exists() {
        let root = temp_dir("discovery-empty");
        let result = discover_languages(&root);

        assert!(!result.has_any_manifest);
        assert!(result.detected.is_empty());

        cleanup_dir(&root);
    }

    #[test]
    fn discover_integrations_detects_codex_and_trellis_hosts() {
        let root = temp_dir("discovery-integrations");
        fs::create_dir_all(root.join(".codex")).expect("write .codex");
        fs::create_dir_all(root.join(".trellis")).expect("write .trellis");

        let result = discover_integrations(&root);
        assert!(
            result
                .iter()
                .any(|entry| entry.provider == IntegrationProvider::Codex && entry.detected)
        );
        assert!(
            result
                .iter()
                .any(|entry| entry.provider == IntegrationProvider::Trellis && entry.detected)
        );

        cleanup_dir(&root);
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
