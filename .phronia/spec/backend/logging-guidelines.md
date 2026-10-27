# Logging Guidelines

> Structured logging rules for `omv`.

---

## Overview

Use logs for diagnostics and auditability. Use localized stdout/stderr for
operator communication. These are different channels and must not be mixed.

## Log Levels

| Level | Use For | Example |
| --- | --- | --- |
| `debug` | step-by-step diagnostics, parsed values, chosen branches | resolved repo root, selected target count |
| `info` | significant command milestones | bump completed, sync completed, init created `.omv` |
| `warn` | recoverable or operator-correctable anomalies | manifest missing, locale fallback used |
| `error` | command-ending failures | invalid state, atomic write failure, NTP hard failure |

## Structured Logging

Preferred fields:

- `command`
- `target_id`
- `language`
- `locale`
- `time_source`
- `version`
- `path`
- `error_kind`

Rules:

- prefer structured key/value logging through one logger, such as `tracing`
- never parse human log strings downstream
- user-facing localized messages must not be emitted through structured logs as
  the primary interface

## What to Log

- command start/end with command name and locale
- selected time source (`system`, `ntp`, `manual-confirmed`)
- target sync decisions
- file write targets and atomic-write success/failure
- locale fallback and catalog parity failures

## What NOT to Log

- secrets
- tokens
- passphrases
- raw private keys
- full credential-bearing URLs
- localized end-user copy as the only copy of an event

## Convention: CLI Output vs Logs

**What**: All direct CLI/TUI messages should come from i18n catalogs. Logs stay
developer-oriented and may remain English-only in V1.

**Why**: Operator output needs localization. Logs need stability and searchable
fields.

**Example**:

```rust
tracing::info!(command = "bump", version = %next_version, "bump completed");
eprintln!("{}", catalog.tf("cli.bump.success", &[("version", &next_version)]));
```
