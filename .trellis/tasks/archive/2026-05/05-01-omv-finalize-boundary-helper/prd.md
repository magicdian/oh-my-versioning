# Finalize Boundary Helper for Trellis

## Parent Task

`.trellis/tasks/04-18-platformized-host-integrations`

## Goal

Add the generic OMV-native helper that host integrations can call at completion
boundaries, and implement the first MVP boundary for Trellis `finish-work`.
The helper derives deterministic finalize-task fields, while the host/agent
still supplies the semantic `change_type`.

## Scope

- Add a generic finalize-boundary helper command under the existing `event`
  namespace or another contract-aligned namespace if implementation research
  proves a better local fit.
- Accept structured boundary identity as provider + boundary name, then flatten
  to the existing legacy `source` string internally.
- Auto-resolve Trellis task identity from `.trellis/.current-task` by default,
  with explicit override support.
- Derive `status=done` and `tests=passed` for the helper-triggered
  `finalize-task` call.
- Require explicit `change_type` from the caller using the existing enum values:
  `bugfix`, `feature`, `refactor`, `docs`, `chore`.
- If `change_type` is missing, do not infer or default; return a structured
  pending/manual-action response.
- Build idempotency fingerprint from:
  - task identity
  - boundary identity
  - workspace snapshot hash
- Include `HEAD`, staged content deltas, unstaged content deltas, and untracked
  content deltas in the workspace snapshot hash.
- Normalize OMV-managed version outputs out of the snapshot hash using managed
  target metadata plus the narrow fixed core file set:
  - `.omv/state.toml`
  - `.omv/finalizations.toml`
  - `.omv/skills/README.md`
- Expose the helper contract in `.omv/ai/contract.json`.
- Install/update one OMV-managed block in the active platform-resolved Trellis
  `finish-work` surface, placed as a dedicated final checklist section before
  `Quick Check Flow`.

## Non-Goals

- Do not replace the existing `omv event finalize-task` contract.
- Do not implement automatic semantic change classification.
- Do not mutate every sibling Trellis command/skill representation; update only
  the active platform-resolved surface.
- Do not make `.omv/ai/*` a second source of truth.

## Ownership

Owned files/modules:

- `src/cli/mod.rs` for helper parsing if needed
- `src/app/mod.rs` helper orchestration
- `src/core/finalization.rs` only for reusable validation/helpers, not policy
  rewrites
- `src/adapter.rs` for `.omv/ai/contract.json` helper contract and managed
  block source generation
- `.agents/skills/finish-work/SKILL.md` managed block installation target
- `resources/i18n/en-US.toml`
- `resources/i18n/zh-CN.toml`
- focused tests for helper behavior and duplicate/idempotency behavior

May consume:

- `src/core/integration.rs`
- `src/storage/integrations.rs`
- target planning metadata from `src/sync/**`

Do not edit:

- `src/ui/**`
- broad target sync adapter behavior except for minimal normalization helper
  hooks required by the fingerprint contract

## Acceptance Criteria

- [ ] Helper accepts provider + boundary identity and maps it to the existing
      flat finalize `source`.
- [ ] Helper auto-resolves active Trellis task identity and supports explicit
      task override.
- [ ] Missing `change_type` returns a pending/manual-action response without
      calling `finalize-task`.
- [ ] Valid helper invocation calls the existing finalize-task path with
      `status=done` and `tests=passed`.
- [ ] Repeating the same workspace snapshot and boundary does not cause a
      second bump.
- [ ] OMV-managed version output normalization prevents finalize's own sync
      output from changing the fingerprint.
- [ ] `.omv/ai/contract.json` advertises the helper.
- [ ] The active Trellis finish-work surface receives exactly one idempotent
      OMV-managed block in the agreed placement.
- [ ] Tests cover missing change type, task auto-resolution, explicit override,
      duplicate invocation, and managed block idempotency.
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
- `.agents/skills/finish-work/SKILL.md`

