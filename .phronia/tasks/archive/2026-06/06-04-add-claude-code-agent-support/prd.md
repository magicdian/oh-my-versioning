# Add Claude Code Agent Support

## Goal

Promote **Claude (Claude Code)** from an MVP-hidden placeholder to a first-class agent
integration, on par with `codex` and `opencode`. Today Claude exists only as a partial
`AgentAdapter` (old `omv adapter` path: a `CLAUDE.md` projection, no host-skill) and is
explicitly hidden from the user-facing `omv integrate` MVP path (`IntegrationProvider`
does not include Claude; contract marks it `mvp_supported: false, hidden_from_init: true`).
This task wires Claude into the `omv integrate` path AND completes the old `omv adapter`
path so both stay consistent.

## What I already know

- The repo has **two parallel agent registries**:
  - `AgentAdapter` enum (`src/core/adapter.rs`) + `install_agent_adapter()`
    (`src/adapter.rs`) — old `omv adapter install/list/status` path. **Claude already
    present** here but only with the `CLAUDE.md` (project-instructions) target; no host-skill.
  - `IntegrationProvider` enum (`src/core/integration.rs`) + `mvp_provider_descriptors()`
    + `integration_target()` (`src/app/mod.rs`) — new `omv integrate status/apply` path,
    which is what `omv init`, discovery, and the UI actually use. **Claude absent entirely.**
- Capability→file projection for the new path is centralized in
  `src/app/mod.rs::integration_target()` (a `match (provider, capability)`).
- `mvp_supported` / `hidden_from_init` are **not enforced in Rust**; they are documentation
  in the static `contract.json`. Real enforcement = whether the provider is in the enum +
  `mvp_provider_descriptors()`.
- Canonical adapter source content is baked into `canonical_sources()` (`src/adapter.rs`)
  and written to `.omv/ai/adapters/...` on every init/integrate run.
- Existing tests already cover Claude managed-block behavior in the old path
  (`install_claude_into_existing_file_uses_managed_block`).

## Decisions (from brainstorm)

