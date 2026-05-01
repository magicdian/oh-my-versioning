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
- ignored external scenario tests for production-like project fixtures when
  target-sync behavior depends on real repository layout

When a command changes output semantics, add assertion coverage for:

- localized success/error message key paths
- structured JSON success/error envelope shape
- target sync result
- persisted `.omv` state

## Scenario: External Project Scenario Tests

### 1. Scope / Trigger

- Trigger: validating `omv plan`, `omv sync`, `omv sync --check`, or `omv bump`
  against a real external repository instead of a synthetic temp fixture.
- This is cross-layer because the flow covers git checkout, committed scenario
  TOML, `.omv` fixture overlay, app runtime time-source injection, target
  adapters, structured JSON output, and tracked-file diffs.

### 2. Signatures

External scenario tests live under `tests/external_scenarios.rs` and committed
fixtures live under:

```text
tests/external_scenarios/<scenario-id>/
├── scenario.toml
└── omv/
    ├── config.toml
    ├── state.toml
    └── targets.toml
```

The app runtime injection seam is:

```rust
pub struct AppRuntime<'a> {
    pub ntp_source: &'a dyn TimeSource,
    pub system_source: &'a dyn TimeSource,
}

pub fn run(cli: Cli) -> Result<AppOutput, OmvError>;
pub fn run_with_runtime(cli: Cli, runtime: &AppRuntime<'_>) -> Result<AppOutput, OmvError>;
```

Production `run` constructs real `NtpTimeSource::default()` and
`SystemTimeSource`; tests use `run_with_runtime` with fixed `TimeSource`
implementations.

### 3. Contracts

Scenario TOML must declare:

```toml
id = "wiremux-2604.30.3"
repo = "https://github.com/magicdian/wiremux.git"
tag = "2604.30.3"
commit = "207fb016c28f82cde971ab4e4ab175a274832ee9"
expected_version = "2605.1.1"
expected_drift = 7
expected_synced = 7
expected_ok = 7

[[assertions]]
path = "VERSION"
text = "2605.1.1"
```

Rules:

- external tests are `#[ignore]` and run explicitly with:

  ```bash
  cargo test --test external_scenarios -- --ignored --nocapture
  ```

- normal `cargo test --all-targets --all-features` must compile external
  scenario tests but must not run network-dependent scenarios
- source caches and runtime worktrees live only under
  `target/external-scenarios/`
- checkout must verify the resolved `HEAD` equals the committed scenario
  `commit`; tag text alone is not enough
- scenario `.omv/*.toml` fixtures should omit `schema_version`; it is internal
  compatibility metadata unless a test explicitly covers schema behavior
- default cleanup is clean-on-success and preserve-on-failure
- `OMV_EXTERNAL_KEEP=1` preserves scenario worktrees even on success
- success output should print `[PASS]` steps plus the tracked files changed by
  OMV so manual testers can inspect what happened
- downstream builds such as `idf.py build` are not part of the default external
  scenario path unless a later scenario explicitly adds them

### 4. Validation & Error Matrix

| Condition | Behavior |
| --- | --- |
| source cache missing | clone `repo` at `tag` into `target/external-scenarios/source-cache/<id>` |
| source cache exists | reuse it after commit verification |
| checked-out `HEAD` differs from `commit` | fail before overlay or OMV mutation |
| `.omv` fixture missing required file | fail before running OMV |
| `omv plan --json` reports unexpected drift count | fail and preserve workspace |
| `omv sync --json` reports wrong version or synced count | fail and preserve workspace |
| `omv sync --check --json` reports drift after sync/bump | fail and preserve workspace |
| assertion file missing or lacks expected text | fail and preserve workspace |
| scenario succeeds and `OMV_EXTERNAL_KEEP` is unset | remove runtime worktree |
| scenario succeeds and `OMV_EXTERNAL_KEEP=1` | preserve runtime worktree and print path |

### 5. Good/Base/Bad Cases

Good:

```text
wiremux external scenario:
  plan reports 7 drift
  sync reports version 2605.1.1 and synced 7
  sync --check reports ok 7 and drift 0
  14 declared file assertions pass
```

Base:

```text
normal cargo test:
  external scenario tests compile
  ignored network tests do not run
```

Bad:

```text
scenario uses only a tag without commit pinning
scenario asserts the whole repository no longer contains the old version
scenario commits checked-out external project files or runtime worktrees
```

### 6. Tests Required

- ignored sync scenario: checkout pinned fixture, overlay `.omv`, run
  `plan/sync/sync --check`, assert declared files reached the target version
- ignored bump scenario: use `run_with_runtime` to inject fixed same-day and
  next-day dates, verify same-day increments and next-day daily reset in the
  external worktree, then run `sync --check`
- normal test run: `cargo test --test external_scenarios` compiles and reports
  the scenarios as ignored
- assertion points:
  - `expected_drift`, `expected_synced`, and `expected_ok`
  - scenario `commit`
  - declared target file contents
  - tracked file diff summary after sync/bump

### 7. Wrong vs Correct

Wrong: run external scenarios in the repository root and rely on live NTP.

```rust
let output = Command::new("omv").arg("bump").output()?;
```

Correct: run in an ignored isolated worktree and inject deterministic time when
testing bump behavior.

```rust
let runtime = AppRuntime {
    ntp_source: &fixed_ntp,
    system_source: &fixed_system,
};
run_with_runtime(cli, &runtime)?;
```

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
