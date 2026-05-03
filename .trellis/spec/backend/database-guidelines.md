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

fn load_integrations(root: &Path) -> Result<OmvIntegrations, OmvError>;
fn load_integrations_if_exists(root: &Path) -> Result<OmvIntegrations, OmvError>;
fn save_integrations(root: &Path, integrations: &OmvIntegrations) -> Result<(), OmvError>;

fn load_finalizations(root: &Path) -> Result<OmvFinalizations, OmvError>;
fn load_finalizations_if_exists(root: &Path) -> Result<OmvFinalizations, OmvError>;
fn save_finalizations(root: &Path, finalizations: &OmvFinalizations) -> Result<(), OmvError>;

fn resolve_omv_root(cwd: &Path) -> Result<PathBuf, OmvError>;
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), OmvError>;
fn ensure_canonical_artifacts(root: &Path) -> Result<(), OmvError>;
```

### 3. Contracts

#### `.omv/config.toml`

User-authored V1 fields:

```toml
locale = "en-US" # or "zh-CN"
timezone = "UTC+0"
project_profile = "personal" # or "oss"
version_output = "date-triplet"
build_policy = "daily-reset" # or "continuous"
ntp_enabled = true
```

Rules:

- `schema_version` is internal compatibility metadata. User-authored fixtures
  and examples should omit it unless testing schema migration behavior.
- `locale` must be normalized to `en-US` or `zh-CN`
- `timezone` must be stored in canonical form chosen by the implementation
- transient CLI flags such as `--no-ntp` (skip NTP for current run) must not
  be persisted

#### `.omv/state.toml`

User-authored V1 fields:

```toml
logical_date = "2026-04-13"
build_number = 1
last_issued_version = "2604.13.1"
last_time_source = "ntp" # or system/manual-confirmed
```

Rules:

- `schema_version` is internal compatibility metadata. User-authored fixtures
  and examples should omit it unless testing schema migration behavior.
- `logical_date` plus `build_number` are the mutable version truth
- `last_issued_version` is derived but stored for human/debug visibility
- if stored `logical_date` is greater than the validated "today", command flow
  must stop and request user confirmation

#### `.omv/targets.toml`

User-authored V1 shape:

```toml
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

- `schema_version` defaults to the current V1 compatibility behavior when it is
  absent. It is not a feature flag and should be omitted from user-authored
  fixtures unless the test explicitly covers schema handling.
- V1 uses a flat list of targets
- each target has exactly one `language`
- `strategy` records how the target should behave when the project did not
  exist yet during `omv init`
- native manifest files are synchronized outputs, not authoritative inputs
- language-based records remain compatible and may coexist with kind-based
  records; `schema_version` is internal compatibility metadata, not a
  user-facing selector

User-authored kind-based shape:

```toml
[[targets]]
id = "root-version-file"
kind = "text-scalar"
adapter = "text"
path = "VERSION"
selector = "whole-file"
template = "{version}\n"
mode = "write"
```

Rules:

- generalized target records use `kind`, not `language`.
- Supported kinds are `text-scalar`, `regex-replace`, `markdown-managed-block`,
  `yaml-scalar`, `c-header-macro`, and `cargo-workspace`.
- unknown `kind` values must load as unsupported targets, appear in plans with
  an update-OMV diagnostic, and never execute writes.
- malformed supported-kind records must fail load with
  `TargetError::InvalidTargetRecord` before sync writes begin.
- `yaml-scalar` currently supports simple mapping scalar paths only; sequences,
  anchors, aliases, and block scalars are rejected.
- `cargo-workspace` supports exact workspace members and one-level `prefix/*`
  member globs. `Cargo.lock` updates are narrow: only matching workspace
  package version lines are updated; OMV does not run `cargo update`.

### Scenario: Kind-Based Target Capability Negotiation

#### 1. Scope / Trigger

- Trigger: adding or changing `.omv/targets.toml` kind-based records,
  introducing a new target kind, or changing target planning/status output.
- This is cross-layer because storage parsing feeds `omv plan`,
  `omv sync --check`, `omv sync`, structured JSON, and contract status mapping.

#### 2. Signatures

