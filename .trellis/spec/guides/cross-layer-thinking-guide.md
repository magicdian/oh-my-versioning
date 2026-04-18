# Cross-Layer Thinking Guide

> Think through `omv` data flow before implementing.

---

## The Problem

Most `omv` bugs will happen at boundaries:

- time source -> version engine
- task/spec completion metadata -> finalize-task event
- finalize-task event -> `.omv/finalizations.toml`
- version engine -> `.omv/state.toml`
- `.omv` truth -> language-native manifests
- `.omv` truth -> `.omv/ai/*` canonical contract
- `.omv/ai/*` -> installed host adapters/spec files
- locale preference -> CLI/TUI rendering
- command result -> structured JSON automation output
- init draft -> persisted config/targets

## Before Implementing Cross-Layer Features

### Step 1: Map the Flow

Use a concrete flow such as:

```text
Task Metadata -> Finalize Event -> .omv/finalizations -> Version Engine -> .omv State -> Target Sync -> Runtime Exports -> User Output
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
| finalize fingerprint -> `.omv/finalizations.toml` | duplicate bump or unrecoverable pending state |
| `.omv` -> manifest sync | manifest drift from truth source |
| `.omv/ai/*` -> host adapters | stale guidance or unmanaged overwrite |
| typed result -> JSON envelope | automation breakage |
| locale preference -> rendered text | hardcoded copy or missing key parity |

### Step 3: Define Ownership Once

- version engine owns version math
- storage owns `.omv` file contracts
- sync adapters own manifest/runtime-export writes
- adapter projection owns `.omv/ai/*` and host-framework mirrors
- i18n catalog owns user-facing copy
- TUI owns interaction, not persistence truth

## Common Cross-Layer Mistakes

### Mistake 1: Two truth sources

**Bad**: reading `Cargo.toml` to decide the next version while also storing
state in `.omv/state.toml`

**Good**: derive next version only from `.omv` plus validated time

### Mistake 1b: Letting host guidance become stale authority

**Bad**: editing `AGENTS.md` or `openspec/project.md` manually and expecting OMV
to infer updated rules

**Good**: refresh host guidance from `.omv/ai/*` through `omv adapter refresh`

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
- [ ] Identified whether `.omv/ai/*` or adapter host files are affected
- [ ] Defined the exact time source and fallback path
- [ ] Defined how localized text is obtained
- [ ] Defined whether JSON output contracts are affected
- [ ] Identified whether target sync is part of the command
- [ ] Checked whether manifest files are outputs rather than truth
- [ ] Defined whether the flow needs audit/idempotency state in `.omv/finalizations.toml`

After implementation:

- [ ] Tested Chinese and English output
- [ ] Tested structured JSON output if automation paths changed
- [ ] Tested bad/malformed `.omv` files
- [ ] Tested duplicate finalize fingerprint behavior if automation can replay the event
- [ ] Tested adapter refresh/install if host projections changed
- [ ] Tested missing target manifest behavior
- [ ] Verified state remains consistent after failure
