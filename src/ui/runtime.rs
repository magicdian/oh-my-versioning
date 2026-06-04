use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::{Frame, Terminal};

use crate::core::integration::{IntegrationCapability, IntegrationProvider};
use crate::core::locale::OperatorLocale;
use crate::core::target::{PreProjectStrategy, TargetLanguage};
use crate::core::versioning::BuildPolicy;
use crate::errors::{CliError, OmvError};
use crate::i18n::Catalog;
use crate::ui::app::{InitRootAction, UiApp, UiMode};
use crate::ui::discovery::DiscoveryResult;
use crate::ui::event::{KeyInput, UiAction};
use crate::ui::screen::popup::{PopupKind, build_policy_choices, locale_choices, strategy_choices};
use crate::ui::state::draft::{InitDraft, integration_capability_target_files};

pub fn run_init_tui(
    catalog: &Catalog,
    discovery: &DiscoveryResult,
    initial_locale: &str,
) -> Result<InitDraft, OmvError> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, catalog, discovery, initial_locale);

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    catalog: &Catalog,
    discovery: &DiscoveryResult,
    initial_locale: &str,
) -> Result<InitDraft, OmvError> {
    let mut app = UiApp::from_discovery(discovery);
    app.draft
        .set_locale(OperatorLocale::from_input(initial_locale));

    loop {
        terminal.draw(|frame| draw_ui(frame, &app, catalog, discovery))?;

        if app.exit_requested {
            return Err(CliError::UserCancelled.into());
        }

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        let Some(input) = map_key_input(key) else {
            continue;
        };

        match input {
            KeyInput::Up => {
                app.move_up();
                continue;
            }
            KeyInput::Down => {
                app.move_down();
                continue;
            }
            _ => {}
        }

        let row_template = app.row_template_at_cursor();
        let selected_language = app.selected_language_at_cursor();
        let selected_integration = app.integration_cursor_at_cursor();
        let action = app.handle_key(input, row_template, selected_language, selected_integration);

        if action == UiAction::Confirm && !app.popup_open() && app.mode == UiMode::InitRoot {
            match app.current_init_root_action() {
                Some(InitRootAction::LanguageSupport) => app.enter_language_support(),
                Some(InitRootAction::HostIntegrations) => app.enter_host_integrations(),
                Some(InitRootAction::Locale) => app.open_locale_popup(),
                Some(InitRootAction::Timezone) => app.open_timezone_popup(),
                Some(InitRootAction::BuildPolicy) => app.open_build_policy_popup(),
                Some(InitRootAction::Review) => app.enter_review(),
                None => {}
            }
        }

        if action == UiAction::Confirm
            && !app.popup_open()
            && app.mode == UiMode::Review
            && matches!(row_template, crate::ui::widget::row::RowTemplate::Action)
        {
            return Ok(app.draft);
        }
    }
}

fn map_key_input(key: KeyEvent) -> Option<KeyInput> {
    if key.kind != KeyEventKind::Press {
        return None;
    }

    let mapped = match key.code {
        KeyCode::Up => KeyInput::Up,
        KeyCode::Down => KeyInput::Down,
        KeyCode::Left => KeyInput::Left,
        KeyCode::Right => KeyInput::Right,
        KeyCode::Enter => KeyInput::Enter,
        KeyCode::Esc => KeyInput::Esc,
        KeyCode::Char(' ') => KeyInput::Space,
        _ => KeyInput::Other,
    };

    Some(mapped)
}

fn draw_ui(frame: &mut Frame, app: &UiApp, catalog: &Catalog, discovery: &DiscoveryResult) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let title = screen_title(app.mode, catalog);
    let rows = menu_rows(app, catalog);
    let lines: Vec<Line<'_>> = rows
        .iter()
        .enumerate()
        .map(|(idx, text)| {
            if idx == app.menu_index && !app.popup_open() {
                Line::from(format!("> {text}"))
            } else {
                Line::from(format!("  {text}"))
            }
        })
        .collect();

    let menu = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(menu, chunks[0]);

    let manifest_status_key = if discovery.has_any_manifest {
        "init.footer.detected_manifest"
    } else {
        "init.footer.no_manifest"
    };
    let footer = Paragraph::new(footer_text(app, catalog, manifest_status_key));
    frame.render_widget(footer, chunks[1]);

    if let Some(popup) = app.popup {
        let popup_area = centered_rect(70, 40, area);
        frame.render_widget(Clear, popup_area);

        let choices = popup_choice_labels(popup.kind, catalog);
        let viewport_size = popup_viewport_capacity(popup_area);
        let (start, end) = popup_choice_window(choices.len(), popup.selected_index, viewport_size);

        let mut popup_lines = vec![Line::from(popup_hint(popup.kind, catalog)), Line::from("")];
        for (idx, label) in choices.iter().enumerate().skip(start).take(end - start) {
            if idx == popup.selected_index {
                popup_lines.push(Line::from(format!("> {label}")));
            } else {
                popup_lines.push(Line::from(format!("  {label}")));
            }
        }

        let popup_widget = Paragraph::new(popup_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(popup_title(popup.kind, catalog)),
        );
        frame.render_widget(popup_widget, popup_area);
    }
}

