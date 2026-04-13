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
| TUI draft state | `src/ui/state/` | toggled languages, popup selection, unsaved edits |

## When to Use Global State

Use shared app-level state only for:

- current screen
- focus/footer selection
- popup state
- init draft aggregate state

Do not promote per-widget rendering details to app-global state unless multiple
screens actually share them.

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
