# Adapter Compatibility and Integration Docs

## Parent Task

`.trellis/tasks/04-18-platformized-host-integrations`

## Goal

Close the platformized host integration transition by keeping existing
`omv adapter ...` behavior compatible during the MVP, aligning generated AI
instructions with the new integration surface, and updating product/spec docs
to match the implemented architecture.

## Scope

- Keep existing `omv adapter install/refresh/list/status` usable as temporary
  compatibility aliases or wrappers where behavior overlaps with
  `omv integrate ...`.
- Update `.omv/ai/instructions.md` and adapter templates so host guidance points
  at the new plan/check/integrate/finalize-boundary contracts without making
  host files authoritative.
- Update specs/docs to reflect:
  - `.omv/integrations.toml`
  - integration provider/capability model
  - `omv integrate status/apply`
  - finalize-boundary helper contract
  - adapter compatibility transition
- Ensure docs distinguish:
  - target adapters
  - AI/spec host projections
  - integration providers/capabilities
  - completion-boundary automation
- Add or update tests for legacy adapter command compatibility where feasible.

## Non-Goals

- Do not implement core integration storage or apply logic from scratch.
- Do not change init UI behavior directly.
- Do not remove any existing adapter command in MVP.
- Do not publish a third-party plugin runtime.

## Ownership

Owned files/modules:

- `src/adapter.rs`
- `.omv/ai/**` generated source expectations/templates as represented in code
- `.trellis/spec/backend/**`
- `.trellis/spec/frontend/**` where init UI docs need alignment
- `.trellis/spec/guides/**`
- `docs/OMV_CONTRACT_ARCHITECTURE.md`
- `docs/examples/**` if examples need integration references
- focused tests for adapter compatibility if command behavior changes

May touch lightly:

- `src/cli/mod.rs`
- `src/app/mod.rs`

Do not edit:

- `src/ui/**` except docs-only references
- `src/storage/integrations.rs` except docs/test naming fixes agreed with the
  foundation worker

## Acceptance Criteria

- [ ] Legacy `omv adapter ...` commands remain available and have a clear MVP
      compatibility path.
- [ ] Generated `.omv/ai/contract.json` and instructions mention the new
      integration/finalize-boundary surface where implemented.
- [ ] Backend specs document `.omv/integrations.toml`, provider capabilities,
      status/failure semantics, and plan/apply behavior.
- [ ] Frontend specs document the init integration review flow.
- [ ] Cross-layer guide includes integration/finalize-boundary data flow.
- [ ] Docs do not describe public plugin runtime as implemented in MVP.
- [ ] Docs avoid treating host adapter files as sources of truth.
- [ ] Relevant tests/docs checks pass in the worker workspace.

## Required Context

- `.trellis/tasks/04-18-platformized-host-integrations/prd.md`
- `docs/OMV_CONTRACT_ARCHITECTURE.md`
- `.trellis/spec/backend/index.md`
- `.trellis/spec/backend/directory-structure.md`
- `.trellis/spec/backend/database-guidelines.md`
- `.trellis/spec/backend/error-handling.md`
- `.trellis/spec/backend/localization-guidelines.md`
- `.trellis/spec/backend/quality-guidelines.md`
- `.trellis/spec/frontend/index.md`
- `.trellis/spec/frontend/component-guidelines.md`
- `.trellis/spec/frontend/state-management.md`
- `.trellis/spec/frontend/type-safety.md`
- `.trellis/spec/guides/cross-layer-thinking-guide.md`
- `.trellis/spec/guides/code-reuse-thinking-guide.md`