fn screen_title(mode: UiMode, catalog: &Catalog) -> String {
    match mode {
        UiMode::InitRoot => catalog.t("init.root.title"),
        UiMode::LanguageSupport => catalog.t("init.language.title"),
        UiMode::HostIntegrations => catalog.t("init.integration.title"),
        UiMode::Review => catalog.t("init.review.title"),
    }
}

fn footer_text(app: &UiApp, catalog: &Catalog, manifest_status_key: &str) -> String {
    let mut segments = vec![
        catalog.t("init.footer.hint"),
        catalog.t(manifest_status_key),
    ];
    if app.mode == UiMode::InitRoot {
        segments.push(catalog.t("init.footer.root_language_tip"));
    }

    segments.join(" | ")
}

fn menu_rows(app: &UiApp, catalog: &Catalog) -> Vec<String> {
    match app.mode {
        UiMode::InitRoot => {
            let enabled = app.draft.enabled_languages().len().to_string();
            let total = app.draft.targets.len().to_string();
            let language_support = catalog.tf(
                "init.root.language_support_with_count",
                &["enabled", enabled.as_str(), "total", total.as_str()],
            );
            let selected_integrations = app.draft.selected_integrations().len().to_string();
            let integration_support = catalog.tf(
                "init.root.integration_support_with_count",
                &["selected", selected_integrations.as_str()],
            );

            vec![
                format!("{language_support} --->"),
                format!("{integration_support} --->"),
                format!(
                    "{} ({}) --->",
                    catalog.t("init.root.locale"),
                    locale_label(app.draft.locale, catalog)
                ),
                format!(
                    "{} ({}) --->",
                    catalog.t("init.root.timezone"),
                    app.draft.timezone_string()
                ),
                format!(
                    "{} ({}) --->",
                    catalog.t("init.root.build_policy"),
                    build_policy_label(app.draft.build_policy, catalog)
                ),
                format!("{} --->", catalog.t("init.root.review")),
            ]
        }
        UiMode::LanguageSupport => {
            let mut rows = app
                .draft
                .targets
                .iter()
                .map(|target| {
                    let marker = if target.enabled { "[*]" } else { "[ ]" };
                    format!("{marker} {}", language_label(target.language, catalog))
                })
                .collect::<Vec<_>>();

            rows.push(format!("{} --->", catalog.t("init.language.strategy")));
            rows
        }
        UiMode::HostIntegrations => integration_rows(app, catalog),
        UiMode::Review => review_rows(app, catalog),
    }
}

fn integration_rows(app: &UiApp, catalog: &Catalog) -> Vec<String> {
    let mut rows = Vec::new();
    for provider in &app.draft.integrations {
        let selected = provider
            .capabilities
            .iter()
            .any(|capability| capability.selected);
        let marker = if selected { "[*]" } else { "[ ]" };
        rows.push(catalog.tf(
            "init.integration.provider_row",
            &[
                "marker",
                marker,
                "provider",
                provider_label(provider.provider, catalog).as_str(),
                "state",
                provider_state_label(provider.detected, provider.recommended, catalog).as_str(),
            ],
        ));

        for capability in &provider.capabilities {
            let marker = if capability.selected { "[*]" } else { "[ ]" };
            rows.push(catalog.tf(
                "init.integration.capability_row",
                &[
                    "marker",
                    marker,
                    "capability",
                    capability_label(capability.capability, catalog).as_str(),
                    "state",
                    capability_state_label(capability.recommended, catalog).as_str(),
                ],
            ));
        }
    }
    rows
}

fn review_rows(app: &UiApp, catalog: &Catalog) -> Vec<String> {
    let mut rows = vec![
        catalog.tf(
            "init.review.languages",
            &[
                "count",
                app.draft.enabled_languages().len().to_string().as_str(),
            ],
        ),
        catalog.t("init.review.integrations_header"),
    ];

    let selected = app.draft.selected_integrations();
    if selected.is_empty() {
        rows.push(catalog.t("init.review.integrations_none"));
    } else {
        for (provider, capability) in selected {
            rows.push(
                catalog.tf(
                    "init.review.integration_item",
                    &[
                        "provider",
                        provider_label(provider, catalog).as_str(),
                        "capability",
                        capability_label(capability, catalog).as_str(),
                        "targets",
                        integration_capability_target_files(provider, capability)
                            .join(", ")
                            .as_str(),
                    ],
                ),
            );
        }
    }

    rows.push(format!("{} --->", catalog.t("init.root.initialize")));
    rows
}