```rust
pub fn load_targets(root: &Path) -> Result<OmvTargets, OmvError>;
pub fn save_targets(root: &Path, targets: &OmvTargets) -> Result<(), OmvError>;
pub fn plan_all_targets(project_root: &Path, targets: &OmvTargets, version: &str) -> PlanSummary;

pub struct OmvTargets {
    pub schema_version: u32,
    pub targets: Vec<OmvTargetRecord>,
    pub v2_targets: Vec<OmvV2TargetRecord>,
    pub unsupported_targets: Vec<OmvUnsupportedTargetRecord>,
}

pub struct OmvUnsupportedTargetRecord {
    pub id: String,
    pub kind: String,
    pub adapter: String,
    pub root: String,
    pub enabled: bool,
    pub paths: Vec<String>,
}
```

#### 3. Contracts

- `schema_version` is internal compatibility metadata. Operators do not select
  feature sets by changing it.
- A target table with `kind` and a known `TargetKind` must parse into
  `OmvV2TargetRecord` and must validate required fields for that kind.
- A target table with an unknown `kind` must parse into
  `OmvUnsupportedTargetRecord`, not fail the whole targets file.
- Enabled unsupported targets must produce a required `PlanStatus::Unsupported`
  result with no operations and an update-OMV diagnostic.
- Disabled unsupported targets must produce `PlanStatus::Skipped` and must not
  block check/apply.
- `omv sync --check` fails for required unsupported targets and includes the
  plan in structured error details.
- `omv sync` must not apply any operation while required unsupported or errored
  targets exist.

#### 4. Validation & Error Matrix

| Input | Storage result | Plan/check/apply behavior |
|-------|----------------|---------------------------|
| `schema_version = 1` with `kind = "text-scalar"` | parse known kind | plan/check/apply normally |
| no `schema_version` with known `kind` | default `schema_version = 1`, parse known kind | plan/check/apply normally |
| `schema_version > current` with known `kind` | parse known kind | plan/check/apply normally |
| unknown enabled `kind` | parse `unsupported_targets` | plan unsupported, check/apply fail without writes |
| unknown disabled `kind` | parse `unsupported_targets` | plan skipped, check/apply do not block |
| known `kind` missing required field | `invalid_target_record` | fail before writes |
| `schema_version = 0` | `invalid_target_record` | fail before planning |

#### 5. Good/Base/Bad Cases

Good:

```toml
[[targets]]
id = "root-version-file"
kind = "text-scalar"
path = "VERSION"
template = "{version}\n"
```

Base compatibility:

```toml
[[targets]]
id = "workspace-rust"
language = "rust"
manifest_path = "Cargo.toml"
runtime_export_path = "src/generated/version.rs"
```

Bad but recoverable by updating OMV:

```toml
[[targets]]
id = "future-workspace"
kind = "future-workspace"
path = "future.toml"
```

Bad and must fail fast:

```toml
[[targets]]
id = "bad-yaml"
kind = "yaml-scalar"
path = "component.yml"
```

#### 6. Tests Required

- storage test: known `kind` loads when `schema_version` is absent or `1`.
- storage test: unknown `kind` loads into `unsupported_targets`.
- plan test: unknown enabled `kind` returns `PlanStatus::Unsupported`, no
  operations, and an update-OMV diagnostic.
- integration test: mixed known and unknown kind records let `omv plan` report
  both targets, make `omv sync --check` fail, make `omv sync` fail, and leave
  target files unwritten.
- regression test: malformed known-kind records still return
  `invalid_target_record` before writes.

#### 7. Wrong vs Correct

Wrong: use schema version as the user-visible capability switch and reject the
whole file before inspecting target records.

```rust
if schema_version > 2 {
    return Err(TargetError::InvalidTargetRecord(...).into());
}
```

Correct: treat schema version as metadata, parse known kinds, and preserve
future kinds as unsupported plan entries.

```rust
if TargetKind::parse(&kind_value).is_some() {
    targets.v2_targets.push(parse_v2_record(record)?);
} else {
    targets.unsupported_targets.push(parse_unsupported_record(record, kind_value)?);
}
```

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
- this is the legacy compatibility registry during the integration transition;
  new provider/capability desired state belongs in `.omv/integrations.toml`

#### `.omv/integrations.toml`

Required MVP shape:

