# Trellis multi-agent distribution model (confirmed from docs + source)

Sources: https://docs.trytrellis.app/ , https://github.com/mindfold-ai/Trellis
(`packages/cli/src/configurators/*.ts`, `templates/common/`, bundled `trellis-meta` platform-map).

## Core finding: per-agent copies, NOT one shared file

`trellis init` renders one canonical template into EACH agent's own directory.
Each agent reads ONLY its own directory.

| Agent | finish-work entrypoint | form | reads `.agents/skills/`? |
|-------|------------------------|------|--------------------------|
| Codex | `.agents/skills/trellis-finish-work/SKILL.md` (v0.5) / `.agents/skills/finish-work/SKILL.md` (v0.4) | skill | writes + reads it |
| Gemini | `.agents/skills/...` | skill | reads it |
| Claude Code | `.claude/commands/trellis/finish-work.md` | slash command | **NO — only `.claude/`** |
| OpenCode | `.opencode/commands/trellis/finish-work.md` | command | only via agentskills.io fallback |
| Cursor | `.cursor/commands/trellis-finish-work.md` | command | NO |
| Kiro | `.kiro/skills/trellis-finish-work/SKILL.md` | skill | NO |
| Copilot | `.github/prompts/finish-work.prompt.md` | prompt | NO |

- `.agents/skills/` is the **agentskills.io class-2 shared layer** (Codex writes it,
  Gemini/Amp/Cline etc. can read it). It is NOT universal. Claude Code (class-1) never reads it.
- class-1 = full hooks + skills + push context (Claude, Cursor, CodeBuddy, Droid).
  class-2 = pull-based prelude, limited hooks (Codex, Gemini) — these write `.agents/skills/`.
- `finalize-boundary` / `OMV-MANAGED` are NOT Trellis concepts — they are OMV's own injection.

## Observed state in xpeng-debug-bridge (Trellis 0.5.19)

All finish-work files are regular files (no symlinks). md5 confirmed:
- `.claude/commands/trellis/finish-work.md` and `.opencode/commands/trellis/finish-work.md`
  are **byte-identical** and contain **0** OMV blocks (66 lines, 4 plain steps).
- `.agents/skills/trellis-finish-work/SKILL.md` (86 lines) is the ONLY file with the
  OMV finalize block (`OMV-MANAGED-BEGIN:spec-trellis-finalize-boundary-finish-work`).
- `.kiro/skills/trellis-finish-work/SKILL.md` (71 lines) has NO OMV block either.

## Implication for OMV

OMV's `finalize-boundary` capability injects only into `.agents/skills/...` (constants
`TRELLIS_FINISH_WORK_V05_PATH` / `_V04_PATH` in `src/app/mod.rs:160-161`). Therefore:
- **Codex** sees the block → triggers `omv event finalize-boundary` on finish-work. ✅
- **OpenCode** "worked" only by the fragile agentskills.io fallback to `.agents/skills/`. ⚠️
- **Claude Code** never reads `.agents/skills/` → never triggers. ❌

Fix must inject the same managed block into each command-type agent's OWN finish-work
entrypoint, removing reliance on the `.agents/skills/` implicit fallback.