fn language_label(language: TargetLanguage, catalog: &Catalog) -> String {
    let key = match language {
        TargetLanguage::CFamily => "target.language.c_family",
        TargetLanguage::Java => "target.language.java",
        TargetLanguage::Rust => "target.language.rust",
        TargetLanguage::Python => "target.language.python",
        TargetLanguage::Go => "target.language.go",
    };

    catalog.t(key)
}

fn strategy_label(strategy: PreProjectStrategy, catalog: &Catalog) -> String {
    let key = match strategy {
        PreProjectStrategy::IntentOnly => "init.popup.strategy.intent_only",
        PreProjectStrategy::InitExportTemplates => "init.popup.strategy.init_export_templates",
        PreProjectStrategy::CreateMinimalScaffold => "init.popup.strategy.create_scaffold",
    };

    catalog.t(key)
}

fn provider_label(provider: IntegrationProvider, catalog: &Catalog) -> String {
    let key = match provider {
        IntegrationProvider::Claude => "integration.provider.claude",
        IntegrationProvider::Codex => "integration.provider.codex",
        IntegrationProvider::Trellis => "integration.provider.trellis",
        IntegrationProvider::OpenCode => "integration.provider.opencode",
    };
    catalog.t(key)
}

fn capability_label(capability: IntegrationCapability, catalog: &Catalog) -> String {
    let key = match capability {
        IntegrationCapability::ProjectInstructions => "integration.capability.project_instructions",
        IntegrationCapability::HostSkill => "integration.capability.host_skill",
        IntegrationCapability::SpecGuide => "integration.capability.spec_guide",
        IntegrationCapability::SpecIndexSnippet => "integration.capability.spec_index_snippet",
        IntegrationCapability::FinalizeBoundary => "integration.capability.finalize_boundary",
    };
    catalog.t(key)
}

fn provider_state_label(detected: bool, recommended: bool, catalog: &Catalog) -> String {
    match (detected, recommended) {
        (true, true) => catalog.t("init.integration.state.detected_recommended"),
        (true, false) => catalog.t("init.integration.state.detected"),
        (false, _) => catalog.t("init.integration.state.not_detected"),
    }
}

fn capability_state_label(recommended: bool, catalog: &Catalog) -> String {
    if recommended {
        catalog.t("init.integration.state.recommended")
    } else {
        catalog.t("init.integration.state.optional")
    }
}

fn popup_title(kind: PopupKind, catalog: &Catalog) -> String {
    match kind {
        PopupKind::PreProjectStrategy => catalog.t("init.popup.strategy.title"),
        PopupKind::Timezone => catalog.t("init.popup.timezone.title"),
        PopupKind::BuildPolicy => catalog.t("init.popup.build_policy.title"),
        PopupKind::Locale => catalog.t("init.popup.locale.title"),
    }
}

fn popup_hint(kind: PopupKind, catalog: &Catalog) -> String {
    match kind {
        PopupKind::PreProjectStrategy => catalog.t("init.popup.strategy.hint"),
        PopupKind::Timezone => catalog.t("init.popup.timezone.hint"),
        PopupKind::BuildPolicy => catalog.t("init.popup.build_policy.hint"),
        PopupKind::Locale => catalog.t("init.popup.locale.hint"),
    }
}

fn popup_choice_labels(kind: PopupKind, catalog: &Catalog) -> Vec<String> {
    match kind {
        PopupKind::PreProjectStrategy => strategy_choices()
            .iter()
            .map(|strategy| strategy_label(*strategy, catalog))
            .collect(),
        PopupKind::Timezone => (-12..=14).map(format_utc_offset).collect(),
        PopupKind::BuildPolicy => build_policy_choices()
            .iter()
            .map(|policy| build_policy_label(*policy, catalog))
            .collect(),
        PopupKind::Locale => locale_choices()
            .iter()
            .map(|locale| locale_label(*locale, catalog))
            .collect(),
    }
}

fn build_policy_label(policy: BuildPolicy, catalog: &Catalog) -> String {
    let key = match policy {
        BuildPolicy::DailyReset => "init.popup.build_policy.daily_reset",
        BuildPolicy::Continuous => "init.popup.build_policy.continuous",
    };
    catalog.t(key)
}

fn locale_label(locale: OperatorLocale, catalog: &Catalog) -> String {
    let key = match locale {
        OperatorLocale::EnUs => "init.popup.locale.en_us",
        OperatorLocale::ZhCn => "init.popup.locale.zh_cn",
    };
    catalog.t(key)
}

fn format_utc_offset(hours: i32) -> String {
    if hours >= 0 {
        format!("UTC+{hours}")
    } else {
        format!("UTC{hours}")
    }
}

