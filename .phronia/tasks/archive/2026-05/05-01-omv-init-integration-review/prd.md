# Init Integration Review Flow

## Parent Task

`.trellis/tasks/04-18-platformized-host-integrations`

## Goal

Add host integration selection and review to `omv init` while preserving the
existing menuconfig-style target setup flow. Init should save integration state,
show a mandatory review screen, and attempt automatic installation only when the
selected integration plan is safe to apply.

## Scope

- Extend init draft state with integration provider/capability selections.
- Detect supported MVP providers during init:
  - Codex
  - Trellis
- Show all MVP-supported providers in the UI, with detected/recommended state.
- Preselect recommended finalize-boundary capability when Trellis is detected,
  while allowing explicit user override.
- Add mandatory integration review that shows:
  - selected providers
  - selected capabilities
  - target files per capability
- Persist selected integration state to `.omv/integrations.toml`.
- After init setup, invoke the integration apply path only when targeted
  worktree safety checks pass; otherwise leave state saved and instruct the user
  to run `omv integrate apply`.
- Keep all UI copy localized.

## Non-Goals

- Do not implement low-level integration storage/registry models from scratch;
  consume the foundation task's API.
- Do not implement provider installation logic directly in render/event code.
- Do not modify finalize-boundary helper internals.
- Do not change language target detection semantics except where needed to
  sequence the new review step.

## Ownership

Owned files/modules:

- `src/ui/state/draft.rs`
- `src/ui/discovery.rs`
- `src/ui/app.rs`
- `src/ui/runtime.rs`
- `src/ui/screen/**`
- `src/ui/widget/**`
- relevant init orchestration in `src/app/mod.rs`
- `resources/i18n/en-US.toml`
- `resources/i18n/zh-CN.toml`
- focused UI/state tests in touched modules

May consume but should avoid redefining:

- `src/core/integration.rs`
- `src/storage/integrations.rs`

Do not edit:

- `src/cli/mod.rs` except if needed for compile integration
- `.agents/skills/finish-work/SKILL.md`

## Acceptance Criteria

- [ ] Init draft state can represent provider/capability selections separately
      from language targets.
- [ ] Init UI includes a mandatory integration review step before any automatic
      install attempt.
- [ ] Review displays selected providers, selected capabilities, and target
      files.
- [ ] Codex and Trellis provider rows include detection/recommendation state.
- [ ] Finalize-boundary is recommended/preselected only when appropriate and can
      be toggled off.
- [ ] If targeted files are unsafe, init saves integration state and tells the
      user to run `omv integrate apply`.
- [ ] UI copy uses catalog keys in both supported locales.
- [ ] Tests cover draft defaults, toggles, review rendering/state, and safe vs
      unsafe automatic apply behavior where feasible.
- [ ] `cargo fmt --check` and relevant Rust tests pass in the worker workspace.

## Required Context

- `.trellis/tasks/04-18-platformized-host-integrations/prd.md`
- `docs/OMV_CONTRACT_ARCHITECTURE.md`
- `.trellis/spec/frontend/component-guidelines.md`
- `.trellis/spec/frontend/state-management.md`
- `.trellis/spec/frontend/type-safety.md`
- `.trellis/spec/frontend/quality-guidelines.md`
- `.trellis/spec/backend/localization-guidelines.md`
- `.trellis/spec/guides/cross-layer-thinking-guide.md`
- `docs/matrix/MENUCONFIG_STYLE_MATRIX.md`

