use serde::Serialize;

use crate::adapter;
use crate::contract::generated::omv::contract::v1::{
    OmvCapabilitySet, OmvCommandSupport, OmvIntegrationSupport, OmvTargetSupport,
};
use crate::core::integration::{
    IntegrationCapability, IntegrationProviderDescriptor, mvp_provider_descriptors,
};
use crate::core::target::{TargetKind, TargetLanguage};

pub const CONTRACT_VERSION: u32 = 2;
pub const STRUCTURED_JSON_CONTRACT_VERSION: &str = "1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilityRegistry {
    pub contract_version: u32,
    pub target_support: Vec<TargetCapability>,
    pub command_support: Vec<CommandCapability>,
    pub integration_support: Vec<IntegrationCapability>,
    pub integration_providers: Vec<IntegrationProviderDescriptor>,
    pub json_contract_support: Vec<String>,
    pub ai_adapter_contract_version: u32,
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self {
            contract_version: CONTRACT_VERSION,
            target_support: vec![
                TargetCapability::RustCargoPackage,
                TargetCapability::PythonManifest,
                TargetCapability::GoModule,
                TargetCapability::JavaMaven,
                TargetCapability::CCMake,
                TargetCapability::RuntimeExport,
                TargetCapability::TextScalar,
                TargetCapability::RegexReplace,
                TargetCapability::MarkdownManagedBlock,
                TargetCapability::YamlScalar,
                TargetCapability::CHeaderMacro,
                TargetCapability::CargoWorkspace,
            ],
            command_support: vec![
                CommandCapability::Current,
                CommandCapability::Bump,
                CommandCapability::Sync,
                CommandCapability::Adapter,
                CommandCapability::EventFinalizeTask,
                CommandCapability::Plan,
            ],
            integration_support: vec![
                IntegrationCapability::ProjectInstructions,
                IntegrationCapability::HostSkill,
                IntegrationCapability::SpecGuide,
                IntegrationCapability::SpecIndexSnippet,
                IntegrationCapability::FinalizeBoundary,
            ],
            integration_providers: mvp_provider_descriptors(),
            json_contract_support: vec![STRUCTURED_JSON_CONTRACT_VERSION.to_owned()],
            ai_adapter_contract_version: adapter::CONTRACT_VERSION,
        }
    }
}

impl CapabilityRegistry {
    pub fn generated_capability_set(&self) -> OmvCapabilitySet {
        OmvCapabilitySet {
            contract_version: self.contract_version,
            target_support: self
                .target_support
                .iter()
                .map(|capability| capability.proto() as i32)
                .collect(),
            command_support: self
                .command_support
                .iter()
                .map(|capability| capability.proto() as i32)
                .collect(),
            json_contract_support: self.json_contract_support.clone(),
            ai_adapter_contract_version: self.ai_adapter_contract_version,
            integration_support: self
                .integration_support
                .iter()
                .map(|capability| integration_capability_proto(*capability) as i32)
                .collect(),
        }
    }

    pub fn supports_language(&self, language: TargetLanguage) -> bool {
        self.target_support
            .contains(&TargetCapability::from_language(language))
            && self
                .target_support
                .contains(&TargetCapability::RuntimeExport)
    }

    pub fn supports_kind(&self, kind: TargetKind) -> bool {
        self.target_support
            .contains(&TargetCapability::from_kind(kind))
    }

