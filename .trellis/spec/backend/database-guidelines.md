# Database Guidelines

> Persistent state contracts for `omv`.

---

## Overview

V1 does not use a database. This file defines the equivalent persistence rules
for canonical TOML files under `.omv/` plus the generated AI/spec contract
artifacts OMV projects from that state.

## Scenario: `.omv` Persistent State

### 1. Scope / Trigger

- Trigger: any change to config loading, version state mutation, target
  registration/sync, adapter projection, or host integration recovery
- Reason code-spec depth is required: these files and generated artifacts are
  the only source of truth or canonical projections for version behavior,
  locale, target synchronization, and adapter installation recovery

### 2. Signatures

The backend should converge on a shape equivalent to:

```rust
fn load_config(root: &Path) -> Result<OmvConfig, OmvError>;
fn save_config(root: &Path, config: &OmvConfig) -> Result<(), OmvError>;

fn load_state(root: &Path) -> Result<OmvState, OmvError>;
fn save_state(root: &Path, state: &OmvState) -> Result<(), OmvError>;

fn load_targets(root: &Path) -> Result<OmvTargets, OmvError>;
fn save_targets(root: &Path, targets: &OmvTargets) -> Result<(), OmvError>;

fn load_adapters(root: &Path) -> Result<OmvAdapters, OmvError>;
fn save_adapters(root: &Path, adapters: &OmvAdapters) -> Result<(), OmvError>;

fn resolve_omv_root(cwd: &Path) -> Result<PathBuf, OmvError>;
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), OmvError>;
fn ensure_canonical_artifacts(root: &Path) -> Result<(), OmvError>;
```

### 3. Contracts

#### `.omv/config.toml`

Required V1 fields:

```toml
schema_version = 1
locale = "en-US" # or "zh-CN"
timezone = "UTC+0"
project_profile = "personal" # or "oss"
version_output = "date-triplet"
build_policy = "daily-reset" # or "continuous"
ntp_enabled = true
```

Rules:

- `locale` must be normalized to `en-US` or `zh-CN`
- `timezone` must be stored in canonical form chosen by the implementation
- transient CLI flags such as `--no-ntp` (skip NTP for current run) must not
  be persisted

#### `.omv/state.toml`

Required V1 fields:

```toml
schema_version = 1
logical_date = "2026-04-13"
build_number = 1
last_issued_version = "2604.13.1"
last_time_source = "ntp" # or system/manual-confirmed
```

Rules:

- `logical_date` plus `build_number` are the mutable version truth
- `last_issued_version` is derived but stored for human/debug visibility
- if stored `logical_date` is greater than the validated "today", command flow
  must stop and request user confirmation

#### `.omv/targets.toml`

Required V1 shape:

```toml
schema_version = 1

[[targets]]
id = "workspace-rust"
language = "rust"
root = "."
manifest_path = "Cargo.toml"
runtime_export_path = "src/generated/version.rs"
strategy = "existing-manifest"
enabled = true
```

Rules:

- V1 uses a flat list of targets
- each target has exactly one `language`
- `strategy` records how the target should behave when the project did not
  exist yet during `omv init`
- native manifest files are synchronized outputs, not authoritative inputs

#### `.omv/adapters.toml`

Required V1 shape:

```toml
schema_version = 1

[[installations]]
kind = "agent"
name = "codex"
install_mode = "hybrid" # or link/materialize
source_contract_version = 1

[[installations.targets]]
path = "AGENTS.md"
source_path = ".omv/ai/adapters/codex/AGENTS.md"
mode = "managed-block" # or link/materialize
```

Rules:

- this file records what OMV projected into host frameworks
- it is registry metadata, not version truth
- `omv adapter refresh` must be able to recreate host projections from this file
- install-mode metadata must describe whether OMV linked, materialized, or
  block-inserted each target

#### `.omv/ai/*`

Required V1 canonical artifacts:

```text
.omv/ai/contract.json
.omv/ai/instructions.md
.omv/ai/adapters/...
```

Rules:

- `.omv/ai/contract.json` is the machine-readable contract for automation and
  adapter generation
- `.omv/ai/instructions.md` is the canonical human-readable OMV versioning
  guidance projected into host frameworks
- host files such as `AGENTS.md`, `CLAUDE.md`, or spec guides are derived
  projections of `.omv/ai/*`
- `.omv/ai/*` is generated and may be safely refreshed by OMV
- `.omv/ai/*` does not replace `.omv/state.toml` as version truth

### 4. Validation & Error Matrix

| Condition | Behavior | Error Class |
| --- | --- | --- |
| `.omv` root cannot be resolved | fail command before mutation | `ConfigError::RootResolution` |
| config locale unsupported | reject load and suggest valid locales | `ConfigError::InvalidLocale` |
| build policy unsupported | reject load | `ConfigError::InvalidBuildPolicy` |
| state file missing during `omv init` | create new default state | none |
| state file missing during `omv bump` after init | fail with recovery hint | `StateError::MissingState` |
| stored logical date > validated today | require manual confirmation flow | `TimeError::FutureStoredDate` |
| targets file malformed | fail before sync | `TargetError::InvalidTargetRecord` |
| adapters registry malformed | fail adapter status/refresh/install recovery path | `AdapterError::Parse` |
| host adapter target conflicts with unmanaged file | stop install rather than overwrite | `AdapterError::Conflict` |
| atomic write fails | leave original file intact | `StorageError::AtomicWriteFailed` |

### 5. Good/Base/Bad Cases

#### Good

- `.omv/config.toml`, `.omv/state.toml`, `.omv/targets.toml`, and
  `.omv/adapters.toml` all parse when present
- stored locale is `zh-CN`
- stored logical date equals validated today; `build_number` increments
- `.omv/ai/*` is refreshed before host adapters are projected

#### Base

- user runs `omv init` in a new directory
- `.omv/` is created with default config/state plus selected targets
- later `omv adapter install --agent codex` creates registry metadata and host
  projections

#### Bad

- `Cargo.toml` is edited manually but `.omv/state.toml` is not updated
- `AGENTS.md` is treated as the primary version policy document
- `omv` must preserve `.omv` as truth and re-sync manifests or host guidance
  from there

### 6. Tests Required

- Config load/save round-trip with `en-US` and `zh-CN`
- State bump tests for `daily-reset` and `continuous`
- Future stored date validation test
- Targets round-trip test with multiple flat entries
- Adapters registry round-trip test with installed targets
- Canonical `.omv/ai/*` generation test
- Atomic write test proving original file survives a simulated failure

Assertion points:

- serialized TOML uses normalized locale values
- invalid enum-like values fail with deterministic errors
- sync code reads targets from `.omv/targets.toml`, not by re-scanning manifests
- adapter refresh can rebuild host projections from `.omv/adapters.toml`

### 7. Wrong vs Correct

#### Wrong

```rust
// Treating Cargo.toml as the truth source
let version = read_cargo_version("Cargo.toml")?;
```

#### Correct

```rust
let state = load_state(omv_root)?;
let next = version_engine.next_version(&config, &state, &validated_today)?;
sync_all_targets(omv_root, &targets, &next)?;
```

## Common Mistakes

- Persisting transient flags into config
- Writing manifests first and `.omv` second
- Using non-atomic writes for `.omv` files
- Treating `.omv/adapters.toml` as version truth
- Adding per-language nesting to targets before V1 actually needs monorepo
  structure
