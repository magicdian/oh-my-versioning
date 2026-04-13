# Menuconfig Style Matrix

This document is the canonical style matrix for `omv` menuconfig-style TUI
flows and any future trusted local UI/TUI host that intentionally reuses the
same menuconfig grammar.

## Scope

- `omv init`
- future `omv` menuconfig-style flows that explicitly align to the same
  row grammar, key semantics, popup contract, and fallback rules

## Principles

- Single-column, step-by-step navigation. No main-flow multi-column layout.
- Content area centered; text inside each row left-aligned.
- Prefixes are semantic, not decorative.
- `>` marks current focus on actionable rows only.
- `Esc` first closes the current popup, then backs out one level, then exits
  from root.
- Title dirty suffix and root exit-confirm must be derived from current draft
  vs session baseline differences, not from edit-history alone.
- Future menuconfig feature work must match this matrix before merge.
- Screen implementations must choose a template first, then fill data.
- Screen implementations must not handcraft semantic prefixes or `--->`.
- `omv init` must prefer automatic project/language discovery, but keep the
  operator in control through explicit toggle and popup choices.
- When no project files exist yet, the UI must not guess silently; it must
  surface the chosen fallback via an explicit popup.

## Template-First Rules

- Any new row must map to one `row template`.
- Any mutually-exclusive option family must map to one `group template`.
- Any popup or overlay must map to one `popup template`.
- Template-level key semantics override screen-local conventions.
- `boolean-toggle-row` and `exclusive-choice-row` are different semantics and
  must never be merged into one ambiguous type.
- `blocked-action-row` and `required-readonly-row` must stay visually and
  behaviorally distinct from normal info rows and action rows.
- Language support selection in `omv init` must use `multi-select-row`.
- Timezone, build policy, and output-mode selection must use
  `field-entry-row` plus an appropriate popup template.

## Row Template Catalog

| Template ID | Intent | Focusable | Canonical Grammar | Key Contract | Notes |
| --- | --- | --- | --- | --- | --- |
| `info-row` | Read-only status/description | no | `--- Label` or `--- Label = Value` | none | navigation must skip |
| `fixed-disabled-row` | Permanently disabled feature projection | no | `- - Label` | none | not an actionable control |
| `fixed-enabled-row` | Permanently enabled feature projection | no | `-*- Label` | none | not an actionable control |
| `boolean-toggle-row` | Pure boolean on/off, no follow-up page | yes | `< > Label` / `<*> Label` | `Space` toggles; `Enter` must not toggle | must not carry `--->` |
| `exclusive-choice-row` | One option inside an exclusive group | yes | `< > Label` / `<*> Label` | `Space` toggles group selection; `Enter` does not change selection | used with `exclusive-choice-group` |
| `exclusive-choice-entry-row` | Exclusive option that also owns a detail entry | yes | `< > Label --->` / `<*> Label --->` | `Space` toggles group selection; `Enter` enters detail of current selected option | used with `exclusive-choice-group` |
| `multi-select-row` | Independent multi-select option | yes | `[ ] Label` / `[*] Label` | `Space` toggles only | must not carry `--->` |
| `field-entry-row` | Field editor entry (text/enum/etc.) | yes | `Label (value) --->` | `Enter` opens editor popup | prefix column kept aligned and blank |
| `action-row` | Navigation/action entry | yes | `Label --->` | `Enter` opens flow | used for submenu and managed actions |
| `blocked-action-row` | Action exists but currently blocked | no | `--- Label (blocked: reason)` | none | reason must be visible inline |
| `required-readonly-row` | Policy/system-enforced fixed state | no | `--- Label = required` | none | must not respond to toggle keys |

## Group Template Catalog

| Template ID | Intent | Allowed Row Templates | Group Contract | Key Contract |
| --- | --- | --- | --- | --- |
| `exclusive-choice-group` | Mutually-exclusive mode/strategy chooser | `exclusive-choice-row`, `exclusive-choice-entry-row` | exactly one option selected at any time | `Space` changes selected option; `Enter` only enters selected option detail when supported |
| `selection-picker-group` | Mutually-exclusive object picker | `exclusive-choice-row`, `exclusive-choice-entry-row` | exactly one option selected; selection binds to external object reference | same as `exclusive-choice-group`; picker outcome must be persisted |

