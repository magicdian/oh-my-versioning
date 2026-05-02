# Directory Structure

> How Rust backend code is organized in `omv`.

---

## Overview

Keep pure versioning/time/storage logic separate from CLI parsing, TUI
rendering, language-target adapters, and AI/spec projection adapters. The goal
is to let `omv bump`, `omv sync`, `omv current`, `omv event finalize-task`,
`omv integrate ...`, and temporary compatibility `omv adapter ...` commands
share one backend core instead of re-implementing the same behavior in
different entry points.

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
│   ├── integration.rs       # provider/capability model and internal registry
│   ├── versioning/          # date/build-number rules and output strategies
│   ├── time/                # system time, NTP, manual confirmation logic
│   ├── locale/              # locale selection and normalization
│   └── target/              # shared target metadata and adapter contracts
├── storage/
│   ├── config.rs            # .omv/config.toml load/save
│   ├── state.rs             # .omv/state.toml load/save
│   ├── targets.rs           # .omv/targets.toml load/save
│   ├── adapters.rs          # .omv/adapters.toml load/save
│   ├── integrations.rs      # .omv/integrations.toml load/save
│   ├── finalizations.rs     # .omv/finalizations.toml load/save
│   └── atomic.rs            # write-temp + rename helpers
├── sync/
│   ├── mod.rs               # deterministic plan model, check mode, and sync coordinator
│   ├── rust.rs              # Cargo.toml + runtime export sync
│   ├── generic.rs           # kind-based text, regex, Markdown, YAML, and C-header target planners
│   ├── cargo_workspace.rs   # kind-based Cargo workspace member and lockfile planner
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
└── omv/contract/versions/
    ├── README.md
    ├── current/
    │   └── contract.proto   # editable latest protobuf contract source
    ├── 1/
    │   └── contract.proto   # frozen language-native target contract snapshot
    └── 2/
        └── contract.proto   # frozen current runtime contract snapshot

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
├── integrations.toml
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
3. integration plan/apply or legacy adapter projection
4. localized or structured output

They should not duplicate version-bump or time-validation logic inline.

For `omv event finalize-task` specifically:

1. CLI parses event fields
2. app validates request shape and loads persistence state
3. core finalization logic decides whether the change is bumpable
4. storage records pending/completed finalization audit entries
5. existing bump/sync orchestration performs version mutation

For `omv integrate apply` specifically:

1. CLI parses integration operation and output mode
2. app loads `.omv/integrations.toml` and internal provider descriptors
3. app re-detects supported providers before planning writes
4. planning computes provider/capability status plus affected host files
5. targeted worktree-safety checks run only over affected integration files
6. successful capability installs are persisted even if other selected
   capabilities fail
7. any selected capability failure returns a non-zero command result with
   stable reason codes and a localized message

### Rule: One adapter per language family

Any file-format or manifest mutation belongs in `src/sync/<language>.rs`, never
inside CLI parsing or TUI event handling.

Language adapters must compute a deterministic plan before writes are applied.
`omv plan`, `omv sync --check`, `omv sync`, and post-`omv bump` sync all share
the same plan model.

### Rule: Keep generated contract code behind `src/contract/`

`proto/omv/contract/versions/current/*.proto` is compiled by `build.rs` into
`OUT_DIR`. Numbered directories under `proto/omv/contract/versions/` are frozen
contract snapshots and must not be mutated after release. Generated Rust code is
included from `src/contract/mod.rs`, is not committed, and must not contain
handwritten business logic. Capability registration and domain mapping live in
handwritten Rust under `src/contract/`.

## Scenario: Stable/Frozen Protobuf Contract Snapshots

### 1. Scope / Trigger

- Trigger: adding, removing, renaming, or changing protobuf contract fields,
  enum values, generated contract paths, `CONTRACT_VERSION`, or build-time
  protobuf compilation.
- This is cross-layer because `build.rs` feeds generated Rust, generated Rust
  feeds `src/contract/registry.rs`, registry values feed plan/capability JSON,
  and frozen snapshots define compatibility boundaries for future migrations.

### 2. Signatures

```rust
// build.rs
prost_build::Config::new().compile_protos(
    &["proto/omv/contract/versions/current/contract.proto"],
    &["proto"],
)?;

// src/contract/mod.rs
include!(concat!(env!("OUT_DIR"), "/omv.contract.v1.rs"));

// src/contract/registry.rs
pub const CONTRACT_VERSION: u32 = <newest frozen version>;
pub const STRUCTURED_JSON_CONTRACT_VERSION: &str = "1";

impl CapabilityRegistry {
    pub fn generated_capability_set(&self) -> OmvCapabilitySet;
}
```

### 3. Contracts

