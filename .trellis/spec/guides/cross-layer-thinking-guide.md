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
- `.omv` language/kind targets -> typed target planner
- `.omv` truth -> deterministic plan -> check/apply result
- `.omv` truth -> `.omv/ai/*` canonical contract
- `.omv/integrations.toml` -> provider detection -> capability status/apply
- `.omv/ai/*` -> installed host adapters/spec files
- finalize-boundary helper -> finalize-task event -> `.omv/finalizations.toml`
- locale preference -> CLI/TUI rendering
- command result -> structured JSON automation output
- init draft -> persisted config/targets

## Before Implementing Cross-Layer Features

### Step 1: Map the Flow

Use a concrete flow such as:

```text
Task Metadata -> Finalize Event -> .omv/finalizations -> Version Engine -> .omv State -> Target Sync -> Runtime Exports -> User Output
```

or:

```text
Init Draft -> .omv/integrations -> Provider Detection -> Integration Plan -> Targeted Safety Check -> Host Projection -> Capability Status -> User Output
```

or:

```text
Host Finish Boundary -> OMV finalize-boundary helper -> finalize-task -> .omv/finalizations -> bump/sync -> structured result
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
| `.omv/targets.toml` kind record -> adapter config | stringly dispatch, missing required fields, unsupported future capabilities |
| plan -> sync/check output | command-specific status drift or accidental mutation in check mode |
| `.omv/integrations.toml` -> provider/capability plan | stale detection, unsafe file mutation, status/failure drift |
| `.omv/ai/*` -> host adapters | stale guidance or unmanaged overwrite |
| host finish surface -> finalize-boundary helper | silent semantic inference, duplicate bump, wrong boundary source |
| typed result -> JSON envelope | automation breakage |
| locale preference -> rendered text | hardcoded copy or missing key parity |

### Step 3: Define Ownership Once

- version engine owns version math
- storage owns `.omv` file contracts
- sync adapters own manifest/runtime-export writes
- kind target adapters own kind-specific replacement semantics after storage has
  produced typed target records or unsupported future-kind placeholders
- contract registry owns supported capability IDs and generated contract mappings
- plan engine owns target status and proposed operations
- adapter projection owns `.omv/ai/*` and host-framework mirrors
- integration model owns provider/capability identity, desired selection,
  detection snapshots, capability status, and failure reasons
- finalize-boundary helper owns deterministic boundary fields, task resolution,
  idempotency fingerprint, and handoff to `finalize-task`
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
or `omv integrate apply` during the compatibility transition

### Mistake 1c: Mixing adapter projection and integration state

**Bad**: using `.omv/adapters.toml` as the selected provider/capability source
for init or integrate commands

**Good**: use `.omv/integrations.toml` for selected providers, selected
capabilities, detection snapshots, status, and failures; keep
`.omv/adapters.toml` as legacy projection recovery metadata

### Mistake 1d: Treating finalize-boundary as a semantic classifier

**Bad**: inferring `change_type` from changed files or commit messages

**Good**: require the host/agent or user to provide one explicit enum value and
return pending/manual-action when it is missing

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
- [ ] Identified whether `.omv/integrations.toml` is read or written
- [ ] Identified whether the flow touches provider detection, selected
      capabilities, capability status, or failure reasons
- [ ] Defined the exact time source and fallback path
- [ ] Defined how localized text is obtained
- [ ] Defined whether JSON output contracts are affected
- [ ] Identified whether target sync is part of the command
- [ ] Identified whether the target is language-based or kind-based
- [ ] Identified unsupported parser cases and future-kind capability handling
- [ ] Identified whether the command is plan-only, check-only, or write/apply
- [ ] Defined whether integration apply needs targeted worktree-safety checks
- [ ] Confirmed host files are derived projections, not source-of-truth inputs
- [ ] Checked whether manifest files are outputs rather than truth
- [ ] Defined whether the flow needs audit/idempotency state in `.omv/finalizations.toml`
- [ ] If using finalize-boundary, defined provider + boundary identity,
      required `change_type`, task resolution, and idempotency fingerprint

After implementation:

- [ ] Tested Chinese and English output
- [ ] Tested structured JSON output if automation paths changed
- [ ] Tested bad/malformed `.omv` files
- [ ] Tested missing and malformed `.omv/integrations.toml` if integration
      paths changed
- [ ] Tested partial integration apply failure and status retry behavior
- [ ] Tested duplicate finalize fingerprint behavior if automation can replay the event
- [ ] Tested finalize-boundary missing `change_type` if completion-boundary
      automation changed
- [ ] Tested adapter refresh/install if host projections changed
- [ ] Tested missing target manifest behavior
- [ ] Verified state remains consistent after failure