- **Scope**: Complete BOTH registries and keep them consistent (`omv integrate` MVP path
  is primary; old `omv adapter` path's Claude host-skill is also filled in).
- **Capabilities**: Claude gets BOTH `project-instructions` (→ `CLAUDE.md`) and
  `host-skill` (→ `.claude/skills/omv-versioning/SKILL.md`), aligned with codex/opencode.
- **Discovery condition**: Claude is "detected" when `.claude/` dir OR `CLAUDE.md` exists.

## Requirements

### New `omv integrate` path (`IntegrationProvider::Claude`)
1. Add `Claude` variant to `IntegrationProvider` enum; update `as_str()` + `parse()`.
2. Add a Claude descriptor to `mvp_provider_descriptors()`:
   - kind `AgentHost`, bootstrap `BootstrapLightweightHost`
   - `project-instructions` → `CLAUDE.md` (default_selected, recommended)
   - `host-skill` → `.claude/skills/omv-versioning/SKILL.md` (default_selected, recommended)
3. Add `integration_target()` arms:
   - `(Claude, ProjectInstructions)` → source `adapters/claude/CLAUDE.md`, host `CLAUDE.md`,
     behavior `FullFileOrManagedBlock`.
   - `(Claude, HostSkill)` → source `adapters/claude/SKILL.md`,
     host `.claude/skills/omv-versioning/SKILL.md`, behavior `DedicatedFile`.
4. Add detection arms: `discover_integrations()` (`src/ui/discovery.rs`) and
   `detect_integration_provider()` (`src/app/mod.rs`) → `.claude` dir OR `CLAUDE.md`.
5. Add UI label arm `provider_label()` (`src/ui/runtime.rs`) →
   `integration.provider.claude`; add the i18n catalog key(s).
6. `default_integration_selected()` stays Codex-only (Claude is opt-in, like opencode).
7. Audit every remaining exhaustive `match` over `IntegrationProvider` (incl.
   `src/ui/state/draft.rs`) and add Claude arms so the project compiles.

### Old `omv adapter` path consistency
8. Add Claude `host-skill` target to `install_agent_adapter()` (`src/adapter.rs`): add the
   `.claude/skills/omv-versioning/SKILL.md` `DedicatedFile` target alongside the existing
   `CLAUDE.md` target.

### Canonical sources & contract
9. Add `adapters/claude/SKILL.md` to `canonical_sources()` (mirror codex/opencode SKILL.md,
   with `source=.omv/ai/adapters/claude/SKILL.md` in the managed-file marker).
10. Update the static `contract.json` `providers.claude` entry to
    `mvp_supported: true`, drop `hidden_from_init`, add `bootstrap_policy` +
    `capabilities: ["project-instructions","host-skill"]` matching codex/opencode.

### Docs & spec
11. Update README agent enumeration so Claude is listed as supported (it already hints at
    `claude`; make it accurate).
12. Update `.trellis/spec/backend/index.md` design-decision note (currently: "Codex,
    OpenCode, and Trellis are the supported MVP providers; Claude and OpenSpec remain
    outside the init UI support matrix") to reflect Claude now being an MVP-supported
    agent provider (OpenSpec stays outside).

## Acceptance Criteria

- [ ] `omv integrate status --json` lists `claude` as an available agent-host provider with
      both capabilities and correct target paths.
- [ ] Selecting Claude + `omv integrate apply` writes `CLAUDE.md` (full file or managed block
      if pre-existing) and `.claude/skills/omv-versioning/SKILL.md` (dedicated managed file).
- [ ] `omv adapter install --agent claude` produces BOTH `CLAUDE.md` and
      `.claude/skills/omv-versioning/SKILL.md`.
- [ ] `omv adapter list` / `available_catalog()` still includes `claude` (unchanged).
- [ ] Discovery detects Claude when `.claude/` or `CLAUDE.md` is present.
- [ ] `contract.json` shows `claude` as `mvp_supported: true` with the two capabilities.
- [ ] New tests: integrate-path Claude apply (both capabilities) + old-path Claude host-skill;
      existing `install_claude_into_existing_file_uses_managed_block` still passes.
- [ ] `cargo build`, `cargo clippy`, `cargo test`, `cargo fmt --check` all green.

## Definition of Done

- Tests added/updated for both paths; all existing tests pass.
- Lint/typecheck/build green (`cargo clippy -- -D warnings`, `cargo test`, `cargo fmt`).
- README updated to reflect Claude as supported.
- OMV finalize-boundary handled at finish-work (this repo manages its own version via OMV).

## Out of Scope

- Public third-party plugin runtime (explicitly future work per contract `plugin_runtime`).
- Making Claude the default-selected provider (stays opt-in like opencode).
- OpenSpec promotion (separate hidden provider, not part of this task).
- Any change to Claude's instruction *content* beyond mirroring the codex/opencode SKILL.md
  structure.

## Technical Approach

Two-registry consistent extension. Primary work is the `omv integrate` path
(`IntegrationProvider::Claude` + descriptor + `integration_target()` arms + detection + UI
label), which is what `omv init`/discovery/UI consume. Secondary work fills the old
`omv adapter` path's missing Claude host-skill target so both paths install identical files.
A new canonical source `adapters/claude/SKILL.md` provides the host-skill content. The
`contract.json` provider block is flipped to MVP-supported. The compiler enforces most
arm additions via exhaustive matches; the runtime `parse()` string match and `contract.json`
are the non-compiler-checked spots to watch.

## Decision (ADR-lite)

**Context**: Claude was scaffolded but hidden; the user-facing path (`omv integrate`) never
knew about it, and even the old path only installed `CLAUDE.md`.
**Decision**: Promote Claude to a full `IntegrationProvider` with two capabilities, and
backfill the old `AgentAdapter` path's host-skill so both registries are consistent.
**Consequences**: ~7 match sites across 4 files gain a Claude arm; one new canonical source
file; contract + README updated. Low risk — reuses existing projection/managed-block
machinery; no new install behaviors introduced.

## Technical Notes

- Key files: `src/core/integration.rs`, `src/app/mod.rs` (`integration_target`,
  `detect_integration_provider`, `default_integration_selected`), `src/ui/discovery.rs`,
  `src/ui/runtime.rs`, `src/ui/state/draft.rs`, `src/core/adapter.rs` (already done),
  `src/adapter.rs` (`install_agent_adapter`, `canonical_sources`, contract JSON).
- i18n: provider labels resolved via catalog key `integration.provider.<name>`; add the
  `claude` key wherever `codex`/`opencode` keys live.
- Existing source file `.omv/ai/adapters/claude/CLAUDE.md` already present and correct.

### Upgrade path for already-initialized projects (zero-conflict)
- The provider list is **compiled into the `omv` binary** (`mvp_provider_descriptors()`),
  not read from `.omv/`. So an existing project picks up Claude only after upgrading to an
  `omv` build that includes this change.
- After upgrade, the user re-runs `omv init`; the TUI now lists `claude` as an agent option.
  Selecting it makes `persist_init_state` auto-apply the capabilities
  (`InitIntegrationApplyStatus::Applied`), writing `CLAUDE.md` +
  `.claude/skills/omv-versioning/SKILL.md` on the spot — no separate `omv integrate apply`
  needed.
- `normalize_integration_state()` (`src/app/mod.rs:1114`) only **appends** missing providers
  to `.omv/integrations.toml` (`if !state.providers.iter().any(...)`); existing
  codex/opencode/trellis selections are preserved untouched. Claude is appended as a new,
  default-unselected provider. **Re-init does not reset or lose prior config.**
- Equivalent non-TUI entry points must produce identical files: `omv integrate apply`
  (after selection) and the legacy `omv adapter install --agent claude`.