## Popup Template Catalog

| Template ID | Intent | Primitive Composition | Key Contract | Notes |
| --- | --- | --- | --- | --- |
| `message-modal` | Read-only policy/help/info popup | `modal-shell` + body text (+ optional `button-row`) | `Esc` closes; `Enter` closes when acknowledge exists | used by help and informational notices |
| `confirm-modal` | Confirm/cancel decision popup | `modal-shell` + body + `button-row` | `Left/Right` focus button; `Enter` confirms current button; `Esc` cancels | used for save, exit, and risk confirms |
| `text-input-modal` | One-line text input editor | `modal-shell` + `input-line` (+ optional `button-row`) | `Left/Right` move caret; `Backspace` deletes; `Enter` commits; `Esc` cancels | used by generic edit flows |
| `choice-list-modal` | One-of-many option picker | `modal-shell` + `choice-list` | `Up/Down` move; `Enter` or `Space` confirms; `Esc` cancels | used by enum/choice editing |
| `waiting-modal` | In-progress blocking state | `modal-shell` + waiting body | usually `Esc` cancels when operation supports it | background menu is frozen |
| `result-modal` | Operation result acknowledgement | `modal-shell` + result body (+ optional `button-row`) | `Enter`/`Esc` acknowledge and close | used by detect/test/init results |
| `one-time-reveal-modal` | Sensitive value reveal with explicit close | `modal-shell` + reveal body (+ optional `button-row`) | `Enter`/`Esc` closes by flow contract | reserved for future sensitive flows |
| `blocking-overlay` | Hard gate overlay (resize/policy lock) | `modal-shell` + blocking message (+ optional `button-row`) | normal navigation disabled until resolved/exit | used when menu cannot safely continue |

## Popup Primitive Mapping

| Primitive | Responsibility | Shared Contract |
| --- | --- | --- |
| `modal-shell` | centered rect + clear + bordered container + title/body slots | all popup templates use one shell layout baseline |
| `button-row` | horizontal button strip with one active button | selected button uses same fallback highlighting as menu rows |
| `choice-list` | vertical selectable options list | active option uses same fallback highlighting as menu rows |
| `input-line` | editable one-line input with caret | caret always visible, left/right/backspace contract stable across editors |

## Layout Matrix

| Area | Structure | Alignment | Focus / Interaction | Notes |
| --- | --- | --- | --- | --- |
| Main frame | bordered single-column screen | centered as a whole | menu list remains active while a footer button is also focusable | no split-pane primary layout |
| Menu list | vertical list of rows | block centered, row text left-aligned | `↑/↓` cycles across actionable rows only | non-focusable rows are skipped |
| Footer button bar | inline action bar | centered | `←/→` moves between `<Select> < Exit > < Help >`; `Enter` activates current footer button | focused footer button uses same selected feedback as menu rows |
| Status / description block | multi-line status panel | left-aligned inside block | read-only | may wrap |
| Popup overlay | centered modal overlay | content left-aligned unless button row requires centering | background menu is frozen | `Esc` closes current popup first |
| Resize guard | centered blocking overlay | centered or left-aligned prompt | normal navigation suspended until size recovers or operator exits | shown when viewport is smaller than minimum supported size |

## Dirty Tracking Contract

- Title dirty suffix appears only when current `init draft` differs from the
  session baseline snapshot.
- Reverting an edited field back to its baseline value clears dirty state for
  that field; if no real differences remain, the title dirty suffix must
  disappear.
- Root `Esc` / `< Exit >` opens `Yes/No/Cancel` save confirm only when real
  unsaved differences remain; if all edits are reverted to baseline, root exit
  must proceed without save confirm.

## Row Grammar Matrix (Template-Derived)

