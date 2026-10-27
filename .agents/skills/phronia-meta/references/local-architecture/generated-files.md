# Local Files Generated After Init

`phronia init` writes the Phronia runtime into the user project. Later, `phronia update` tries to update Phronia-managed template files, but it uses `.phronia/.template-hashes.json` to determine which files have already been modified by the user.

This page only describes files that are visible and editable inside the user project.

## `.phronia/`

```text
.phronia/
├── workflow.md
├── config.yaml
├── .developer
├── .version
├── .template-hashes.json
├── .runtime/
├── scripts/
├── spec/
├── tasks/
└── workspace/
```

| Path | Usually editable? | Notes |
| --- | --- | --- |
| `.phronia/workflow.md` | Yes | Local workflow documentation and AI routing rules. |
| `.phronia/config.yaml` | Yes | Project configuration, hooks, packages, journal line limits, and related settings. |
| `.phronia/spec/` | Yes | Project specs, intended to be updated regularly by users and AI. |
| `.phronia/tasks/` | Yes | Task material and research artifacts, maintained by the task workflow. |
| `.phronia/workspace/` | Yes | Session records, usually written by `add_session.py`. |
| `.phronia/scripts/` | Carefully | Local runtime. It can be customized, but only after understanding the call chain. |
| `.phronia/.runtime/` | No | Runtime state, usually written automatically by hooks/scripts. |
| `.phronia/.developer` | Carefully | Current developer identity. |
| `.phronia/.version` | No | Phronia version record used by update/migration logic. |
| `.phronia/.template-hashes.json` | No | Template hash record. Do not hand-write business rules here. |

## Platform Directories

Different platforms generate different directories. Common categories:

| Category | Example paths | Purpose |
| --- | --- | --- |
| hooks | `.claude/hooks/`, `.codex/hooks/`, `.cursor/hooks/` | Inject session context, workflow-state, and sub-agent context. |
| settings | `.claude/settings.json`, `.codex/hooks.json`, `.qoder/settings.json`, `.trae/hooks.json` | Tell the platform when to run hooks or plugins. |
| agents | `.claude/agents/`, `.codex/agents/`, `.kiro/agents/`, `.zcode/agents/` | Define agents such as `phronia-research`, `phronia-implement`, and `phronia-check`. |
| skills | `.claude/skills/`, `.agents/skills/`, `.qoder/skills/`, `.zcode/skills/` | Skills that auto-trigger or can be read by AI. |
| commands/prompts/workflows | `.cursor/commands/`, `.github/prompts/`, `.devin/workflows/`, `.zcode/commands/` | Explicit user-invoked command or workflow entry points. |

When modifying a platform directory, also confirm whether `.phronia/workflow.md` still describes the same flow.

## Meaning Of Template Hashes

`.phronia/.template-hashes.json` records the content hash from the last time Phronia wrote a template file. `phronia update` uses it to distinguish three cases:

| Case | Update behavior |
| --- | --- |
| File was not modified by the user | It can be updated automatically. |
| File was modified by the user | Prompt the user to overwrite, keep, or generate `.new`. |
| File is no longer a current template | It may be deleted, renamed, or preserved according to migration rules. |

When an AI customizes local Phronia files, it does not need to maintain hashes manually. It is normal for Phronia update to recognize the result as "modified by the user."

## Local Customization Boundaries

Editable by default:

- `.phronia/workflow.md`
- `.phronia/config.yaml`
- `.phronia/spec/**`
- `.phronia/scripts/**`
- Platform hooks, settings, agents, skills, commands, prompts, and workflows

Do not edit by default:

- Global npm install directory
- `node_modules/@phronia/cli`
- Phronia GitHub repository source code
- Concrete state files under `.phronia/.runtime/**`
- Hash contents inside `.phronia/.template-hashes.json`

Switch to the Phronia CLI source-code perspective only when the user explicitly wants to contribute upstream.
