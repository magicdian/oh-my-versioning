use crate::ui::screen::popup::PopupKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupState {
    pub kind: PopupKind,
    pub open: bool,
    pub selected_index: usize,
}

impl PopupState {
    pub fn open(kind: PopupKind) -> Self {
        Self {
            kind,
            open: true,
            selected_index: 0,
        }
    }
}
