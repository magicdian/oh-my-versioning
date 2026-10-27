# Component Guidelines

> How `ratatui` screens and widgets are built in `omv`.

---

## Overview

Treat screens, rows, and popups as the component system for this project.
Rendering must follow the canonical menuconfig contract in
`docs/matrix/MENUCONFIG_STYLE_MATRIX.md`.

## Component Structure

Each screen should have:

1. typed input state
2. a function that derives row descriptors
3. a render function
4. event handling that returns typed actions, not direct file writes

Example shape:

```rust
struct InitRootViewModel { /* localized labels + row states */ }

fn build_init_root_model(draft: &InitDraft, catalog: &Catalog) -> InitRootViewModel;
fn render_init_root(frame: &mut Frame, model: &InitRootViewModel);
```

## Props Conventions

- pass typed view models to renderers
- pass `Catalog` or already localized text, never raw locale strings plus keys
  plus lookup logic in the widget
- pass row template variants explicitly; do not handcraft prefix glyphs inline

## Styling Patterns

- one-column menuconfig layout only for main flow
- center the content block; left-align text inside rows
- keep row-template grammar exactly aligned with the matrix doc
- preserve right-side `--->` suffix during truncation
- for `choice-list-modal`, render a viewport window when options exceed popup
  height; selected option must stay visible while moving `Up/Down`

## Init Root Field Entries

`omv init` root rows should keep field-entry semantics for operator-editable
settings:

- `Language (value) --->`
- `Timezone (value) --->`
- `Build Policy (value) --->`

These rows open choice popups on `Enter`. They must not toggle on `Space`.

## Init Integration Review

`omv init` must include a review/confirm step for host integrations before
automatic installation is attempted.

Review rows should show:

- provider identity (`codex`, `trellis`)
- detection/recommendation state
- selected capability identifiers
- affected target files per capability

Use menuconfig semantics:

- provider/capability enablement rows are toggle rows; `Space` toggles
- detail rows that show target files use `--->` and open read-only detail
  popups on `Enter`
- the final apply/continue action is explicit and follows the existing confirm
  flow

The screen must consume typed detection/plan data from backend orchestration.
Render code must not inspect the filesystem, decide bootstrap policy, or write
host files.

## Accessibility

For terminal UI, accessibility means predictable keyboard semantics and readable
state transitions:

- `Space` toggles only toggle rows
- `Enter` follows action/detail flows
- `Esc` exits popup/back/root in the documented order
- status/help text may wrap; menu rows must remain single-line

## Common Mistakes

### Don't: Encode business logic in render code

Renderers should not decide version numbers, parse manifests, or write files.

### Don't: Draw semantic prefixes manually

Use row-template-driven rendering so every screen behaves the same way.

### Don't: Hardcode labels inside components

All copy must come from i18n catalogs.
