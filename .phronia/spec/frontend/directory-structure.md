# Directory Structure

> How TUI/frontend code is organized in `omv`.

---

## Overview

Keep UI rendering, event handling, and draft-state management separate so the
same backend command logic can be reused outside the TUI when needed.

## Directory Layout

```text
src/ui/
├── app.rs                 # app bootstrap and top-level event loop
├── screen/
│   ├── init_root.rs       # main init menu
│   ├── language.rs        # language support screen/section
│   ├── review.rs
│   └── popup.rs
├── widget/
│   ├── row.rs             # row-template rendering
│   ├── footer.rs
│   └── modal.rs
├── state/
│   ├── draft.rs           # init draft state
│   ├── focus.rs           # focus model and navigation
│   └── popup.rs           # popup state
└── event.rs               # key events -> actions
```

## Module Organization

- `screen/` decides what should be shown
- `widget/` decides how it is rendered
- `state/` owns transient UI state only
- backend/app code owns persistence, validation, and synchronization

## Naming Conventions

- screen modules: `<flow>_<screen>.rs`
- widgets: named by reusable UI concept such as `row`, `modal`, `footer`
- event enums: `UiAction`, `PopupAction`, `FocusTarget`

## Examples

Follow these separations:

- menuconfig row grammar comes from the matrix doc, not from ad hoc widget code
- locale strings come from catalogs, not from `screen/*.rs`
- popup selection results mutate draft state, then backend code persists
