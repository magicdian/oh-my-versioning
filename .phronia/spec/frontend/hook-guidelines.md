# Hook Guidelines

> Reusable interaction-helper rules for the Rust TUI.

---

## Overview

`omv` does not use React hooks. This file exists to document the equivalent
abstractions for reusable stateful UI behavior in Rust.

## Custom Hook Patterns

Preferred equivalents:

- typed event reducers
- focus controllers
- popup state machines
- helper functions that derive row models from draft state

Good pattern:

```rust
fn handle_key(draft: &mut InitDraft, focus: &mut FocusState, key: KeyEvent) -> UiAction;
```

## Data Fetching

The init TUI may trigger backend discovery or validation work, but the UI layer
must not perform raw filesystem/network calls directly. Route them through
command/application services and surface progress via waiting/result popups.

## Naming Conventions

- use `handle_*`, `derive_*`, `reduce_*`, `apply_*`
- avoid `use_*` names because they imply React semantics the project does not
  have

## Common Mistakes

### Don't: Port React mental models literally

Avoid building pseudo-hooks that obscure ordinary Rust ownership and state flow.

### Don't: Keep hidden mutable global UI state

All draft/focus/popup state must be explicit and testable.
