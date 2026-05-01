# State Management

> How operator-facing state is managed in `omv`.

---

## Overview

The project has three state categories and they must not be mixed.

## State Categories

| Category | Owner | Examples |
| --- | --- | --- |
| Persistent state | `.omv/` files via backend storage | locale, timezone, build policy, targets |
| Command runtime state | app/backend orchestration | detected manifests, validated current date, sync results |
| TUI draft state | `src/ui/state/` | toggled languages, locale/timezone/build-policy selection, popup selection, unsaved edits |

Integration state follows the same separation:

| Category | Owner | Examples |
| --- | --- | --- |
| Persistent integration state | `.omv/integrations.toml` via backend storage | selected providers, selected capabilities, last detection snapshot, capability status/failure |
| Runtime integration plan | app/backend orchestration | fresh provider detection, affected files, targeted worktree-safety result |
| TUI integration draft | `src/ui/state/` | temporary provider/capability toggles before init confirmation |

## When to Use Global State

Use shared app-level state only for:

- current screen
- focus/footer selection
- popup state
- init draft aggregate state

Do not promote per-widget rendering details to app-global state unless multiple
screens actually share them.

Integration review draft state should store typed provider/capability
selections separately from language target selections. Persist only after the
user confirms init. If targeted integration files are unsafe, init should still
save `.omv/integrations.toml` and tell the user to run `omv integrate apply`
later instead of mutating files from UI state.

## Server State

There is no server state in V1. NTP responses and auto-discovery results are
ephemeral command/runtime state and should be cached only as needed for the
current command.

## Common Mistakes

### Don't: Mutate `.omv` during every keypress

TUI edits should stay in draft state until the user confirms initialization or
save.

### Don't: Recompute discovery/network work during render

Compute once through a command path, then store the result in runtime state.

### Don't: Apply integrations from key handlers

Integration apply belongs to backend/app orchestration after confirmation and
targeted safety checks. The TUI only edits draft selections and displays the
review result.
