# Error Handling

> Error handling contracts for `omv`.

---

## Overview

`omv` is a CLI. Failures must be understandable to operators and actionable for
developers. Expected problems should return typed errors and localized output;
they must not panic.

## Error Types

Use a top-level error enum equivalent to:

```rust
enum OmvError {
    Cli(CliError),
    Adapter(AdapterError),
    Config(ConfigError),
    Finalization(FinalizationError),
    Integration(IntegrationError),
    State(StateError),
    Time(TimeError),
    Ntp(NtpError),
    Target(TargetError),
    I18n(I18nError),
    Storage(StorageError),
    Io(std::io::Error),
}
```

Rules:

- each leaf error must preserve enough context to produce a localized user
  message and a structured log record
- render user-facing text at the boundary, not deep inside core logic
- map exit behavior from typed errors, not from string matching
- machine-readable output modes must serialize typed errors into a stable JSON
  envelope

## Scenario: Version Bump, Adapter Projection, and Structured Output

### 1. Scope / Trigger

- Trigger: `omv bump`, `omv sync`, `omv current`, `omv event finalize-task`,
  `omv event finalize-boundary`, `omv integrate ...`, temporary compatibility
  `omv adapter ...`, or any command that validates current date against stored
  version state

### 2. Signatures

```rust
fn run_bump(args: BumpArgs) -> Result<BumpResult, OmvError>;
fn run_event(args: EventArgs) -> Result<EventResult, OmvError>;
fn run_integrate(args: IntegrateArgs) -> Result<IntegrateResult, OmvError>;
fn validate_current_date(
    config: &OmvConfig,
    state: &OmvState,
    time_source: &dyn TimeSource,
) -> Result<ValidatedDate, OmvError>;
fn execute_finalize_task(args: FinalizeTaskArgs) -> Result<FinalizeTaskResult, OmvError>;
fn execute_finalize_boundary(args: FinalizeBoundaryArgs) -> Result<FinalizeBoundaryResult, OmvError>;
fn render_error(locale: &Catalog, err: &OmvError) -> String;
fn render_structured_error(command: &str, err: &OmvError) -> String;
fn apply_runtime_ntp_override(config: &mut OmvConfig, no_ntp: bool);
```

### 3. Contracts

- expected operator failures return `Err(OmvError::...)`
- CLI layer formats a localized message to stderr
- JSON mode writes a shared structured error envelope to stderr
- logs may contain more diagnostic detail than the operator-facing message
- future-date conflicts must stop mutation and request explicit operator input
- NTP failure does not justify mutating system time or silently trusting bad
  state
- `--no-ntp` is a runtime-only override and must never persist into
  `.omv/config.toml`
- `omv event finalize-task` must fail fast on missing/invalid required fields
  before writing pending audit state
- duplicate finalize fingerprints are a success/no-op path, not an operator
  error
- pending finalize recovery may sync current state, but must not bump twice
- `omv event finalize-boundary` must require explicit `change_type`; if it is
  missing, it returns a structured pending/manual-action result and must not
  call `finalize-task`
- finalize-boundary helpers should flatten structured provider + boundary name
  into the legacy finalize-task `source` string internally
- `omv sync --check` must return a typed target error on required drift,
  missing targets, unsupported targets, or planning errors; JSON error details
  should include the deterministic plan so automation can inspect failures
- `omv integrate status` must treat missing `.omv/integrations.toml` as empty
  state and report the provider/capability matrix
- `omv integrate apply` must re-detect before mutation and must fail before
  writes when `.omv/integrations.toml` is malformed
- integration apply is best-effort per capability but not success-masked:
  successful capability writes are persisted, failed capabilities include
  stable reason codes, and the command returns non-zero when any selected
  capability fails
- legacy `omv adapter install/refresh/list/status` remains available during
  MVP. Where behavior overlaps with integration apply/status, it should share
  projection and status helpers rather than creating divergent semantics.

Structured error contract:

```json
{
  "ok": false,
  "contract_version": "1",
  "command": "runtime",
  "data": null,
  "error": {
    "code": "missing_state",
    "message": "missing state file: ...",
    "details": {
      "path": ".omv/state.toml"
    }
  }
}
```

Rules:

- `error.code` must be stable and machine-usable
- `error.message` may be human-readable and diagnostic
- `error.details` should expose structured context such as `path`, `reason`,
  `agent`, `spec`, or validated dates

### 4. Validation & Error Matrix

