# Extend OMV finalize-boundary to all command-type agent finish-work entrypoints

## Goal

OMV's Trellis `finalize-boundary` capability injects its managed block ONLY into
`.agents/skills/trellis-finish-work/SKILL.md` (the agentskills.io / Codex-Gemini layer).
Claude Code never reads `.agents/skills/` — it reads `.claude/commands/trellis/finish-work.md`
— so running `/trellis:finish-work` under Claude never triggers the OMV version bump.
OpenCode only "works" by a fragile agentskills.io fallback to the same shared file. This task
makes finalize-boundary inject into the OWN finish-work entrypoint of each **selected** agent
(claude/opencode/codex), so the OMV trigger is consistent and explicit per the user's
opted-in agents.

## What I already know (confirmed by research + source)

- Trellis distributes per-agent copies; each agent reads only its own dir. See
  [`research/trellis-agent-distribution.md`](research/trellis-agent-distribution.md).
- Command-type finish-work entrypoints (each agent's own file):
  - Claude Code: `.claude/commands/trellis/finish-work.md`
  - OpenCode:    `.opencode/commands/trellis/finish-work.md`
  - Cursor:      `.cursor/commands/trellis-finish-work.md`
- Skill-type (already covered / out of scope shift): `.agents/skills/trellis-finish-work/SKILL.md`
  (v0.5) and `.agents/skills/finish-work/SKILL.md` (v0.4) — Codex/Gemini.
- In xpeng-debug-bridge, `.claude/...` and `.opencode/...` finish-work commands are
  byte-identical and contain **0** OMV blocks; only `.agents/skills/...` has the block.
- OMV side (this repo):
  - `src/app/mod.rs:160-167` — `TRELLIS_FINISH_WORK_V05_PATH` / `_V04_PATH` / backups.
  - `integration_target()` maps `(Trellis, FinalizeBoundary)` → single `host_rel`
    = `TRELLIS_FINISH_WORK_V05_PATH`, behavior `TrellisFinalizeBoundary`.
  - `resolve_trellis_finish_work_path()` picks ONE path (v05/v04) by existence + block.
  - `install_integration_target()` upserts the block via the pure fn
    `adapter::upsert_trellis_finish_work_finalize_block(existing)` (reusable for any file).
  - `probe_trellis_finalize_boundary()` + `trellis_finalize_boundary_mismatch()` report
    installed/pending/failed and detect drift/backup-only states.

## Decisions (from brainstorm)

- **Scope**: cover exactly three agents — **claude, opencode, codex**. (Cursor and other
  agents are out of scope for now.) OpenCode is explicitly included — its current behavior
  relies on a fragile `.agents/skills/` fallback and must get an explicit injection too.
- **Agent → finish-work entrypoint mapping**:
  - claude   → `.claude/commands/trellis/finish-work.md`
  - opencode → `.opencode/commands/trellis/finish-work.md`
  - codex    → `.agents/skills/trellis-finish-work/SKILL.md` (v0.5) /
    `.agents/skills/finish-work/SKILL.md` (v0.4) — i.e. the EXISTING `.agents/skills/` path.
- **Trigger condition**: inject into an agent's finish-work entrypoint **only when that agent
  provider is SELECTED** in `.omv/integrations.toml` (chosen during `omv init`, or selected
  later and re-applied via `omv integrate apply`). finalize-boundary's effective target set =
  union, over selected agent providers, of each one's finish-work entrypoint. This avoids
  writing into agents the user did not opt into. (Note: codex's entrypoint == the existing
  `.agents/skills/` path, so prior behavior is preserved when codex is selected.)

## Requirements

1. Define an agent→entrypoint map for the three in-scope agents (claude, opencode, codex)
   per the mapping above. codex resolves to the existing v0.5/v0.4 `.agents/skills/` paths.
2. finalize-boundary is a **Trellis** capability, but its target set is now derived from the
   **selected agent providers** in `.omv/integrations.toml`. At apply time:
   - compute the union of finish-work entrypoints for all selected in-scope agents;
   - for each entrypoint that EXISTS, upsert the OMV block (reuse
     `upsert_trellis_finish_work_finalize_block`, idempotent);
   - do NOT touch entrypoints of unselected agents.
3. `omv integrate apply` reports finalize-boundary `installed` only when every required
   entrypoint (selected agent + file exists) carries the block; surface a partial/mismatch
   state otherwise (extend `probe_trellis_finalize_boundary`).
4. Worktree-safety + idempotency: re-running apply is a no-op on already-injected files;
   safety check covers all resolved target files.
5. Preserve existing v0.5/v0.4 `.agents/skills/` behavior and backup/drift detection for the
   codex path.
6. Failure / empty cases:
   - selected agent's entrypoint file does not exist → treat consistent with today's
     "no finish-work surface" handling (pending/failure with actionable message), per agent.
   - Trellis not detected → unchanged (finalize-boundary already gated on Trellis detection).
7. Backward compatibility: a project where only codex is selected must behave exactly as
   today (block in `.agents/skills/...`, same resolution/precedence).

## Acceptance Criteria

- [ ] With Trellis detected and claude+opencode+codex all selected, `omv integrate apply`
      injects the OMV finalize block into `.claude/commands/trellis/finish-work.md`,
      `.opencode/commands/trellis/finish-work.md`, and `.agents/skills/trellis-finish-work/SKILL.md`,
      each exactly once (idempotent on re-apply).
- [ ] When only claude is selected, only `.claude/...` is injected; `.opencode/...` and
      `.agents/skills/...` are left untouched — proves selection-gating works and the Claude
      gap is closed.
- [ ] When only codex is selected, behavior is identical to today (block in `.agents/skills/...`
      with v0.5/v0.4 precedence + backup/drift detection).
- [ ] `omv integrate status --json` reports finalize-boundary `installed` only when every
      required (selected-agent + file-exists) entrypoint carries the block; otherwise a clear
      pending/mismatch naming the offending path(s).
- [ ] OpenCode entrypoint gets an explicit block (no longer relies on the `.agents/skills/`
      fallback) when opencode is selected.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets --all-features -D warnings`,
      `cargo test` all green; new tests cover selection-gated multi-target inject,
      single-agent injection, idempotency, and partial/mismatch status.

## Definition of Done

- Tests added for: claude-only inject, multi-entrypoint inject, idempotent re-apply,
  partial/mismatch status, and the no-surface failure path.
- Lint/typecheck/build green.
- Docs/spec updated: backend spec note on finalize-boundary target set; README if it
  documents finalize-boundary surfaces.
- OMV finalize-boundary handled at finish-work for this repo's own version bump.

## Technical Approach

Convert the finalize-boundary target model from a single `host_rel` to a resolved LIST of
finish-work entrypoints derived from the **selected agent providers**. Add an agent→entrypoint
mapping for claude/opencode/codex (codex = existing v0.5/v0.4 `.agents/skills/` resolution).
At apply, read `.omv/integrations.toml`, take the selected in-scope agents, resolve each to its
entrypoint, and upsert the shared managed block into every one that exists (reusing the
idempotent pure fn). Extend `probe_trellis_finalize_boundary` to aggregate per-file block
presence across the required set into one capability status, reusing the existing
mismatch/backup reporting for diagnostics.

Key tension 1: `IntegrationTarget` carries one `host_rel`. finalize-boundary is already
special-cased (`TrellisFinalizeBoundary`), so multi-file handling lives inside that special
case (apply/probe/safety branches) without changing the single-`host_rel` shape for other
capabilities.

Key tension 2: finalize-boundary belongs to the **Trellis** provider, but its target set now
depends on **agent** provider selection — a cross-provider read. The apply/probe code already
has access to integration state; resolve the selected-agent set there. Keep this dependency
explicit and localized to the finalize-boundary branches.

## Decision (ADR-lite)

**Context**: finalize-boundary only reached `.agents/skills/`, so Claude (and fragilely
OpenCode) didn't trigger OMV version bumps on finish-work.
**Decision**: Generalize finalize-boundary to inject into the finish-work entrypoint of each
**selected** agent (claude/opencode/codex), gated on agent provider selection in
`.omv/integrations.toml`; injection happens at `omv init` apply or a later
`omv integrate apply` after selection.
**Consequences**: finalize-boundary becomes selection-driven multi-target inside its
special-case branches; cross-provider read (Trellis capability ← agent selection); richer
status aggregation; removes reliance on the agentskills.io fallback. Backward-compatible:
codex-only selection == today's `.agents/skills/` behavior.

## Out of Scope

- Agents other than claude/opencode/codex (Cursor, Kiro, Copilot, Gemini, etc.).
- Injecting into agents the user did NOT select.
- Skill-type non-OMV agents that lack the block by Trellis design (e.g. `.kiro/skills/...`).
- Changing Trellis itself or its templates.
- The separate "add Claude agent-host support" work (already shipped).
- Auto-running finalize on finish-work (that's the agent following the injected block;
  OMV only projects the guidance).

## Technical Notes

- Reuse `adapter::upsert_trellis_finish_work_finalize_block` (idempotent; test at
  `src/adapter.rs:1015-1016`).
- Touch points: `src/app/mod.rs` finalize-boundary constants (160-167),
  `integration_target` (1445), `resolve_integration_target`/`resolve_trellis_finish_work_path`
  (1453-1481), `install_integration_target` TrellisFinalizeBoundary branch (1610-1624),
  `probe_trellis_finalize_boundary` (1511-1565), `check_integration_target_safety` (1573).
- Block identifier: `spec-trellis-finalize-boundary-finish-work`
  (`adapter::TRELLIS_FINISH_WORK_BLOCK_NAME`).
- i18n: any new status/failure copy must use catalog keys (backend localization spec).
