# Integrate Status and Apply Planning Engine

## Parent Task

`.trellis/tasks/04-18-platformized-host-integrations`

## Goal

Implement the post-init command surface for host integrations:
`omv integrate status` and `omv integrate apply`. The implementation must follow
the contract-driven architecture's plan-before-mutate pattern and consume the
integration foundation from the storage/registry task.

## Scope

- Add CLI parsing for:
  - `omv integrate status`
  - `omv integrate apply`
- Add an internal integration plan model that computes provider/capability
  status before mutation.
- Always re-detect supported providers before `apply`.
- Implement provider-specific bootstrap policy for MVP:
  - Codex can bootstrap lightweight instruction host files.
  - Trellis requires an existing Trellis installation before mutation.
- Apply selected/pending capabilities by projecting existing `.omv/ai/*`
  adapter content where applicable.
- Implement targeted worktree-safety checks over integration-affected files.
- Preserve successful capability installs when other capabilities fail.
- Return non-zero from `apply` if any selected capability fails.
- Render status/apply output as text and structured JSON using existing output
  envelope conventions.

## Non-Goals

- Do not add init UI selection/review.
- Do not create the Trellis finalize-boundary helper internals; call out that
  capability as pending/unsupported until the helper task provides it.
- Do not redesign existing target sync planning.
- Do not remove legacy `omv adapter` commands.

## Ownership

Owned files/modules:

- `src/cli/mod.rs`
- `src/app/mod.rs`
- `src/errors.rs` for integration-specific errors
- `resources/i18n/en-US.toml`
- `resources/i18n/zh-CN.toml`
- `tests/integration.rs` or focused integration tests for CLI behavior

May consume but should avoid structurally redesigning:

- `src/core/integration.rs`
- `src/storage/integrations.rs`
- `src/contract/registry.rs`
- `src/adapter.rs`

Do not edit:

- `src/ui/**`
- `.agents/skills/finish-work/SKILL.md`

## Acceptance Criteria

- [ ] `omv integrate status --json` reports provider + capability matrix.
- [ ] `omv integrate apply --json` reports per-capability success/failure and
      uses a non-zero exit path for any failed selected capability.
- [ ] Apply re-detects the workspace before mutation.
- [ ] Apply uses targeted safety checks over the files it would mutate.
- [ ] Successful capability installs are persisted even when another selected
      capability fails.
- [ ] Text output is localized through catalog keys, not inline English-only
      strings.
- [ ] Tests cover status with no integrations file, apply success, partial
      failure, non-detected Trellis policy, and JSON envelope shape.
- [ ] `cargo fmt --check` and relevant Rust tests pass in the worker workspace.

## Required Context

- `.trellis/tasks/04-18-platformized-host-integrations/prd.md`
- `docs/OMV_CONTRACT_ARCHITECTURE.md`
- `.trellis/spec/backend/directory-structure.md`
- `.trellis/spec/backend/database-guidelines.md`
- `.trellis/spec/backend/error-handling.md`
- `.trellis/spec/backend/localization-guidelines.md`
- `.trellis/spec/backend/quality-guidelines.md`
- `.trellis/spec/guides/cross-layer-thinking-guide.md`

