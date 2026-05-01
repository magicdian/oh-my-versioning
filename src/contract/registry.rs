use serde::Serialize;

use crate::adapter;
use crate::contract::generated::omv::contract::v1::{
    OmvCapabilitySet, OmvCommandSupport, OmvTargetSupport,
};
use crate::core::target::{TargetKind, TargetLanguage};

pub const CONTRACT_VERSION: u32 = 1;
pub const STRUCTURED_JSON_CONTRACT_VERSION: &str = "1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilityRegistry {
    pub contract_version: u32,
    pub target_support: Vec<TargetCapability>,
    pub command_support: Vec<CommandCapability>,
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

pub fn stage1_registry() -> CapabilityRegistry {
    CapabilityRegistry::default()
}

#[cfg(test)]
mod tests {
    use crate::contract::registry::{CommandCapability, TargetCapability, stage1_registry};
    use crate::core::target::TargetLanguage;

    #[test]
    fn stage1_registry_maps_to_generated_contract_values() {
        let registry = stage1_registry();
        let generated = registry.generated_capability_set();

        assert_eq!(generated.contract_version, 1);
        assert_eq!(generated.ai_adapter_contract_version, 1);
        assert!(registry.supports_language(TargetLanguage::Rust));
        assert!(registry.supports_kind(crate::core::target::TargetKind::TextScalar));
        assert!(registry.target_support.contains(&TargetCapability::CCMake));
        assert!(registry.command_support.contains(&CommandCapability::Plan));
        assert_eq!(
            generated.target_support.len(),
            registry.target_support.len()
        );
    }
}