- `proto/omv/contract/versions/current/contract.proto` is the editable latest
  contract source compiled by `build.rs`.
- `proto/omv/contract/versions/<n>/contract.proto` files are frozen snapshots.
- After bootstrap or release, `versions/current/contract.proto` must be
  byte-for-byte identical to the highest numbered frozen snapshot.
- `src/contract/registry.rs::CONTRACT_VERSION` must equal the highest numbered
  frozen snapshot compiled into the binary.
- Protobuf package naming may stay `omv.contract.v1` across snapshot numbers
  until the generated Rust namespace itself needs a breaking change.
- `STRUCTURED_JSON_CONTRACT_VERSION` and `adapter::CONTRACT_VERSION` are
  separate compatibility domains and must not be bumped just because the
  protobuf contract snapshot changes.
- Removed protobuf fields or enum values must be marked `reserved` in future
  snapshots. Do not reuse numeric tags or enum values.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| `current` differs from newest frozen snapshot after release/bootstrap | unit test fails |
| `CONTRACT_VERSION` differs from newest frozen snapshot number | unit test fails |
| additive proto change in development | update `current`, then freeze a new numbered snapshot before merge/release |
| removed field or enum value | reserve the numeric tag/value in the next snapshot |
| change only affects structured JSON envelope | update JSON contract/version docs, not protobuf snapshot by default |
| change only affects `.omv/*.toml` storage | update storage schema docs, not protobuf snapshot by default |

### 5. Good/Base/Bad Cases

Good:

```text
proto/omv/contract/versions/current/contract.proto
proto/omv/contract/versions/3/contract.proto
src/contract/registry.rs::CONTRACT_VERSION = 3
```

Base:

```text
current == versions/2
CONTRACT_VERSION == 2
STRUCTURED_JSON_CONTRACT_VERSION == "1"
adapter::CONTRACT_VERSION == 1
```

Bad:

```text
current has an added enum value, newest frozen snapshot is still versions/2,
and CONTRACT_VERSION remains 2.
```

### 6. Tests Required

- unit test: read `proto/omv/contract/versions/current/contract.proto` and the
  highest numeric `versions/<n>/contract.proto`; assert byte-for-byte equality.
- unit test: assert the highest numeric frozen directory equals
  `src/contract/registry.rs::CONTRACT_VERSION`.
- boundary test: assert frozen v1 excludes generalized target-kind and host
  integration capabilities.
- boundary test: assert frozen v2 includes generalized target-kind capabilities
  and host integration capability metadata.
- build test: `cargo test --all-targets --all-features` must compile generated
  Rust from `versions/current`.

Assertion points:

- generated `OmvCapabilitySet.contract_version`
- presence/absence of stable enum names in frozen snapshots
- `OmvCapabilitySet.integration_support = 6` remains in the v2/current
  snapshot until a future v3 supersedes it

### 7. Wrong vs Correct

#### Wrong

```text
Edit proto/omv/contract/versions/2/contract.proto directly for a new
capability and leave current plus CONTRACT_VERSION unchanged.
```

#### Correct

```text
Edit proto/omv/contract/versions/current/contract.proto, copy it to
proto/omv/contract/versions/3/contract.proto at the freeze point, then set
src/contract/registry.rs::CONTRACT_VERSION = 3 and update guard tests/docs.
```

### Rule: Keep AI/spec adapter projection separate from language sync

`src/adapter.rs` owns generation of `.omv/ai/*` and projection into host files
such as `AGENTS.md`, `CLAUDE.md`, or spec-framework guides. It must not own
version math or language-manifest edits.

Legacy `omv adapter ...` commands may continue to call this projection path
during the MVP transition. New provider/capability selection and status logic
belongs to the integration model and `.omv/integrations.toml`, not to
`.omv/adapters.toml`.

### Rule: Keep integration providers internal in MVP

MVP provider descriptors are internal registry data for supported hosts and
capabilities. Do not expose a third-party plugin SDK/runtime or load arbitrary
provider code in MVP. Public plugin runtime support remains future work.

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
- integration provider/capability identity lives in `src/core/integration.rs`
- language-specific sync never bypasses `src/sync/`
- capability IDs never bypass `src/contract/registry.rs`
- agent/spec host projection never bypasses `src/adapter.rs`
- integration state load/save never bypasses `src/storage/integrations.rs`
- generalized kind target parsing never bypasses `src/storage/targets.rs`
- kind target writes never bypass the shared `src/sync/mod.rs` plan/apply
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

### Don't: Treat legacy adapter commands as the expanding integration API

`omv adapter install/refresh/list/status` remains for compatibility. New
provider selection, capability status, plan/apply behavior, and
completion-boundary automation belong under `omv integrate ...` and the
integration model.
