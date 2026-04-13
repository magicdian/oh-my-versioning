use crate::core::target::{PreProjectStrategy, TargetLanguage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetDraft {
    pub language: TargetLanguage,
    pub enabled: bool,
    pub strategy: PreProjectStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitDraft {
    pub targets: Vec<TargetDraft>,
    pub pre_project_strategy: PreProjectStrategy,
}

impl Default for InitDraft {
    fn default() -> Self {
        Self::from_detected_languages(&[])
    }
}

impl InitDraft {
    pub fn from_detected_languages(detected: &[TargetLanguage]) -> Self {
        let targets = TargetLanguage::all()
            .iter()
            .map(|language| TargetDraft {
                language: *language,
                enabled: detected.contains(language),
                strategy: PreProjectStrategy::IntentOnly,
            })
            .collect();

        Self {
            targets,
            pre_project_strategy: PreProjectStrategy::IntentOnly,
        }
    }

    pub fn toggle_language(&mut self, language: TargetLanguage) {
        if let Some(target) = self
            .targets
            .iter_mut()
            .find(|target| target.language == language)
        {
            target.enabled = !target.enabled;
        }
    }

    pub fn set_pre_project_strategy(&mut self, strategy: PreProjectStrategy) {
        self.pre_project_strategy = strategy;
        for target in &mut self.targets {
            if target.enabled {
                target.strategy = strategy;
            }
        }
    }

    pub fn enabled_languages(&self) -> Vec<TargetLanguage> {
        self.targets
            .iter()
            .filter(|target| target.enabled)
            .map(|target| target.language)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::target::{PreProjectStrategy, TargetLanguage};

    use super::InitDraft;

    #[test]
    fn auto_detected_languages_start_enabled() {
        let draft =
            InitDraft::from_detected_languages(&[TargetLanguage::Rust, TargetLanguage::Python]);

        assert!(draft.enabled_languages().contains(&TargetLanguage::Rust));
        assert!(draft.enabled_languages().contains(&TargetLanguage::Python));
        assert!(!draft.enabled_languages().contains(&TargetLanguage::Go));
    }

    #[test]
    fn toggle_language_switches_enabled_state() {
        let mut draft = InitDraft::from_detected_languages(&[]);
        assert!(!draft.enabled_languages().contains(&TargetLanguage::Go));

        draft.toggle_language(TargetLanguage::Go);
        assert!(draft.enabled_languages().contains(&TargetLanguage::Go));

        draft.toggle_language(TargetLanguage::Go);
        assert!(!draft.enabled_languages().contains(&TargetLanguage::Go));
    }

    #[test]
    fn strategy_popup_selection_updates_enabled_targets() {
        let mut draft = InitDraft::from_detected_languages(&[TargetLanguage::Rust]);

        draft.set_pre_project_strategy(PreProjectStrategy::InitExportTemplates);

        assert_eq!(
            draft.pre_project_strategy,
            PreProjectStrategy::InitExportTemplates
        );
        let rust_target = draft
            .targets
            .iter()
            .find(|target| target.language == TargetLanguage::Rust)
            .expect("rust target should exist");
        assert_eq!(
            rust_target.strategy,
            PreProjectStrategy::InitExportTemplates
        );

        let java_target = draft
            .targets
            .iter()
            .find(|target| target.language == TargetLanguage::Java)
            .expect("java target should exist");
        assert_eq!(java_target.strategy, PreProjectStrategy::IntentOnly);
    }
}
