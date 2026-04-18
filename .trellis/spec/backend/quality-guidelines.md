# Quality Guidelines

> Code quality standards for `omv` backend development.

---

## Overview

`omv` changes version truth. Bad writes or silent drift are worse than a failed
command. Backend quality work is therefore correctness-first.

## Forbidden Patterns

### Don't: Hardcode operator-facing strings in Rust code

All CLI and TUI copy must come from catalogs under `resources/i18n/`.

### Don't: Re-implement version formatting or bump logic in multiple modules

There must be one version engine.

### Don't: Let host adapters become a second source of truth

`AGENTS.md`, `CLAUDE.md`, OpenSpec files, and Trellis guides are projections of
`.omv/ai/*`, not canonical version policy stores.

### Don't: Mutate native manifests without going through a target adapter

This breaks cross-language consistency.

### Don't: Panic on expected operator failures

Invalid locale, malformed TOML, missing target manifest, and NTP failure are not
panic-worthy.

### Don't: Write `.omv` files non-atomically

Partial writes can corrupt the source of truth.

## Required Patterns

- typed enums for locale, build policy, version output, and target language
- atomic writes for `.omv` files
- localized CLI/TUI copy through catalogs
- adapter-based sync per language family
- adapter registry plus canonical `.omv/ai/*` generation for agent/spec
  projections
- parity tests between `en-US` and `zh-CN`
- `cargo clippy --all-targets --all-features -- -D warnings` as a blocking gate before merge

## Testing Requirements

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- unit tests for version calculation and time-validation branching
- persistence round-trip tests for all `.omv` files
- adapter tests for each supported language family
- adapter install/refresh tests for supported host projections
- locale parity/fallback tests
- integration tests for `omv init`, `omv current`, `omv bump`, `omv sync`,
  and `omv adapter ...`

When a command changes output semantics, add assertion coverage for:

- localized success/error message key paths
- structured JSON success/error envelope shape
- target sync result
- persisted `.omv` state

## Code Review Checklist

- Is `.omv` still the only truth source?
- Are `.omv/ai/*` and installed host adapters still thin projections?
- Are locale strings catalog-driven?
- Is version logic reused instead of copied?
- Are errors typed and localized at the boundary?
- Are structured JSON keys stable for automation?
- Does the change preserve the V1 flat target model?
- Are tests covering both `daily-reset` and `continuous` where relevant?
