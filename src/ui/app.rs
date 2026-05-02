use crate::core::integration::{IntegrationCapability, IntegrationProvider};
use crate::core::locale::OperatorLocale;
use crate::core::target::{PreProjectStrategy, TargetLanguage};
use crate::core::versioning::BuildPolicy;
use crate::ui::discovery::DiscoveryResult;
use crate::ui::event::{KeyInput, UiAction, map_key_to_action};
use crate::ui::screen::popup::{
    PopupKind, build_policy_choices, locale_choices, timezone_choice_count,
};
use crate::ui::state::draft::InitDraft;
use crate::ui::state::focus::FocusTarget;
use crate::ui::state::popup::PopupState;
use crate::ui::widget::row::RowTemplate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    InitRoot,
    LanguageSupport,
    HostIntegrations,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitRootAction {
    LanguageSupport,
    HostIntegrations,
    Locale,
    Timezone,
    BuildPolicy,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationCursor {
    Provider(IntegrationProvider),
    Capability(IntegrationProvider, IntegrationCapability),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiApp {
    pub mode: UiMode,
    pub focus: FocusTarget,
    pub draft: InitDraft,
    pub popup: Option<PopupState>,
    pub menu_index: usize,
    pub exit_requested: bool,
}

impl Default for UiApp {
    fn default() -> Self {
        Self {
            mode: UiMode::InitRoot,
            focus: FocusTarget::Menu,
            draft: InitDraft::default(),
            popup: None,
            menu_index: 0,
            exit_requested: false,
        }
    }
}

impl UiApp {
    pub fn from_discovery(discovery: &DiscoveryResult) -> Self {
        Self {
            draft: InitDraft::from_discovery(&discovery.detected, &discovery.integrations),
            ..Self::default()
        }
    }

    pub fn visible_rows_len(&self) -> usize {
        match self.mode {
            UiMode::InitRoot => 6,
            UiMode::LanguageSupport => self.draft.targets.len() + 1,
            UiMode::HostIntegrations => self.integration_cursor_order().len(),
            UiMode::Review => self.review_rows_len(),
        }
    }

    pub fn row_template_at_cursor(&self) -> RowTemplate {
        match self.mode {
            UiMode::InitRoot => match self.menu_index {
                2..=4 => RowTemplate::FieldEntry,
                _ => RowTemplate::Action,
            },
            UiMode::LanguageSupport => {
                if self.menu_index < self.draft.targets.len() {
                    RowTemplate::Toggle
                } else {
                    RowTemplate::Action
                }
            }
            UiMode::HostIntegrations => RowTemplate::Toggle,
            UiMode::Review => {
                if self.menu_index + 1 == self.review_rows_len() {
                    RowTemplate::Action
                } else {
                    RowTemplate::Info
                }
            }
        }
    }

    pub fn selected_language_at_cursor(&self) -> Option<TargetLanguage> {
        if self.mode != UiMode::LanguageSupport {
            return None;
        }

        self.draft
            .targets
            .get(self.menu_index)
            .map(|target| target.language)
    }

    pub fn integration_cursor_at_cursor(&self) -> Option<IntegrationCursor> {
        if self.mode != UiMode::HostIntegrations {
            return None;
        }

        self.integration_cursor_order()
            .get(self.menu_index)
            .copied()
    }

    pub fn current_init_root_action(&self) -> Option<InitRootAction> {
        if self.mode != UiMode::InitRoot {
            return None;
        }

        match self.menu_index {
            0 => Some(InitRootAction::LanguageSupport),
            1 => Some(InitRootAction::HostIntegrations),
            2 => Some(InitRootAction::Locale),
            3 => Some(InitRootAction::Timezone),
            4 => Some(InitRootAction::BuildPolicy),
            5 => Some(InitRootAction::Review),
            _ => None,
        }
    }

    pub fn enter_language_support(&mut self) {
        self.mode = UiMode::LanguageSupport;
        self.focus = FocusTarget::Menu;
        self.menu_index = 0;
    }

    pub fn enter_host_integrations(&mut self) {
        self.mode = UiMode::HostIntegrations;
        self.focus = FocusTarget::Menu;
        self.menu_index = 0;
    }

    pub fn enter_review(&mut self) {
        self.mode = UiMode::Review;
        self.focus = FocusTarget::Menu;
        self.menu_index = self.review_rows_len().saturating_sub(1);
    }

    pub fn popup_open(&self) -> bool {
        self.popup.is_some()
    }

    pub fn move_up(&mut self) {
        if let Some(mut popup) = self.popup {
            let len = self.popup_choice_count(popup.kind);
            popup.selected_index = if popup.selected_index == 0 {
                len - 1
            } else {
                popup.selected_index - 1
            };
            self.popup = Some(popup);
            return;
        }

        let len = self.visible_rows_len();
        if len == 0 {
            return;
        }
        self.menu_index = if self.menu_index == 0 {
            len - 1
        } else {
            self.menu_index - 1
        };
    }

    pub fn move_down(&mut self) {
        if let Some(mut popup) = self.popup {
            let len = self.popup_choice_count(popup.kind);
            popup.selected_index = (popup.selected_index + 1) % len;
            self.popup = Some(popup);
            return;
        }

        let len = self.visible_rows_len();
        if len == 0 {
            return;
        }
        self.menu_index = (self.menu_index + 1) % len;
    }

    pub fn open_pre_project_strategy_popup(&mut self) {
        self.popup = Some(PopupState::open_with_selection(
            PopupKind::PreProjectStrategy,
            self.strategy_popup_index(),
        ));
        self.focus = FocusTarget::Popup;
    }

    pub fn open_timezone_popup(&mut self) {
        self.popup = Some(PopupState::open_with_selection(
            PopupKind::Timezone,
            self.draft.timezone_popup_index(),
        ));
        self.focus = FocusTarget::Popup;
    }

    pub fn open_locale_popup(&mut self) {
        self.popup = Some(PopupState::open_with_selection(
            PopupKind::Locale,
            self.draft.locale_popup_index(),
        ));
        self.focus = FocusTarget::Popup;
    }

    pub fn open_build_policy_popup(&mut self) {
        self.popup = Some(PopupState::open_with_selection(
            PopupKind::BuildPolicy,
            self.build_policy_popup_index(),
        ));
        self.focus = FocusTarget::Popup;
    }

    pub fn confirm_pre_project_strategy(&mut self, strategy: PreProjectStrategy) {
        self.draft.set_pre_project_strategy(strategy);
        self.popup = None;
        self.focus = FocusTarget::Menu;
    }

    pub fn handle_key(
        &mut self,
        input: KeyInput,
        row_template: RowTemplate,
        selected_language: Option<TargetLanguage>,
        selected_integration: Option<IntegrationCursor>,
    ) -> UiAction {
        let popup_open = self.popup.is_some();
        let action = map_key_to_action(input, row_template, popup_open);

        match action {
            UiAction::Toggle => {
                if popup_open {
                    if let Some(state) = self.popup {
                        self.apply_popup_choice(state.kind, state.selected_index);
                    }
                    return UiAction::Confirm;
                }
                if let Some(language) = selected_language {
                    self.draft.toggle_language(language);
                }
                if let Some(integration) = selected_integration {
                    match integration {
                        IntegrationCursor::Provider(provider) => {
                            self.draft.toggle_integration_provider(provider);
                        }
                        IntegrationCursor::Capability(provider, capability) => {
                            self.draft
                                .toggle_integration_capability(provider, capability);
                        }
                    }
                }
            }
            UiAction::Confirm => {
                if popup_open {
                    if let Some(state) = self.popup {
                        self.apply_popup_choice(state.kind, state.selected_index);
                    }
                } else if matches!(row_template, RowTemplate::Action)
                    && self.mode == UiMode::LanguageSupport
                    && self.menu_index == self.draft.targets.len()
                {
                    self.open_pre_project_strategy_popup();
                }
            }
            UiAction::Back => {
                self.handle_escape();
            }
            _ => {}
        }

        action
    }

    fn popup_choice_count(&self, kind: PopupKind) -> usize {
        match kind {
            PopupKind::PreProjectStrategy => 3,
            PopupKind::Timezone => timezone_choice_count(),
            PopupKind::BuildPolicy => build_policy_choices().len(),
            PopupKind::Locale => locale_choices().len(),
        }
    }

    fn apply_popup_choice(&mut self, kind: PopupKind, selected_index: usize) {
        match kind {
            PopupKind::PreProjectStrategy => {
                let strategy = match selected_index {
                    1 => PreProjectStrategy::InitExportTemplates,
                    2 => PreProjectStrategy::CreateMinimalScaffold,
                    _ => PreProjectStrategy::IntentOnly,
                };
                self.confirm_pre_project_strategy(strategy);
            }
            PopupKind::Timezone => {
                self.draft.set_timezone_from_popup_index(selected_index);
                self.popup = None;
                self.focus = FocusTarget::Menu;
            }
            PopupKind::BuildPolicy => {
                let build_policy = match selected_index {
                    1 => BuildPolicy::Continuous,
                    _ => BuildPolicy::DailyReset,
                };
                self.draft.set_build_policy(build_policy);
                self.popup = None;
                self.focus = FocusTarget::Menu;
            }
            PopupKind::Locale => {
                let locale = match selected_index {
                    1 => OperatorLocale::ZhCn,
                    _ => OperatorLocale::EnUs,
                };
                self.draft.set_locale(locale);
                self.popup = None;
                self.focus = FocusTarget::Menu;
            }
        }
    }

    fn strategy_popup_index(&self) -> usize {
        match self.draft.pre_project_strategy {
            PreProjectStrategy::IntentOnly => 0,
            PreProjectStrategy::InitExportTemplates => 1,
            PreProjectStrategy::CreateMinimalScaffold => 2,
        }
    }

    fn build_policy_popup_index(&self) -> usize {
        match self.draft.build_policy {
            BuildPolicy::DailyReset => 0,
            BuildPolicy::Continuous => 1,
        }
    }

    fn handle_escape(&mut self) {
        if self.popup.is_some() {
            self.popup = None;
            self.focus = FocusTarget::Menu;
            return;
        }

        if self.mode != UiMode::InitRoot {
            self.mode = UiMode::InitRoot;
            self.focus = FocusTarget::Menu;
            self.menu_index = 0;
            return;
        }

        self.exit_requested = true;
    }

    fn integration_cursor_order(&self) -> Vec<IntegrationCursor> {
        self.draft
            .integrations
            .iter()
            .flat_map(|provider| {
                std::iter::once(IntegrationCursor::Provider(provider.provider)).chain(
                    provider.capabilities.iter().map(|capability| {
                        IntegrationCursor::Capability(provider.provider, capability.capability)
                    }),
                )
            })
            .collect()
    }

    fn review_rows_len(&self) -> usize {
        self.draft.selected_integrations().len().max(1) + 3
    }
}

#[cfg(test)]
mod tests {
    use crate::core::integration::{IntegrationCapability, IntegrationProvider};
    use crate::core::locale::OperatorLocale;
    use crate::core::target::{PreProjectStrategy, TargetLanguage};
    use crate::core::versioning::BuildPolicy;
    use crate::ui::app::{IntegrationCursor, UiApp, UiMode};
    use crate::ui::discovery::DiscoveryResult;
    use crate::ui::event::KeyInput;
    use crate::ui::state::focus::FocusTarget;
    use crate::ui::widget::row::RowTemplate;

    fn language_index(app: &UiApp, language: TargetLanguage) -> usize {
        app.draft
            .targets
            .iter()
            .position(|target| target.language == language)
            .expect("language must exist")
    }

    #[test]
    fn discovered_languages_are_preselected_in_draft() {
        let discovery = DiscoveryResult {
            detected: vec![TargetLanguage::Rust, TargetLanguage::Python],
            has_any_manifest: true,
            integrations: Vec::new(),
        };
        let app = UiApp::from_discovery(&discovery);

        assert!(
            app.draft
                .enabled_languages()
                .contains(&TargetLanguage::Rust)
        );
        assert!(
            app.draft
                .enabled_languages()
                .contains(&TargetLanguage::Python)
        );
    }

    #[test]
    fn space_toggles_selected_language() {
        let mut app = UiApp {
            mode: UiMode::LanguageSupport,
            ..UiApp::default()
        };
        app.menu_index = language_index(&app, TargetLanguage::Go);

        assert!(!app.draft.enabled_languages().contains(&TargetLanguage::Go));
        let action = app.handle_key(
            KeyInput::Space,
            RowTemplate::Toggle,
            Some(TargetLanguage::Go),
            None,
        );
        assert_eq!(action, crate::ui::event::UiAction::Toggle);
        assert!(app.draft.enabled_languages().contains(&TargetLanguage::Go));
    }

    #[test]
    fn enter_on_strategy_action_opens_popup() {
        let mut app = UiApp {
            mode: UiMode::LanguageSupport,
            ..UiApp::default()
        };
        app.menu_index = app.draft.targets.len();

        let action = app.handle_key(KeyInput::Enter, RowTemplate::Action, None, None);
        assert_eq!(action, crate::ui::event::UiAction::Confirm);
        assert!(app.popup.is_some());
        assert_eq!(app.focus, FocusTarget::Popup);
    }

    #[test]
    fn popup_choice_persists_strategy_into_draft() {
        let mut app = UiApp {
            mode: UiMode::LanguageSupport,
            ..UiApp::default()
        };
        app.menu_index = language_index(&app, TargetLanguage::Rust);
        app.draft.toggle_language(TargetLanguage::Rust);
        app.open_pre_project_strategy_popup();

        if let Some(mut popup) = app.popup {
            popup.selected_index = 2;
            app.popup = Some(popup);
        }

        app.handle_key(KeyInput::Enter, RowTemplate::Action, None, None);

        assert_eq!(
            app.draft.pre_project_strategy,
            PreProjectStrategy::CreateMinimalScaffold
        );
        assert!(app.popup.is_none());
        assert_eq!(app.focus, FocusTarget::Menu);
    }

    #[test]
    fn popup_choice_persists_timezone_into_draft() {
        let mut app = UiApp::default();
        app.open_timezone_popup();

        if let Some(mut popup) = app.popup {
            popup.selected_index = 20;
            app.popup = Some(popup);
        }

        app.handle_key(KeyInput::Enter, RowTemplate::FieldEntry, None, None);
        assert_eq!(app.draft.timezone_string(), "UTC+8");
        assert!(app.popup.is_none());
    }

    #[test]
    fn popup_choice_persists_build_policy_into_draft() {
        let mut app = UiApp::default();
        app.open_build_policy_popup();

        if let Some(mut popup) = app.popup {
            popup.selected_index = 1;
            app.popup = Some(popup);
        }

        app.handle_key(KeyInput::Enter, RowTemplate::FieldEntry, None, None);
        assert_eq!(app.draft.build_policy, BuildPolicy::Continuous);
        assert!(app.popup.is_none());
    }

    #[test]
    fn popup_choice_persists_locale_into_draft() {
        let mut app = UiApp::default();
        app.open_locale_popup();

        if let Some(mut popup) = app.popup {
            popup.selected_index = 1;
            app.popup = Some(popup);
        }

        app.handle_key(KeyInput::Enter, RowTemplate::FieldEntry, None, None);
        assert_eq!(app.draft.locale, OperatorLocale::ZhCn);
        assert!(app.popup.is_none());
    }

    #[test]
    fn esc_closes_popup_then_backs_to_root_then_exits() {
        let mut app = UiApp {
            mode: UiMode::LanguageSupport,
            ..UiApp::default()
        };
        app.open_pre_project_strategy_popup();

        app.handle_key(KeyInput::Esc, RowTemplate::Action, None, None);
        assert!(app.popup.is_none());
        assert_eq!(app.mode, UiMode::LanguageSupport);
        assert!(!app.exit_requested);

        app.handle_key(KeyInput::Esc, RowTemplate::Action, None, None);
        assert_eq!(app.mode, UiMode::InitRoot);
        assert!(!app.exit_requested);

        app.handle_key(KeyInput::Esc, RowTemplate::Action, None, None);
        assert!(app.exit_requested);
    }

    #[test]
    fn move_navigation_wraps_for_menu_and_popup() {
        let mut app = UiApp::default();
        app.move_up();
        assert_eq!(app.menu_index, app.visible_rows_len() - 1);

        app.move_down();
        assert_eq!(app.menu_index, 0);

        app.open_pre_project_strategy_popup();
        app.move_up();
        let popup = app.popup.expect("popup should stay open");
        assert_eq!(popup.selected_index, 2);
    }

    #[test]
    fn space_toggles_selected_integration_capability() {
        let mut app = UiApp {
            mode: UiMode::HostIntegrations,
            ..UiApp::default()
        };

        let action = app.handle_key(
            KeyInput::Space,
            RowTemplate::Toggle,
            None,
            Some(IntegrationCursor::Capability(
                IntegrationProvider::Codex,
                IntegrationCapability::HostSkill,
            )),
        );

        assert_eq!(action, crate::ui::event::UiAction::Toggle);
        assert!(
            app.draft
                .selected_integrations()
                .contains(&(IntegrationProvider::Codex, IntegrationCapability::HostSkill))
        );
    }
}
