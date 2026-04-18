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
  `omv adapter ...`, or any command that validates current date against stored
  version state

### 2. Signatures

```rust
fn run_bump(args: BumpArgs) -> Result<BumpResult, OmvError>;
fn run_event(args: EventArgs) -> Result<EventResult, OmvError>;
fn validate_current_date(
    config: &OmvConfig,
    state: &OmvState,
    time_source: &dyn TimeSource,
) -> Result<ValidatedDate, OmvError>;
fn execute_finalize_task(args: FinalizeTaskArgs) -> Result<FinalizeTaskResult, OmvError>;
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
| adapter install targets existing unmanaged host file | fail without overwriting file | move file or choose another host path |
| `--json` requested and command fails | emit structured envelope to stderr | inspect `error.code` and `error.details` |
| i18n key missing in selected locale | fall back to `en-US`; if absent there too, return key text | fix catalog parity |

### 5. Good/Base/Bad Cases

#### Good

- `omv bump` validates time, computes next version, syncs targets, and prints a
  localized success message
- `omv adapter status --json` returns a stable envelope with installed adapter
  metadata
- `omv event finalize-task --json` returns a stable success envelope for
  `bumped`, `noop`, duplicate, and recovered paths

#### Base

- NTP is enabled, lookup succeeds, stored date is equal to validated today
- finalize-task is called once with a new semantic fingerprint and passing tests

#### Bad

- command encounters malformed `.omv/state.toml` and continues by guessing
  defaults
- adapter install overwrites an unmanaged `AGENTS.md` without surfacing a
  conflict
- finalize-task sees a duplicate fingerprint and bumps a second time anyway

### 6. Tests Required

- typed error -> localized message mapping
- typed error -> structured JSON error mapping
- future stored date returns blocking error path
- malformed config returns deterministic parse/validation error
- target sync failure leaves state/manifests consistent according to command
  transaction strategy
- finalize-task missing field returns typed validation error
- finalize-task invalid enum-like field returns typed validation error
- finalize-task duplicate fingerprint returns structured success instead of
  error

Assertion points:

- expected operator errors do not panic
- stderr text is locale-aware
- error variants carry machine-usable classification
- JSON failures preserve a stable top-level envelope
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