| Row Template | Focus Marker | Semantic Prefix | Canonical Grammar | Focusable | Primary Keys | Selected Feedback | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `info-row` | none | `---` | `--- Label` or `--- Label = Value` | no | none | none | navigation must skip |
| `fixed-disabled-row` | none | `- -` | `- - Label` | no | none | none | navigation must skip |
| `fixed-enabled-row` | none | `-*-` | `-*- Label` | no | none | none | navigation must skip |
| `boolean-toggle-row` (off/on) | `>` when selected | `< >` / `<*>` | `< > Label` / `<*> Label` | yes | `Space` toggles only | reverse or shared fallback | no `--->` |
| `exclusive-choice-row` (off/on) | `>` when selected | `< >` / `<*>` | `< > Label` / `<*> Label` | yes | `Space` toggles group selection | reverse or shared fallback | must be inside a group template |
| `exclusive-choice-entry-row` (off/on) | `>` when selected | `< >` / `<*>` | `< > Label --->` / `<*> Label --->` | yes | `Space` toggles group selection; `Enter` enters detail | reverse or shared fallback | selection and detail are split responsibilities |
| `multi-select-row` (off/on) | `>` when selected | `[ ]` / `[*]` | `[ ] Label` / `[*] Label` | yes | `Space` toggles only | reverse or shared fallback | must not be combined with `--->` |
| `field-entry-row` | `>` when selected | blank fixed-width prefix column | `Label (value) --->` | yes | `Enter` opens popup | reverse or shared fallback | prefix column remains visually aligned |
| `action-row` | `>` when selected | blank fixed-width prefix column | `Label --->` | yes | `Enter` opens submenu / popup / confirm chain | reverse or shared fallback | used for navigation and managed flows |
| `blocked-action-row` | none | `---` | `--- Label (blocked: reason)` | no | none | none | must expose blocking reason |
| `required-readonly-row` | none | `---` | `--- Label = required` | no | none | none | fixed by policy/system |
| `info-row` empty-state variant | none | `---` | `--- Message` | no | none | none | empty-state and hint rows |

## Field-To-Template Mapping (OMV Init)

This table maps `omv init` fields and entry families to canonical row grammar.
New `omv` menu fields should match one of these mappings.

| Scope | Field / Entry | Template Mapping | Canonical Prefix | Canonical Grammar | Keys |
| --- | --- | --- | --- | --- | --- |
| Init Root | detected repository root | `info-row` | `---` | `--- Project Root = value` | none |
| Init Root | detection summary | `info-row` | `---` | `--- Detection = value` | none |
| Init Root | timezone | `field-entry-row` + `choice-list-modal` | blank prefix column | `Timezone (value) --->` | `Enter` opens choice popup |
| Init Root | project profile recommendation | `field-entry-row` + `choice-list-modal` | blank prefix column | `Project Profile (value) --->` | `Enter` opens choice popup |
| Init Root | version output mode | `field-entry-row` + `choice-list-modal` | blank prefix column | `Version Output (value) --->` | `Enter` opens choice popup |
| Init Root | build number policy | `field-entry-row` + `choice-list-modal` | blank prefix column | `Build Policy (value) --->` | `Enter` opens choice popup |
| Init Root | use NTP validation | `boolean-toggle-row` | `< >` / `<*>` | `< > Use NTP Validation` / `<*> Use NTP Validation` | `Space` toggles, `Enter` does not toggle |
| Init Root | skip NTP for current run | `boolean-toggle-row` | `< >` / `<*>` | `< > Skip NTP For This Init` / `<*> Skip NTP For This Init` | `Space` toggles, `Enter` does not toggle |
| Language Support | discovered language entry | `multi-select-row` | `[ ]` / `[*]` | `[ ] Rust` / `[*] Rust` | auto-discovered entries start selected; `Space` toggles |
| Language Support | manually available language entry | `multi-select-row` | `[ ]` / `[*]` | `[ ] Python` / `[*] Python` | user may enable even when not detected; `Space` toggles |
| Language Support | no language detected hint | `info-row` | `---` | `--- No project manifests detected yet` | none |
| Language Support | choose pre-project behavior | `action-row` | blank prefix column | `Missing Project Strategy --->` | `Enter` opens strategy popup |
| Generated Outputs | runtime artifact mode | `field-entry-row` + `choice-list-modal` | blank prefix column | `Runtime Export Mode (value) --->` | `Enter` opens choice popup |
| Generated Outputs | native manifest sync mode | `field-entry-row` + `choice-list-modal` | blank prefix column | `Manifest Sync Mode (value) --->` | `Enter` opens choice popup |
| AI Integration | skill generation | `boolean-toggle-row` | `< >` / `<*>` | `< > Generate AI Skills` / `<*> Generate AI Skills` | `Space` toggles, `Enter` does not toggle |
| AI Integration | skill framework guidance | `action-row` | blank prefix column | `AI Integration Guidance --->` | `Enter` opens detail/help flow |
| Review | save initialization | `action-row` | blank prefix column | `Initialize OMV --->` | `Enter` opens confirm/result flow |
| Review | invalid state | `blocked-action-row` | `---` | `--- Initialize OMV (blocked: reason)` | none |

