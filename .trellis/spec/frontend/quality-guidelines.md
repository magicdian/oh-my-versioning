# Quality Guidelines

> Quality standards for `omv` CLI/TUI behavior.

---

## Overview

The UI is small, but correctness matters because it configures the version truth
source. Interaction bugs can corrupt configuration just as badly as backend
bugs.

## Forbidden Patterns

- hardcoded operator-facing strings
- business logic inside widget rendering
- direct filesystem/network access from render/event code
- key semantics that diverge from the menuconfig matrix
- row wrapping that breaks semantic prefixes or `--->`

## Required Patterns

- matrix-aligned row templates and popup semantics
- localized status/help/confirm text
- explicit dirty-state tracking based on current draft vs baseline
- viewport guard for sizes below `80x24`
- popup list virtualization/windowing when choice count exceeds popup viewport

## Testing Requirements

- keyboard contract tests for `Space`, `Enter`, and `Esc`
- viewport guard tests
- locale rendering tests for English and Chinese
- init flow tests for auto-detected languages and manual toggles
- pre-project strategy popup tests
- choice-list popup tests proving selected item remains visible when scrolling

## Code Review Checklist

- Does the screen follow the matrix grammar?
- Are toggles using `Space` only?
- Are strings catalog-driven?
- Is draft state separate from persisted state?
- Do popup flows match the documented close/confirm behavior?