```toml
schema_version = 1

[[providers]]
id = "codex"
provider_type = "agent"
detected = true
selected = true
last_detected_at = "1713446400"

[[providers.capabilities]]
id = "project-instructions"
selected = true
status = "installed" # selected, pending, installed, failed
target_files = ["AGENTS.md"]

[[providers.capabilities]]
id = "host-skill"
selected = true
status = "installed"
target_files = [".codex/skills/omv-versioning/SKILL.md"]

[[providers]]
id = "trellis"
provider_type = "spec"
detected = true
selected = true
last_detected_at = "1713446400"

[[providers.capabilities]]
id = "finalize-boundary"
selected = true
status = "pending"
target_files = [".agents/skills/finish-work/SKILL.md"]

[providers.capabilities.failure]
reason_code = "target_file_dirty"
message = "integration target has existing unreviewed changes"
```

Rules:

- this file is the canonical host integration desired-state and recovery file
- records persist selected providers/capabilities plus the last provider-level
  detection snapshot
- capability identities are medium-grained: `project-instructions`,
  `host-skill`, `spec-guide`, `spec-index-snippet`, and `finalize-boundary`
- MVP supported providers are `codex` and `trellis`; `claude` and `openspec`
  are future providers and should stay hidden from init UI
- `codex` may bootstrap lightweight instruction host files
- `trellis` requires an existing Trellis installation before OMV mutates
  Trellis files
- `omv integrate apply` must re-detect before planning/writing
- failed capabilities keep stable `reason_code` plus a human-readable message
- best-effort apply preserves successful capability writes but returns non-zero
  if any selected capability failed
- host files are derived projections from `.omv/ai/*` and are never authority
- `.omv/integrations.toml` does not replace `.omv/state.toml`, `.omv/targets.toml`,
  or `.omv/adapters.toml`; each file owns a distinct domain
- MVP providers are internal registry descriptors, not a public plugin runtime

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
- canonical guidance mentions `omv plan --json` and
  `omv sync --check --json` as non-mutating ways to preview target writes and
  detect drift
- host files such as `AGENTS.md`, `CLAUDE.md`, or spec guides are derived
  projections of `.omv/ai/*`
- host adapters with their own file-level syntax contracts must remain valid
  for that host after OMV metadata is added. For Codex skills,
  `adapters/codex/SKILL.md` and `.codex/skills/*/SKILL.md` must start with
  YAML frontmatter delimited by `---`; OMV managed-file comments belong after
  the frontmatter block, not before it.
- generated guidance should mention `omv integrate status/apply` and the
  finalize-boundary helper contract where available
- generated guidance must not make installed host files authoritative
- `.omv/ai/*` is generated and may be safely refreshed by OMV
- `.omv/ai/*` does not replace `.omv/state.toml` as version truth

#### `.omv/finalizations.toml`

Required V1 shape for finalize-task audit and dedupe:

```toml
schema_version = 1

[[entries]]
task_id = "04-18-product-gaps-automation-hooks"
fingerprint = "task-1:v1"
change_type = "bugfix"
task_status = "done"
tests_status = "passed"
source = "trellis-finish-work"
outcome = "bumped" # or pending/noop
reason = "semantic-change"
version_before = "2604.13.1"
version_after = "2604.13.2"
recorded_at = "1713446400"
```

Rules:

- this file is audit and idempotency state for `omv event finalize-task`
- fingerprints must be unique per logical completion event
- `pending` entries mean a semantic finalize started but did not finish cleanly
- if a pending fingerprint exists and version truth already moved, the next
  finalize attempt must recover by syncing current state instead of bumping
  again
- `noop` entries still matter; they prevent repeated finalize work from being
  re-evaluated differently later in the same completion event
- `noop` entries do not synchronize target files. Completion-boundary workflows
  that need to apply existing `.omv` truth to Cargo, README, ESP, or other
  targets must run `omv sync --json` explicitly and then verify with
  `omv sync --check --json`.