## OMV Init Popup Mapping

| Init Need | Popup Template | Contract |
| --- | --- | --- |
| timezone selection | `choice-list-modal` | user chooses timezone; recommended default may be preselected |
| project profile selection | `choice-list-modal` | profile controls recommendation text, not hidden behavior |
| build policy selection | `choice-list-modal` | `daily-reset` and `continuous` must both be visible |
| pre-project strategy selection | `choice-list-modal` | must offer all three user-selectable strategies before init completes |
| missing-manifest explanation | `message-modal` | explains why sync targets are not yet materialized |
| initialize confirmation | `confirm-modal` | confirms `.omv` creation and target registration choices |
| discovery in progress | `waiting-modal` | shown while auto-detection runs |
| initialization result | `result-modal` | summarizes files written, targets registered, and next command |

## Pre-Project Strategy Contract

When `omv init` runs before any project manifests exist and the user has
selected one or more language supports, the flow must present an explicit
strategy chooser popup. The operator must be able to choose exactly one of:

1. Record support intent only
2. Initialize runtime export templates without touching native manifests
3. Create minimal language project scaffolding

This choice must be persisted in `.omv` target registration or init state so
later `omv sync` behavior is explainable and reproducible.

## Navigation And Key Matrix

| Context | `↑/↓` | `←/→` | `Space` | `Enter` | `Esc` |
| --- | --- | --- | --- | --- | --- |
| Main menu list | move across actionable rows only, with wrap | footer button focus | toggle `[ ]/[*]` and `< >/<*>` state when applicable | follow `--->` only; no-arrow single-choice rows do not toggle on `Enter` | back one level, or exit from root |
| Footer button bar | no effect on list index | move across footer buttons | no semantic action | activate current footer button | same as screen-level `Esc` |
| Confirm popup | none | move across popup buttons | optional alias of confirm only when explicitly allowed | activate current popup button | close/cancel popup |
| Choice popup | move across choice rows | none | confirm current choice | confirm current choice | close/cancel popup |
| Text input popup | none | move caret | insert space if text input allows it | commit input | close/cancel popup |
| Discovery waiting popup | none | none | none | none | cancel probe and close waiting popup when supported |
| Result popup | none | none | none | close result popup | close result popup |

## Minimum Viewport And Overflow Rules

- Minimum supported viewport for the canonical menu layout is `80x24`.
- Below `80x24`, the UI must show a resize-required overlay instead of trying
  to render the normal menu layout.
- Menu rows must remain single-line. They must never wrap onto multiple lines.
- When a row is too long, preserve:
  - the left focus marker column
  - the semantic prefix column
  - the right-side `--->` suffix when present
- Truncate the middle text with `...` or an equivalent controlled ellipsis.
- Status/help text may wrap within their own blocks.

## Known Current Deviations

No known deviations currently remain for the `omv` menuconfig contract.

## Maintenance Rules

- Any menuconfig feature change that alters row grammar, popup behavior, key
  semantics, fallback rendering, viewport rules, or focus behavior must update
  this matrix and the corresponding `omv` product spec or PRD.
- This document records how `omv` menuconfig must look and behave.
- A change that introduces or modifies menuconfig rows, popups, or key
  semantics must not be considered complete until this matrix is aligned with
  the implementation.
