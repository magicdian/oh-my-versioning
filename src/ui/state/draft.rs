use crate::core::locale::OperatorLocale;
use crate::core::target::{PreProjectStrategy, TargetLanguage};
use crate::core::versioning::BuildPolicy;

pub const MIN_TIMEZONE_OFFSET_HOURS: i8 = -12;
pub const MAX_TIMEZONE_OFFSET_HOURS: i8 = 14;

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
    pub timezone_offset_hours: i8,
    pub build_policy: BuildPolicy,
    pub locale: OperatorLocale,
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
            timezone_offset_hours: 0,
            build_policy: BuildPolicy::DailyReset,
            locale: OperatorLocale::EnUs,
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

    pub fn set_timezone_offset_hours(&mut self, hours: i8) {
        self.timezone_offset_hours =
            hours.clamp(MIN_TIMEZONE_OFFSET_HOURS, MAX_TIMEZONE_OFFSET_HOURS);
    }

    pub fn timezone_popup_index(&self) -> usize {
        (self.timezone_offset_hours - MIN_TIMEZONE_OFFSET_HOURS) as usize
    }

    pub fn set_timezone_from_popup_index(&mut self, index: usize) {
        let max_index = (MAX_TIMEZONE_OFFSET_HOURS - MIN_TIMEZONE_OFFSET_HOURS) as usize;
        let clamped = index.min(max_index) as i8;
        self.timezone_offset_hours = MIN_TIMEZONE_OFFSET_HOURS + clamped;
    }

    pub fn timezone_string(&self) -> String {
        format_utc_offset(self.timezone_offset_hours)
    }

    pub fn set_build_policy(&mut self, build_policy: BuildPolicy) {
        self.build_policy = build_policy;
    }

    pub fn set_locale(&mut self, locale: OperatorLocale) {
        self.locale = locale;
    }

    pub fn locale_popup_index(&self) -> usize {
        match self.locale {
            OperatorLocale::EnUs => 0,
            OperatorLocale::ZhCn => 1,
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

fn format_utc_offset(offset: i8) -> String {
    if offset >= 0 {
        format!("UTC+{offset}")
    } else {
        format!("UTC{offset}")
    }
}

#[cfg(test)]
mod tests {
    use crate::core::locale::OperatorLocale;
    use crate::core::target::{PreProjectStrategy, TargetLanguage};
    use crate::core::versioning::BuildPolicy;

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

    #[test]
    fn timezone_popup_index_round_trip_uses_canonical_utc_format() {
        let mut draft = InitDraft::default();
        draft.set_timezone_from_popup_index(20);

        assert_eq!(draft.timezone_string(), "UTC+8");
        assert_eq!(draft.timezone_popup_index(), 20);
    }

    #[test]
    fn build_policy_is_mutable_in_draft() {
        let mut draft = InitDraft::default();
        assert_eq!(draft.build_policy, BuildPolicy::DailyReset);

        draft.set_build_policy(BuildPolicy::Continuous);
        assert_eq!(draft.build_policy, BuildPolicy::Continuous);
    }

    #[test]
    fn locale_popup_index_tracks_selected_locale() {
        let mut draft = InitDraft::default();
        assert_eq!(draft.locale_popup_index(), 0);

        draft.set_locale(OperatorLocale::ZhCn);
        assert_eq!(draft.locale_popup_index(), 1);
    }
}
