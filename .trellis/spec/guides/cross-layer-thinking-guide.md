# Cross-Layer Thinking Guide

> Think through `omv` data flow before implementing.

---

## The Problem

Most `omv` bugs will happen at boundaries:

- time source -> version engine
- version engine -> `.omv/state.toml`
- `.omv` truth -> language-native manifests
- locale preference -> CLI/TUI rendering
- init draft -> persisted config/targets

## Before Implementing Cross-Layer Features

### Step 1: Map the Flow

Use a concrete flow such as:

```text
Time Source -> Version Engine -> .omv State -> Target Sync -> Runtime Exports -> User Output
```

For each step, ask:

- what is the typed input?
- what is the persisted/output contract?
- what error should stop the flow?

### Step 2: Check the Critical Boundaries

| Boundary | Typical Risk |
| --- | --- |
| system/NTP/manual date -> validated date | false trust, future-date corruption |
| draft state -> persisted `.omv` | accidental partial saves |
| `.omv` -> manifest sync | manifest drift from truth source |
| locale preference -> rendered text | hardcoded copy or missing key parity |

### Step 3: Define Ownership Once

- version engine owns version math
- storage owns `.omv` file contracts
- sync adapters own manifest/runtime-export writes
- i18n catalog owns user-facing copy
- TUI owns interaction, not persistence truth

## Common Cross-Layer Mistakes

### Mistake 1: Two truth sources

**Bad**: reading `Cargo.toml` to decide the next version while also storing
state in `.omv/state.toml`

**Good**: derive next version only from `.omv` plus validated time

### Mistake 2: Locale split-brain

**Bad**: CLI reads locale from config, but TUI keeps a different internal
default

**Good**: both use the same normalized locale from `.omv/config.toml`

### Mistake 3: UI-driven persistence

**Bad**: writing files directly inside key handlers

**Good**: UI confirms a draft, backend persists atomically

## Checklist for OMV Features

Before implementation:

- [ ] Identified which `.omv` files are read and written
- [ ] Defined the exact time source and fallback path
- [ ] Defined how localized text is obtained
- [ ] Identified whether target sync is part of the command
- [ ] Checked whether manifest files are outputs rather than truth

After implementation:

- [ ] Tested Chinese and English output
- [ ] Tested bad/malformed `.omv` files
- [ ] Tested missing target manifest behavior
- [ ] Verified state remains consistent after failure
