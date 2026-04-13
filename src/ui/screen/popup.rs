use crate::core::locale::OperatorLocale;
use crate::core::target::PreProjectStrategy;
use crate::core::versioning::BuildPolicy;
use crate::ui::state::draft::{MAX_TIMEZONE_OFFSET_HOURS, MIN_TIMEZONE_OFFSET_HOURS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupKind {
    PreProjectStrategy,
    Timezone,
    BuildPolicy,
    Locale,
}

pub fn strategy_choices() -> &'static [PreProjectStrategy] {
    const CHOICES: [PreProjectStrategy; 3] = [
        PreProjectStrategy::IntentOnly,
        PreProjectStrategy::InitExportTemplates,
        PreProjectStrategy::CreateMinimalScaffold,
    ];
    &CHOICES
}

pub fn build_policy_choices() -> &'static [BuildPolicy] {
    const CHOICES: [BuildPolicy; 2] = [BuildPolicy::DailyReset, BuildPolicy::Continuous];
    &CHOICES
}

pub fn timezone_choice_count() -> usize {
    (MAX_TIMEZONE_OFFSET_HOURS - MIN_TIMEZONE_OFFSET_HOURS + 1) as usize
}

pub fn locale_choices() -> &'static [OperatorLocale] {
    const CHOICES: [OperatorLocale; 2] = [OperatorLocale::EnUs, OperatorLocale::ZhCn];
    &CHOICES
}
