use crate::ui::screen::popup::PopupKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupState {
    pub kind: PopupKind,
    pub open: bool,
    pub selected_index: usize,
}

impl PopupState {
    pub fn open(kind: PopupKind) -> Self {
        Self::open_with_selection(kind, 0)
    }

    pub fn open_with_selection(kind: PopupKind, selected_index: usize) -> Self {
        Self {
            kind,
            open: true,
            selected_index,
        }
    }
}
