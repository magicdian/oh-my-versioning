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

### Don't: Treat legacy adapter state as the integration source of truth

`.omv/adapters.toml` records projection recovery for compatibility. Provider
selection, detection snapshots, capability status, and capability failure
recovery belong in `.omv/integrations.toml`.

### Don't: Expose a public plugin runtime in MVP

MVP providers are internal registry entries. Do not load third-party provider
code, promise an SDK, or document public plugin installation as implemented
behavior.

### Don't: Mutate native manifests without going through a target adapter

This breaks cross-language consistency.

### Don't: Add command-specific target drift logic

`omv plan`, `omv sync --check`, `omv sync`, and post-`omv bump` sync must share
the same deterministic plan engine.

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
- protobuf contract source under `proto/` with generated Rust kept in `OUT_DIR`
- handwritten capability registry backed by generated contract enums
- deterministic plan status coverage for `ok`, `drift`, `missing`,
  `unsupported`, `error`, and `skipped`
- kind target adapters for text, regex, Markdown, YAML, C header, and Cargo
  workspace must return deterministic summaries rather than full file dumps
- structured formats should use structured parsing where practical; the current
  limited YAML scalar parser must reject unsupported YAML features explicitly
- adapter registry plus canonical `.omv/ai/*` generation for agent/spec
  projections
- internal integration provider registry with capability-granular statuses for
  `codex`, `trellis`, `project-instructions`, `host-skill`, `spec-guide`,
  `spec-index-snippet`, and `finalize-boundary`
- `.omv/integrations.toml` persistence using atomic writes
- `omv integrate apply` plan-before-mutate behavior with targeted worktree
  safety and non-zero partial-failure behavior
- parity tests between `en-US` and `zh-CN`
- `cargo clippy --all-targets --all-features -- -D warnings` as a blocking gate before merge

## Testing Requirements

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- unit tests for version calculation and time-validation branching
- persistence round-trip tests for all `.omv` files
- adapter tests for each supported language family
- adapter install/refresh tests for supported host projections
- integration storage round-trip and missing/malformed state tests
- integration status/apply tests for no state, safe apply, unsupported
  provider/capability, unsafe target file, and partial failure
- finalize-boundary helper tests for missing change type, task resolution,
  idempotency, and no silent semantic inference
- compatibility tests proving `omv adapter install/refresh/list/status` remain
  available while `omv integrate ...` becomes the forward command family
- locale parity/fallback tests
- integration tests for `omv init`, `omv current`, `omv bump`, `omv sync`,
  and `omv adapter ...`
- integration tests for `omv plan --json` and `omv sync --check`
- integration tests for mixed language/kind target planning, unknown-kind
  unsupported diagnostics, check failure without mutation, sync apply, and
  check success after sync

When a command changes output semantics, add assertion coverage for:

- localized success/error message key paths
- structured JSON success/error envelope shape
- target sync result
- persisted `.omv` state

## Code Review Checklist

- Is `.omv` still the only truth source?
- Are `.omv/ai/*` and installed host adapters still thin projections?
- Is `.omv/integrations.toml` the only integration desired-state/recovery
  source?
- Are legacy `omv adapter ...` commands compatibility paths rather than a new
  feature expansion surface?
- Does the change avoid public plugin runtime claims for MVP?
- Are locale strings catalog-driven?
- Is version logic reused instead of copied?
- Are errors typed and localized at the boundary?
- Are structured JSON keys stable for automation?
- Does the change preserve the V1 flat target model?
- Do all target writes flow through the shared plan/apply boundary?
- Are tests covering both `daily-reset` and `continuous` where relevant?