    pub fn supports_integration_capability(&self, capability: IntegrationCapability) -> bool {
        self.integration_support.contains(&capability)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetCapability {
    RustCargoPackage,
    PythonManifest,
    GoModule,
    JavaMaven,
    #[serde(rename = "c-cmake")]
    CCMake,
    RuntimeExport,
    TextScalar,
    RegexReplace,
    MarkdownManagedBlock,
    YamlScalar,
    CHeaderMacro,
    CargoWorkspace,
}

impl TargetCapability {
    pub fn from_language(language: TargetLanguage) -> Self {
        match language {
            TargetLanguage::Rust => Self::RustCargoPackage,
            TargetLanguage::Python => Self::PythonManifest,
            TargetLanguage::Go => Self::GoModule,
            TargetLanguage::Java => Self::JavaMaven,
            TargetLanguage::CFamily => Self::CCMake,
        }
    }

    pub fn from_kind(kind: TargetKind) -> Self {
        match kind {
            TargetKind::TextScalar => Self::TextScalar,
            TargetKind::RegexReplace => Self::RegexReplace,
            TargetKind::MarkdownManagedBlock => Self::MarkdownManagedBlock,
            TargetKind::YamlScalar => Self::YamlScalar,
            TargetKind::CHeaderMacro => Self::CHeaderMacro,
            TargetKind::CargoWorkspace => Self::CargoWorkspace,
        }
    }

    pub fn proto(self) -> OmvTargetSupport {
        match self {
            Self::RustCargoPackage => OmvTargetSupport::RustCargoPackage,
            Self::PythonManifest => OmvTargetSupport::PythonManifest,
            Self::GoModule => OmvTargetSupport::GoModule,
            Self::JavaMaven => OmvTargetSupport::JavaMaven,
            Self::CCMake => OmvTargetSupport::CCmake,
            Self::RuntimeExport => OmvTargetSupport::RuntimeExport,
            Self::TextScalar => OmvTargetSupport::TextScalar,
            Self::RegexReplace => OmvTargetSupport::RegexReplace,
            Self::MarkdownManagedBlock => OmvTargetSupport::MarkdownManagedBlock,
            Self::YamlScalar => OmvTargetSupport::YamlScalar,
            Self::CHeaderMacro => OmvTargetSupport::CHeaderMacro,
            Self::CargoWorkspace => OmvTargetSupport::CargoWorkspace,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandCapability {
    Current,
    Bump,
    Sync,
    Adapter,
    EventFinalizeTask,
    Plan,
}

impl CommandCapability {
    pub fn proto(self) -> OmvCommandSupport {
        match self {
            Self::Current => OmvCommandSupport::Current,
            Self::Bump => OmvCommandSupport::Bump,
            Self::Sync => OmvCommandSupport::Sync,
            Self::Adapter => OmvCommandSupport::Adapter,
            Self::EventFinalizeTask => OmvCommandSupport::EventFinalizeTask,
            Self::Plan => OmvCommandSupport::Plan,
        }
    }
}

fn integration_capability_proto(capability: IntegrationCapability) -> OmvIntegrationSupport {
    match capability {
        IntegrationCapability::ProjectInstructions => OmvIntegrationSupport::ProjectInstructions,
        IntegrationCapability::HostSkill => OmvIntegrationSupport::HostSkill,
        IntegrationCapability::SpecGuide => OmvIntegrationSupport::SpecGuide,
        IntegrationCapability::SpecIndexSnippet => OmvIntegrationSupport::SpecIndexSnippet,
        IntegrationCapability::FinalizeBoundary => OmvIntegrationSupport::FinalizeBoundary,
    }
}

pub fn stage1_registry() -> CapabilityRegistry {
    CapabilityRegistry::default()
}

#[cfg(test)]
mod tests {
    use crate::contract::registry::{CommandCapability, TargetCapability, stage1_registry};
    use crate::core::integration::{IntegrationCapability, IntegrationProvider};
    use crate::core::target::TargetLanguage;

    #[test]
    fn stage1_registry_maps_to_generated_contract_values() {
        let registry = stage1_registry();
        let generated = registry.generated_capability_set();

        assert_eq!(generated.contract_version, 2);
        assert_eq!(generated.ai_adapter_contract_version, 1);
        assert!(registry.supports_language(TargetLanguage::Rust));
        assert!(registry.supports_kind(crate::core::target::TargetKind::TextScalar));
        assert!(registry.target_support.contains(&TargetCapability::CCMake));
        assert!(registry.command_support.contains(&CommandCapability::Plan));
        assert!(registry.supports_integration_capability(IntegrationCapability::FinalizeBoundary));
        assert_eq!(
            generated.target_support.len(),
            registry.target_support.len()
        );
        assert_eq!(
            generated.integration_support.len(),
            registry.integration_support.len()
        );
    }

    #[test]
    fn current_proto_matches_latest_frozen_snapshot() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let latest_version = newest_frozen_contract_version(root);
        let current = std::fs::read_to_string(
            root.join("proto/omv/contract/versions/current/contract.proto"),
        )
        .expect("current contract proto should exist");
        let latest = std::fs::read_to_string(root.join(format!(
            "proto/omv/contract/versions/{latest_version}/contract.proto"
        )))
        .expect("latest frozen contract proto should exist");

        assert_eq!(latest_version, super::CONTRACT_VERSION);
        assert_eq!(current, latest);
    }

    #[test]
    fn frozen_v1_and_v2_capture_contract_boundaries() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let v1 = std::fs::read_to_string(root.join("proto/omv/contract/versions/1/contract.proto"))
            .expect("v1 frozen contract proto should exist");
        let v2 = std::fs::read_to_string(root.join("proto/omv/contract/versions/2/contract.proto"))
            .expect("v2 frozen contract proto should exist");

        assert!(v1.contains("OMV_TARGET_SUPPORT_RUNTIME_EXPORT = 6;"));
        assert!(!v1.contains("OMV_TARGET_SUPPORT_MARKDOWN_MANAGED_BLOCK"));
        assert!(!v1.contains("OmvIntegrationSupport"));
        assert!(v2.contains("OMV_TARGET_SUPPORT_MARKDOWN_MANAGED_BLOCK = 9;"));
        assert!(v2.contains("OMV_TARGET_SUPPORT_YAML_SCALAR = 10;"));
        assert!(v2.contains("OMV_TARGET_SUPPORT_CARGO_WORKSPACE = 12;"));
        assert!(v2.contains("OmvIntegrationSupport"));
        assert!(v2.contains("repeated OmvIntegrationSupport integration_support = 6;"));
    }

    fn newest_frozen_contract_version(root: &std::path::Path) -> u32 {
        std::fs::read_dir(root.join("proto/omv/contract/versions"))
            .expect("contract versions directory should exist")
            .filter_map(|entry| {
                let entry = entry.expect("contract version directory entry should be readable");
                if !entry
                    .file_type()
                    .expect("file type should be readable")
                    .is_dir()
                {
                    return None;
                }
                entry.file_name().to_string_lossy().parse::<u32>().ok()
            })
            .max()
            .expect("at least one frozen contract version should exist")
    }

    #[test]
    fn registry_exposes_mvp_integration_provider_descriptors() {
        let registry = stage1_registry();
        let codex = registry
            .integration_providers
            .iter()
            .find(|provider| provider.provider == IntegrationProvider::Codex)
            .expect("codex provider should be registered");
        let trellis = registry
            .integration_providers
            .iter()
            .find(|provider| provider.provider == IntegrationProvider::Trellis)
            .expect("trellis provider should be registered");

        assert!(
            codex
                .capabilities
                .iter()
                .any(|capability| capability.capability == IntegrationCapability::HostSkill)
        );
        assert!(trellis.capabilities.iter().any(|capability| {
            capability.capability == IntegrationCapability::FinalizeBoundary
        }));
    }
}