- this file is not version truth; `.omv/state.toml` remains version truth

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
| integrations file missing during init/status/apply | treat as empty integration state | none |
| integrations file malformed | fail before detection or host-file writes | `IntegrationError::Parse` |
| selected integration provider unsupported in MVP | fail or mark failed with stable reason before mutation | `IntegrationError::UnsupportedProvider` |
| selected capability unsupported by detected provider | fail/mark capability failed before mutation | `IntegrationError::UnsupportedCapability` |
| integration target file has unsafe existing changes | save desired state, skip write, and report retry/apply guidance | `IntegrationError::UnsafeTarget` |
| integration apply partially succeeds | persist successful capabilities, record failures, return non-zero | `IntegrationError::PartialFailure` |
| finalizations file missing during finalize | treat as empty audit state | none |
| finalizations file malformed | fail finalize before new decision is recorded | `FinalizationError::Parse` |
| finalize-task required field missing | fail before mutation | `FinalizationError::MissingField` |
| finalize-task enum-like field invalid | fail before mutation | `FinalizationError::InvalidField` |
| `omv sync --check` finds required target drift, missing target output, unsupported capability, or planning error | fail without mutation and include plan details in JSON error details | `TargetError::CheckFailed` |
| host adapter target conflicts with unmanaged file | stop install rather than overwrite | `AdapterError::Conflict` |
| atomic write fails | leave original file intact | `StorageError::AtomicWriteFailed` |

### 5. Good/Base/Bad Cases

#### Good

- `.omv/config.toml`, `.omv/state.toml`, `.omv/targets.toml`, and
  `.omv/adapters.toml` all parse when present
- `.omv/integrations.toml` parses when present and defaults to empty when absent
- stored locale is `zh-CN`
- stored logical date equals validated today; `build_number` increments
- `.omv/ai/*` is refreshed before host adapters are projected
- finalize-task writes one completed audit entry per fingerprint

#### Base

- user runs `omv init` in a new directory
- `.omv/` is created with default config/state plus selected targets
- later `omv integrate status` reports selected/detected provider capability
  state from `.omv/integrations.toml`
- legacy `omv adapter install --agent codex` remains available for MVP
  compatibility and creates registry metadata plus host projections
- later `omv event finalize-task` records either `noop` or `bumped` with a
  fingerprint-backed audit trail

#### Bad

- `Cargo.toml` is edited manually but `.omv/state.toml` is not updated
- `AGENTS.md` is treated as the primary version policy document
- `.omv/integrations.toml` is ignored and adapter status is used as the only
  host integration source
- a repeated finalize call with the same fingerprint bumps twice
- `omv` must preserve `.omv` as truth and re-sync manifests or host guidance
  from there

### 6. Tests Required

- Config load/save round-trip with `en-US` and `zh-CN`
- State bump tests for `daily-reset` and `continuous`
- Future stored date validation test
- Targets round-trip test with multiple flat entries
- Adapters registry round-trip test with installed targets
- Integrations round-trip test with selected providers, capability statuses,
  detection snapshot, and failure reason
- Integrations missing-file test proving absent state defaults to empty
- Integrations malformed-file test proving apply/status fail before writes
- Finalizations round-trip test with pending/bumped/noop entries
- Canonical `.omv/ai/*` generation test
- Codex skill projection test asserting both canonical
  `.omv/ai/adapters/codex/SKILL.md` and installed
  `.codex/skills/omv-versioning/SKILL.md` start with YAML frontmatter before
  OMV managed-file metadata
- `omv plan --json` test proving target status output is produced without writes
- `omv sync --check` success and drift-failure tests proving check mode does
  not mutate targets
- kind target load/save tests for each generalized target kind or a mixed
  representative
- mixed language/kind plan, check, sync, and bump tests proving all target
  kinds use the shared plan engine
- Atomic write test proving original file survives a simulated failure
- Finalize-task duplicate fingerprint test proving the second call does not bump
- Finalize-task noop test proving non-semantic changes are audited without
  mutation
- Finish-boundary host projection test proving generated guidance includes
  `omv sync --check --json`, an explicit `omv sync --json` repair path, and a
  warning that non-semantic finalize no-ops do not write target files

Assertion points:

- serialized TOML uses normalized locale values
- invalid enum-like values fail with deterministic errors
- sync code reads targets from `.omv/targets.toml`, not by re-scanning manifests
- adapter refresh can rebuild host projections from `.omv/adapters.toml`
- integrate apply re-detects providers and persists capability-level successes
  and failures
- finalize-task recovery path syncs current state when a pending fingerprint
  already moved version truth

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
- Treating `.omv/adapters.toml` as integration desired state after the MVP
  transition adds `.omv/integrations.toml`
- Treating installed host files as integration authority
- Treating `.omv/finalizations.toml` as version truth instead of audit/dedupe
- Adding per-language nesting to targets before V1 actually needs monorepo
  structure
