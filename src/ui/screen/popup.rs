use crate::core::target::PreProjectStrategy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupKind {
    PreProjectStrategy,
}

pub fn strategy_choices() -> &'static [PreProjectStrategy] {
    const CHOICES: [PreProjectStrategy; 3] = [
        PreProjectStrategy::IntentOnly,
        PreProjectStrategy::InitExportTemplates,
        PreProjectStrategy::CreateMinimalScaffold,
    ];
    &CHOICES
}
