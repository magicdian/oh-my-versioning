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
    Config(ConfigError),
    State(StateError),
    Time(TimeError),
    Ntp(NtpError),
    Target(TargetError),
    I18n(I18nError),
    Io(std::io::Error),
}
```

Rules:

- each leaf error must preserve enough context to produce a localized user
  message and a structured log record
- render user-facing text at the boundary, not deep inside core logic
- map exit behavior from typed errors, not from string matching

## Scenario: Version Bump and Time Validation

### 1. Scope / Trigger

- Trigger: `omv bump`, `omv sync`, `omv init`, or any command that validates
  current date against stored version state

### 2. Signatures

```rust
fn run_bump(args: BumpArgs) -> Result<BumpResult, OmvError>;
fn validate_current_date(
    config: &OmvConfig,
    state: &OmvState,
    time_source: &dyn TimeSource,
) -> Result<ValidatedDate, OmvError>;
fn render_error(locale: &Catalog, err: &OmvError) -> String;
```

### 3. Contracts

- expected operator failures return `Err(OmvError::...)`
- CLI layer formats a localized message to stderr
- logs may contain more diagnostic detail than the operator-facing message
- future-date conflicts must stop mutation and request explicit operator input
- NTP failure does not justify mutating system time or silently trusting bad
  state

### 4. Validation & Error Matrix

| Condition | CLI Behavior | Recovery Hint |
| --- | --- | --- |
| unsupported locale in config | fail fast before command body | pick `en-US` or `zh-CN` |
| NTP lookup fails and command requires strict validation | fail or request explicit skip flow | rerun with skip flag if appropriate |
| stored date > validated current date | block and ask for manual confirmation | operator confirms correct date |
| target manifest missing for registered existing target | fail sync for that target | repair target or rerun init |
| i18n key missing in selected locale | fall back to `en-US`; if absent there too, return key text | fix catalog parity |

### 5. Good/Base/Bad Cases

#### Good

- `omv bump` validates time, computes next version, syncs targets, and prints a
  localized success message

#### Base

- NTP is enabled, lookup succeeds, stored date is equal to validated today

#### Bad

- command encounters malformed `.omv/state.toml` and continues by guessing
  defaults

### 6. Tests Required

- typed error -> localized message mapping
- future stored date returns blocking error path
- malformed config returns deterministic parse/validation error
- target sync failure leaves state/manifests consistent according to command
  transaction strategy

Assertion points:

- expected operator errors do not panic
- stderr text is locale-aware
- error variants carry machine-usable classification

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
