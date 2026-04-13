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

use crate::core::target::{PreProjectStrategy, TargetLanguage};
use crate::errors::{CliError, OmvError};
use crate::i18n::Catalog;
use crate::ui::app::{InitRootAction, UiApp, UiMode};
use crate::ui::discovery::DiscoveryResult;
use crate::ui::event::{KeyInput, UiAction};
use crate::ui::screen::popup::strategy_choices;
use crate::ui::state::draft::InitDraft;

pub fn run_init_tui(catalog: &Catalog, discovery: &DiscoveryResult) -> Result<InitDraft, OmvError> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, catalog, discovery);

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    catalog: &Catalog,
    discovery: &DiscoveryResult,
) -> Result<InitDraft, OmvError> {
    let mut app = UiApp::from_discovery(discovery);

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
        let action = app.handle_key(input, row_template, selected_language);

        if action == UiAction::Confirm && !app.popup_open() && app.mode == UiMode::InitRoot {
            match app.current_init_root_action() {
                Some(InitRootAction::LanguageSupport) => app.enter_language_support(),
                Some(InitRootAction::Initialize) => return Ok(app.draft),
                None => {}
            }
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

        let mut popup_lines = vec![
            Line::from(catalog.t("init.popup.strategy.hint")),
            Line::from(""),
        ];
        for (idx, strategy) in strategy_choices().iter().enumerate() {
            let label = strategy_label(*strategy, catalog);
            if idx == popup.selected_index {
                popup_lines.push(Line::from(format!("> {label}")));
            } else {
                popup_lines.push(Line::from(format!("  {label}")));
            }
        }

        let popup_widget = Paragraph::new(popup_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(catalog.t("init.popup.strategy.title")),
        );
        frame.render_widget(popup_widget, popup_area);
    }
}

fn screen_title(mode: UiMode, catalog: &Catalog) -> String {
    match mode {
        UiMode::InitRoot => catalog.t("init.root.title"),
        UiMode::LanguageSupport => catalog.t("init.language.title"),
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

            vec![
                format!("{language_support} --->"),
                format!("{} --->", catalog.t("init.root.initialize")),
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
        UiMode::Review => vec![format!("{} --->", catalog.t("init.root.initialize"))],
    }
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
    use crate::core::target::TargetLanguage;
    use crate::ui::app::{UiApp, UiMode};
    use crate::ui::discovery::DiscoveryResult;
    use crate::ui::runtime::{footer_text, menu_rows};

    #[test]
    fn init_root_language_row_displays_enabled_count() {
        let catalog = crate::i18n::load_catalog("en-US").expect("catalog should load");
        let discovery = DiscoveryResult {
            detected: vec![TargetLanguage::Rust, TargetLanguage::Python],
            has_any_manifest: true,
        };
        let app = UiApp::from_discovery(&discovery);

        let rows = menu_rows(&app, &catalog);
        let total = TargetLanguage::all().len().to_string();
        let expected_label = catalog.tf(
            "init.root.language_support_with_count",
            &["enabled", "2", "total", total.as_str()],
        );
        assert_eq!(rows[0], format!("{expected_label} --->"));
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
}