fn popup_viewport_capacity(popup_area: Rect) -> usize {
    let inner_height = popup_area.height.saturating_sub(2) as usize;
    inner_height.saturating_sub(2).max(1)
}

fn popup_choice_window(total: usize, selected: usize, viewport: usize) -> (usize, usize) {
    if total == 0 {
        return (0, 0);
    }
    if total <= viewport {
        return (0, total);
    }

    let selected = selected.min(total - 1);
    let mut start = selected.saturating_sub(viewport / 2);
    if start + viewport > total {
        start = total - viewport;
    }
    (start, start + viewport)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use crate::core::integration::IntegrationProvider;
    use crate::core::locale::OperatorLocale;
    use crate::core::target::TargetLanguage;
    use crate::ui::app::{UiApp, UiMode};
    use crate::ui::discovery::DiscoveryResult;
    use crate::ui::runtime::{
        footer_text, locale_label, menu_rows, popup_choice_labels, popup_choice_window,
    };
    use crate::ui::screen::popup::PopupKind;

    #[test]
    fn init_root_language_row_displays_enabled_count() {
        let catalog = crate::i18n::load_catalog("en-US").expect("catalog should load");
        let discovery = DiscoveryResult {
            detected: vec![TargetLanguage::Rust, TargetLanguage::Python],
            has_any_manifest: true,
            integrations: Vec::new(),
        };
        let app = UiApp::from_discovery(&discovery);

        let rows = menu_rows(&app, &catalog);
        let total = TargetLanguage::all().len().to_string();
        let expected_label = catalog.tf(
            "init.root.language_support_with_count",
            &["enabled", "2", "total", total.as_str()],
        );
        assert_eq!(rows[0], format!("{expected_label} --->"));
        assert!(rows[1].contains("Host Integrations"));
        assert!(rows[2].contains("Language"));
        assert!(rows[3].contains("Timezone"));
        assert!(rows[4].contains("Build Policy"));
    }

    #[test]
    fn init_root_footer_shows_language_entry_hint() {
        let catalog = crate::i18n::load_catalog("zh-CN").expect("catalog should load");
        let app = UiApp {
            mode: UiMode::InitRoot,
            ..UiApp::default()
        };

        let footer = footer_text(&app, &catalog, "init.footer.no_manifest");
        assert!(footer.contains(catalog.t("init.footer.root_language_tip").as_str()));
    }

    #[test]
    fn timezone_popup_contains_utc_zero_option() {
        let catalog = crate::i18n::load_catalog("en-US").expect("catalog should load");
        let choices = popup_choice_labels(PopupKind::Timezone, &catalog);
        assert!(choices.contains(&"UTC+0".to_owned()));
    }

    #[test]
    fn locale_popup_contains_both_operator_languages() {
        let catalog = crate::i18n::load_catalog("en-US").expect("catalog should load");
        let choices = popup_choice_labels(PopupKind::Locale, &catalog);
        assert!(choices.contains(&locale_label(OperatorLocale::EnUs, &catalog)));
        assert!(choices.contains(&locale_label(OperatorLocale::ZhCn, &catalog)));
    }

    #[test]
    fn popup_choice_window_keeps_selected_item_visible() {
        let (start, end) = popup_choice_window(27, 20, 8);
        assert!(start <= 20);
        assert!(end > 20);
        assert_eq!(end - start, 8);
    }

    #[test]
    fn integration_rows_show_provider_detection_and_capabilities() {
        let catalog = crate::i18n::load_catalog("en-US").expect("catalog should load");
        let discovery = DiscoveryResult {
            detected: vec![],
            has_any_manifest: false,
            integrations: vec![crate::ui::discovery::IntegrationProviderDiscovery {
                provider: IntegrationProvider::Trellis,
                detected: true,
            }],
        };
        let app = UiApp {
            mode: UiMode::HostIntegrations,
            ..UiApp::from_discovery(&discovery)
        };

        let rows = menu_rows(&app, &catalog);
        assert!(rows.iter().any(|row| row.contains("Trellis")));
        assert!(rows.iter().any(|row| row.contains("finalize-boundary")));
        assert!(rows.iter().any(|row| row.contains("detected")));
    }

    #[test]
    fn review_rows_include_selected_targets() {
        let catalog = crate::i18n::load_catalog("en-US").expect("catalog should load");
        let discovery = DiscoveryResult {
            detected: vec![],
            has_any_manifest: false,
            integrations: vec![crate::ui::discovery::IntegrationProviderDiscovery {
                provider: IntegrationProvider::Codex,
                detected: true,
            }],
        };
        let app = UiApp {
            mode: UiMode::Review,
            ..UiApp::from_discovery(&discovery)
        };

        let rows = menu_rows(&app, &catalog);
        assert!(rows.iter().any(|row| row.contains("AGENTS.md")));
        assert!(rows.iter().any(|row| row.contains(".codex/skills")));
    }
}
