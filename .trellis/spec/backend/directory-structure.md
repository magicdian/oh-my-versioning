# Directory Structure

> How Rust backend code is organized in `omv`.

---

## Overview

Keep pure versioning/time/storage logic separate from CLI parsing, TUI
rendering, language-target adapters, and AI/spec projection adapters. The goal
is to let `omv bump`, `omv sync`, `omv current`, `omv event finalize-task`, and
`omv adapter ...` share one backend core instead of re-implementing the same
behavior in different entry points.

## Directory Layout

```text
src/
├── main.rs                  # CLI entrypoint
├── cli/                     # clap commands, flags, dispatch
├── app/                     # orchestration layer for commands/use-cases
├── adapter.rs               # OMV AI/spec contract generation + adapter install flow
├── contract/                # generated protobuf boundary + handwritten capability registry
├── core/
│   ├── adapter.rs           # adapter enums and install-mode types
│   ├── finalization.rs      # finalize-task semantic decision rules
│   ├── versioning/          # date/build-number rules and output strategies
│   ├── time/                # system time, NTP, manual confirmation logic
│   ├── locale/              # locale selection and normalization
│   └── target/              # shared target metadata and adapter contracts
├── storage/
│   ├── config.rs            # .omv/config.toml load/save
│   ├── state.rs             # .omv/state.toml load/save
│   ├── targets.rs           # .omv/targets.toml load/save
│   ├── adapters.rs          # .omv/adapters.toml load/save
│   ├── finalizations.rs     # .omv/finalizations.toml load/save
│   └── atomic.rs            # write-temp + rename helpers
├── sync/
│   ├── mod.rs               # deterministic plan model, check mode, and sync coordinator
│   ├── rust.rs              # Cargo.toml + runtime export sync
│   ├── generic.rs           # V2 text, regex, Markdown, YAML, and C-header target planners
│   ├── cargo_workspace.rs   # V2 Cargo workspace member and lockfile planner
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

proto/
└── omv/contract/v1/
    └── contract.proto       # protobuf source for contract v1 generated Rust stubs

tests/
├── cli/
├── integration/
└── snapshots/
```

Generated project artifacts:

```text
.omv/
├── config.toml
├── state.toml
├── targets.toml
├── adapters.toml
└── ai/
    ├── contract.json
    ├── instructions.md
    └── adapters/
```

## Module Organization

### Rule: Keep the core pure

Code in `src/core/` must not read files, render TUI widgets, or print directly
to stdout/stderr. It should operate on typed inputs and return typed results.

### Rule: `src/app/` orchestrates, it does not invent business rules

Command handlers should compose:

1. storage reads
2. core logic
3. adapter sync or projection
4. localized or structured output

They should not duplicate version-bump or time-validation logic inline.

For `omv event finalize-task` specifically:

1. CLI parses event fields
2. app validates request shape and loads persistence state
3. core finalization logic decides whether the change is bumpable
4. storage records pending/completed finalization audit entries
5. existing bump/sync orchestration performs version mutation

### Rule: One adapter per language family

Any file-format or manifest mutation belongs in `src/sync/<language>.rs`, never
inside CLI parsing or TUI event handling.

Language adapters must compute a deterministic plan before writes are applied.
`omv plan`, `omv sync --check`, `omv sync`, and post-`omv bump` sync all share
the same plan model.

### Rule: Keep generated contract code behind `src/contract/`

`proto/omv/contract/v1/*.proto` is compiled by `build.rs` into `OUT_DIR`.
Generated Rust code is included from `src/contract/mod.rs`, is not committed,
and must not contain handwritten business logic. Capability registration and
domain mapping live in handwritten Rust under `src/contract/`.

### Rule: Keep AI/spec adapter projection separate from language sync

`src/adapter.rs` owns generation of `.omv/ai/*` and projection into host files
such as `AGENTS.md`, `CLAUDE.md`, or spec-framework guides. It must not own
version math or language-manifest edits.

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
  `OmvConfig`, `OmvState`, `OmvTargetRecord`, `OmvAdapters`,
  `OmvFinalizations`

## Examples

Use these boundaries as the baseline pattern:

- version calculation lives in `src/core/versioning/`
- locale catalog loading lives in `src/i18n.rs`
- `.omv` persistence lives in `src/storage/`
- finalize-task semantic decision lives in `src/core/finalization.rs`
- language-specific sync never bypasses `src/sync/`
- capability IDs never bypass `src/contract/registry.rs`
- agent/spec host projection never bypasses `src/adapter.rs`
- generalized V2 target parsing never bypasses `src/storage/targets.rs`
- V2 target writes never bypass the shared `src/sync/mod.rs` plan/apply
  boundary

## Common Mistakes

### Don't: Put target-specific file edits in command handlers

This causes `omv bump` and `omv sync` to drift.

### Don't: Mix persisted state with TUI draft state

TUI draft edits belong in `src/ui/state/`; persisted `.omv` records belong in
`src/storage/`.

### Don't: Treat generated runtime export files as canonical data

They are outputs. `.omv` remains the truth.

### Don't: Mix host-framework projection with canonical OMV instructions

Detailed rules belong under `.omv/ai/*`; host files should stay thin and
replaceable.
