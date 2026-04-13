# Directory Structure

> How Rust backend code is organized in `omv`.

---

## Overview

Keep pure versioning/time/storage logic separate from CLI parsing, TUI
rendering, and language-target adapters. The goal is to let `omv bump`,
`omv sync`, and `omv init` share one backend core instead of re-implementing
the same behavior in different entry points.

## Directory Layout

```text
src/
├── main.rs                  # CLI entrypoint
├── cli/                     # clap commands, flags, dispatch
├── app/                     # orchestration layer for commands/use-cases
├── core/
│   ├── versioning/          # date/build-number rules and output strategies
│   ├── time/                # system time, NTP, manual confirmation logic
│   ├── locale/              # locale selection and normalization
│   └── target/              # shared target metadata and adapter contracts
├── storage/
│   ├── config.rs            # .omv/config.toml load/save
│   ├── state.rs             # .omv/state.toml load/save
│   ├── targets.rs           # .omv/targets.toml load/save
│   └── atomic.rs            # write-temp + rename helpers
├── sync/
│   ├── mod.rs               # target sync coordinator
│   ├── rust.rs              # Cargo.toml + runtime export sync
│   ├── python.rs
│   ├── go.rs
│   ├── java.rs
│   └── c_family.rs          # C/C++ manifest/export rules
├── i18n.rs                  # catalog loading, fallback, formatting
├── ui/                      # ratatui/menuconfig runtime
│   ├── app.rs
│   ├── screen/
│   ├── widget/
│   └── state/
└── test_support/            # shared fixtures/builders for tests

resources/
└── i18n/
    ├── en-US.toml
    └── zh-CN.toml

tests/
├── cli/
├── integration/
└── snapshots/
```

## Module Organization

### Rule: Keep the core pure

Code in `src/core/` must not read files, render TUI widgets, or print directly
to stdout/stderr. It should operate on typed inputs and return typed results.

### Rule: `src/app/` orchestrates, it does not invent business rules

Command handlers should compose:

1. storage reads
2. core logic
3. adapter sync
4. localized user output

They should not duplicate version-bump or time-validation logic inline.

### Rule: One adapter per language family

Any file-format or manifest mutation belongs in `src/sync/<language>.rs`, never
inside CLI parsing or TUI event handling.

### Rule: Shared path resolution belongs in storage/app helpers

Repo-root detection, `.omv` root resolution, and atomic write helpers must live
in reusable modules. Do not re-derive them in every command.

## Naming Conventions

- Module/file names: `snake_case`
- Types/enums/traits: `PascalCase`
- Subcommand handlers: `verb_noun`, for example `init_project`,
  `bump_version`, `sync_targets`
- Adapter traits: `<Domain>Noun`, for example `TargetSyncAdapter`,
  `TimeSource`
- TOML schema types should mirror file names:
  `OmvConfig`, `OmvState`, `OmvTargetRecord`

## Examples

Use these boundaries as the baseline pattern:

- version calculation lives in `src/core/versioning/`
- locale catalog loading lives in `src/i18n.rs`
- `.omv` persistence lives in `src/storage/`
- language-specific sync never bypasses `src/sync/`

## Common Mistakes

### Don't: Put target-specific file edits in command handlers

This causes `omv bump` and `omv sync` to drift.

### Don't: Mix persisted state with TUI draft state

TUI draft edits belong in `src/ui/state/`; persisted `.omv` records belong in
`src/storage/`.

### Don't: Treat generated runtime export files as canonical data

They are outputs. `.omv` remains the truth.
