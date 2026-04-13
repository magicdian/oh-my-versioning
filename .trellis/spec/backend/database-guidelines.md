# Database Guidelines

> Persistent state contracts for `omv`.

---

## Overview

V1 does not use a database. This file defines the equivalent persistence rules
for the three canonical TOML files under `.omv/`.

## Scenario: `.omv` Persistent State

### 1. Scope / Trigger

- Trigger: any change to config loading, version state mutation, or target
  registration/sync
- Reason code-spec depth is required: these files are the only source of truth
  for version behavior, locale, and target synchronization

### 2. Signatures

The backend should converge on a shape equivalent to:

```rust
fn load_config(root: &Path) -> Result<OmvConfig, OmvError>;
fn save_config(root: &Path, config: &OmvConfig) -> Result<(), OmvError>;

fn load_state(root: &Path) -> Result<OmvState, OmvError>;
fn save_state(root: &Path, state: &OmvState) -> Result<(), OmvError>;

fn load_targets(root: &Path) -> Result<OmvTargets, OmvError>;
fn save_targets(root: &Path, targets: &OmvTargets) -> Result<(), OmvError>;

fn resolve_omv_root(cwd: &Path) -> Result<PathBuf, OmvError>;
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), OmvError>;
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
- transient CLI flags such as "skip NTP for this run" must not be persisted

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
| atomic write fails | leave original file intact | `StorageError::AtomicWriteFailed` |

### 5. Good/Base/Bad Cases

#### Good

- `.omv/config.toml`, `.omv/state.toml`, and `.omv/targets.toml` all exist and
  parse
- stored locale is `zh-CN`
- stored logical date equals validated today; `build_number` increments

#### Base

- user runs `omv init` in a new directory
- `.omv/` is created with default config/state plus selected targets

#### Bad

- `Cargo.toml` is edited manually but `.omv/state.toml` is not updated
- `omv` must preserve `.omv` as truth and re-sync manifests from there

### 6. Tests Required

- Config load/save round-trip with `en-US` and `zh-CN`
- State bump tests for `daily-reset` and `continuous`
- Future stored date validation test
- Targets round-trip test with multiple flat entries
- Atomic write test proving original file survives a simulated failure

Assertion points:

- serialized TOML uses normalized locale values
- invalid enum-like values fail with deterministic errors
- sync code reads targets from `.omv/targets.toml`, not by re-scanning manifests

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
- Adding per-language nesting to targets before V1 actually needs monorepo
  structure