| Condition | CLI Behavior | Recovery Hint |
| --- | --- | --- |
| unsupported locale in config | fail fast before command body | pick `en-US` or `zh-CN` |
| NTP lookup fails and command requires strict validation | fail or request explicit skip flow | rerun with skip flag if appropriate |
| `--no-ntp` passed for `omv bump` | use system-time source for this run only | rerun without flag to restore default NTP validation |
| stored date > validated current date | block and ask for manual confirmation | operator confirms correct date |
| finalize-task missing `--task-id`, `--change-type`, `--status`, `--tests`, `--fingerprint`, or `--source` | fail before mutation | caller must supply full completion metadata |
| finalize-task uses unsupported change/status/tests value | fail before mutation | caller fixes enum-like field values |
| duplicate finalize fingerprint already completed | return success result without second bump | caller may treat as idempotent completion |
| pending finalize fingerprint already moved version truth | recover by syncing current state and mark recovered success | rerun finalize safely |
| target manifest missing for registered existing target | fail sync for that target | repair target or rerun init |
| unknown target `kind` | plan as unsupported without executing writes | update OMV to a binary that supports that capability |
| malformed supported-kind target record | fail before planning writes | fix required fields or supported enum values |
| regex target has zero or ambiguous matches | fail planning for that target | refine pattern or set `allow_multiple = true` intentionally |
| Markdown managed block markers are missing, duplicated, or inverted | fail planning for that target | repair markers |
| YAML scalar uses unsupported YAML feature | fail planning for that target | use a simple mapping scalar path or a future fuller YAML adapter |
| Cargo workspace lockfile drifts with `lockfile = "check"` | fail `sync --check`; `sync` does not run broad cargo updates | choose `lockfile = "update"` for narrow package-line updates |
| `omv sync --check` detects required drift | fail without writing files | inspect `omv plan` or run `omv sync` intentionally |
| `.omv/integrations.toml` missing during `integrate status` | report empty selected state plus supported provider descriptors | run init or select integrations |
| `.omv/integrations.toml` malformed | fail before host detection or writes | repair TOML |
| selected provider is outside MVP support matrix | mark unsupported/failed with stable reason | choose Codex or Trellis in MVP |
| Trellis selected but not detected | fail/mark selected Trellis capabilities failed before mutation | install Trellis then rerun apply |
| Codex selected but not detected | apply may bootstrap lightweight instruction files | review generated changes |
| targeted integration file has unsafe existing changes | skip that capability and persist failure reason | review/stash changes then rerun apply |
| integration apply has partial capability failure | preserve successes, record failures, return non-zero | inspect status and retry |
| finalize-boundary missing `change_type` | return pending/manual-action JSON and do not call finalize-task | ask user for one enum value |
| adapter install targets existing unmanaged host file | fail without overwriting file | move file or choose another host path |
| `--json` requested and command fails | emit structured envelope to stderr | inspect `error.code` and `error.details` |
| i18n key missing in selected locale | fall back to `en-US`; if absent there too, return key text | fix catalog parity |

### 5. Good/Base/Bad Cases

#### Good

- `omv bump` validates time, computes next version, syncs targets, and prints a
  localized success message
- `omv adapter status --json` returns a stable envelope with installed adapter
  metadata
- `omv integrate status --json` returns a stable provider + capability matrix
  from `.omv/integrations.toml` plus current detection
- `omv integrate apply --json` returns per-capability success/failure and uses
  non-zero exit behavior for any failed selected capability
- `omv event finalize-task --json` returns a stable success envelope for
  `bumped`, `noop`, duplicate, and recovered paths
- `omv event finalize-boundary --json` returns pending/manual-action when
  semantic `change_type` is not supplied

#### Base

- NTP is enabled, lookup succeeds, stored date is equal to validated today
- finalize-task is called once with a new semantic fingerprint and passing tests

#### Bad

- command encounters malformed `.omv/state.toml` and continues by guessing
  defaults
- adapter install overwrites an unmanaged `AGENTS.md` without surfacing a
  conflict
- integrate apply reports success while one selected capability failed
- finalize-boundary silently guesses `change_type`
- finalize-task sees a duplicate fingerprint and bumps a second time anyway

### 6. Tests Required

- typed error -> localized message mapping
- typed error -> structured JSON error mapping
- future stored date returns blocking error path
- malformed config returns deterministic parse/validation error
- target sync failure leaves state/manifests consistent according to command
  transaction strategy
- sync check drift failure returns a typed structured error and leaves target
  files unchanged
- malformed supported-kind target records return `invalid_target_record`
- unknown target kinds appear as unsupported plan entries with an update-OMV
  diagnostic
- kind adapter planning failures appear as target plan errors and do not write
  files in `omv plan` or `omv sync --check`
- finalize-task missing field returns typed validation error
- finalize-task invalid enum-like field returns typed validation error
- finalize-task duplicate fingerprint returns structured success instead of
  error
- integrate status covers missing integrations state
- integrate apply covers safe success, unsafe target failure, unsupported
  provider/capability, and partial failure
- legacy adapter command compatibility tests prove existing adapter commands
  remain accepted during the transition
- finalize-boundary missing change type returns pending/manual-action without
  calling finalize-task

Assertion points:

- expected operator errors do not panic
- stderr text is locale-aware
- error variants carry machine-usable classification
- JSON failures preserve a stable top-level envelope
- integration failure details include provider, capability, target path where
  applicable, and stable reason code
- finalize-task idempotency does not rely on string matching or best-effort
  caller behavior

### 7. Wrong vs Correct

#### Wrong

```rust
panic!("invalid locale: {}", locale);
```

#### Correct

```rust
return Err(OmvError::Config(ConfigError::InvalidLocale(locale.into())));
```

## Common Mistakes

### Don't: Format human messages in core logic

That mixes locale rendering with business rules.

### Don't: Swallow validation failures and "best effort" write files

`omv` is the version truth source. Silent partial success creates corruption.

### Don't: Use `unwrap()` on persisted data or catalog loads

Persistence and localization are both operator inputs.
