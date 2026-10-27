# Integration Contract and Storage Foundation

## Parent Task

`.trellis/tasks/04-18-platformized-host-integrations`

## Goal

Create the backend foundation for platformized host integrations without
implementing UI flows or installation execution. This task defines integration
providers, medium-grained capabilities, detection/status state, and
`.omv/integrations.toml` persistence in a shape that aligns with the existing
contract-driven architecture.

## Scope

- Add typed integration provider and capability models for the MVP:
  - `codex`
  - `trellis`
  - `project-instructions`
  - `host-skill`
  - `spec-guide`
  - `spec-index-snippet`
  - `finalize-boundary`
- Extend the contract/capability model so integration capabilities are visible
  alongside target and command capabilities without mixing the enum domains.
- Add `.omv/integrations.toml` load/save support.
- Persist selected provider/capability state plus the last known provider-level
  detection snapshot.
- Define stable capability status and failure reason structures:
  `selected`, `pending`, `installed`, `failed` plus stable reason code and
  display message.
- Provide tests for round-trip serialization, missing-file default behavior,
  malformed TOML errors, and registry capability mapping.

## Non-Goals

- Do not implement `omv integrate status` or `omv integrate apply`.
- Do not mutate host files.
- Do not change `omv init` UI screens.
- Do not implement finalize-boundary helper execution.
- Do not replace existing `adapter` command behavior.

## Ownership

Owned files/modules:

- `src/core/integration.rs` or `src/core/integration/mod.rs`
- `src/storage/integrations.rs`
- `src/contract/registry.rs`
- `proto/omv/contract/v1/contract.proto` only if generated contract values are
  needed for integration capability visibility
- `src/core/mod.rs`
- `src/storage/mod.rs`
- focused unit tests in touched modules

Avoid editing these except for compile integration if absolutely required:

- `src/cli/mod.rs`
- `src/app/mod.rs`
- `src/ui/**`
- `src/adapter.rs`
- `.agents/skills/finish-work/SKILL.md`

## Acceptance Criteria

- [ ] `.omv/integrations.toml` can be loaded when present and treated as empty
      when absent.
- [ ] The storage layer writes deterministic TOML using existing atomic write
      conventions.
- [ ] The model separates provider identity, capability identity, desired
      selection, observed detection, install status, and failure reason.
- [ ] The MVP registry exposes Codex and Trellis provider descriptors without
      hard-coding `codex + trellis` as a pair type.
- [ ] Integration capabilities are represented in the contract registry without
      reusing target capability enums.
- [ ] Tests cover round-trip persistence, absent file behavior, malformed file
      errors, and capability registry contents.
- [ ] `cargo fmt --check` and relevant Rust tests pass in the worker workspace.

## Required Context

- `.trellis/tasks/04-18-platformized-host-integrations/prd.md`
- `docs/OMV_CONTRACT_ARCHITECTURE.md`
- `.trellis/spec/backend/directory-structure.md`
- `.trellis/spec/backend/database-guidelines.md`
- `.trellis/spec/backend/error-handling.md`
- `.trellis/spec/backend/quality-guidelines.md`
- `.trellis/spec/guides/cross-layer-thinking-guide.md`

