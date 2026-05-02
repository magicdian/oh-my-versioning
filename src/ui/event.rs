use crate::ui::widget::row::RowTemplate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyInput {
    Up,
    Down,
    Left,
    Right,
    Space,
    Enter,
    Esc,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAction {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Toggle,
    Confirm,
    Back,
    Exit,
    Noop,
}

pub fn map_key_to_action(input: KeyInput, row_template: RowTemplate, popup_open: bool) -> UiAction {
    match input {
        KeyInput::Up => UiAction::MoveUp,
        KeyInput::Down => UiAction::MoveDown,
        KeyInput::Left => UiAction::MoveLeft,
        KeyInput::Right => UiAction::MoveRight,
        KeyInput::Esc => UiAction::Back,
        KeyInput::Space => {
            if popup_open {
                UiAction::Confirm
            } else if matches!(row_template, RowTemplate::Toggle) {
                UiAction::Toggle
            } else {
                UiAction::Noop
            }
        }
        KeyInput::Enter => {
            if popup_open {
                UiAction::Confirm
            } else {
                match row_template {
                    RowTemplate::Info | RowTemplate::Toggle => UiAction::Noop,
                    RowTemplate::FieldEntry | RowTemplate::Action => UiAction::Confirm,
                }
            }
        }
        KeyInput::Other => UiAction::Noop,
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::event::{KeyInput, UiAction, map_key_to_action};
    use crate::ui::widget::row::RowTemplate;

    #[test]
    fn space_toggles_only_toggle_rows() {
        assert_eq!(
            map_key_to_action(KeyInput::Space, RowTemplate::Toggle, false),
            UiAction::Toggle
        );
        assert_eq!(
            map_key_to_action(KeyInput::Space, RowTemplate::Action, false),
            UiAction::Noop
        );
        assert_eq!(
            map_key_to_action(KeyInput::Space, RowTemplate::Info, false),
            UiAction::Noop
        );
    }

    #[test]
    fn enter_follows_arrow_rows_but_not_toggle_rows() {
        assert_eq!(
            map_key_to_action(KeyInput::Enter, RowTemplate::FieldEntry, false),
            UiAction::Confirm
        );
        assert_eq!(
            map_key_to_action(KeyInput::Enter, RowTemplate::Action, false),
            UiAction::Confirm
        );
        assert_eq!(
            map_key_to_action(KeyInput::Enter, RowTemplate::Toggle, false),
            UiAction::Noop
        );
        assert_eq!(
            map_key_to_action(KeyInput::Enter, RowTemplate::Info, false),
            UiAction::Noop
        );
    }

    #[test]
    fn popup_uses_space_or_enter_as_confirm_alias() {
        assert_eq!(
            map_key_to_action(KeyInput::Space, RowTemplate::FieldEntry, true),
            UiAction::Confirm
        );
        assert_eq!(
            map_key_to_action(KeyInput::Enter, RowTemplate::Toggle, true),
            UiAction::Confirm
        );
    }

    #[test]
    fn esc_always_maps_to_back() {
        assert_eq!(
            map_key_to_action(KeyInput::Esc, RowTemplate::Action, false),
            UiAction::Back
        );
        assert_eq!(
            map_key_to_action(KeyInput::Esc, RowTemplate::Action, true),
            UiAction::Back
        );
    }
}
