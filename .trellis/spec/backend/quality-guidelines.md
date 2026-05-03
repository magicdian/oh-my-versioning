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

### Don't: Publish npm through long-lived tokens

Release workflows for `@magicdian/omv` must use npm Trusted Publishing/OIDC.
Do not add `NPM_TOKEN`, `NODE_AUTH_TOKEN`, `.npmrc` auth tokens, or package
publish secrets to GitHub Actions, docs, or generated release scripts.

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
- generated host projections must preserve host loader syntax before adding
  OMV metadata. In particular, Codex `SKILL.md` files must begin with YAML
  frontmatter; managed-file comments must not precede the opening `---`.
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
- finish-boundary projection tests must prove target drift is checked before
  finalization, explicit `omv sync --json` is the repair path, and
  non-semantic no-op finalizations are not described as target writes
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

## Scenario: GitHub Release and npm OIDC Distribution

### 1. Scope / Trigger

- Trigger: changing OMV release automation, package metadata, npm installer
  behavior, supported release targets, or user-facing installation docs.
- This is infrastructure/cross-layer work because Cargo metadata feeds `dist`,
  `dist` generates GitHub Actions and npm package artifacts, GitHub Releases
  host platform binaries, and npm Trusted Publishing exposes the install command
  users execute.

### 2. Signatures

Release configuration files:

```text
Cargo.toml
dist-workspace.toml
.github/dist-build-setup.yml
.github/scripts/install-protoc.sh
.github/workflows/release.yml
.github/workflows/npm-trusted-publish.yml
docs/RELEASING.md
README.md
```

Required package metadata:

```toml
[package]
name = "omv"
version = "<SemVer-compatible OMV version>"
description = "Date-based version management with one local source of truth."
homepage = "https://github.com/magicdian/oh-my-versioning"
repository = "https://github.com/magicdian/oh-my-versioning"
license = "MIT"
readme = "README.md"

[profile.dist]
inherits = "release"
lto = "thin"
```

Required dist configuration:

```toml
[dist]
cargo-dist-version = "0.31.0"
ci = "github"
github-build-setup = "../dist-build-setup.yml"
installers = ["npm"]
publish-jobs = ["./npm-trusted-publish"]
github-custom-job-permissions = { "npm-trusted-publish" = { "actions" = "read", "contents" = "read", "id-token" = "write" } }
npm-scope = "@magicdian"
hosting = "github"
```

Required targets:

```text
x86_64-apple-darwin
aarch64-apple-darwin
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
x86_64-pc-windows-msvc
aarch64-pc-windows-msvc
```

### 3. Contracts

- GitHub repository metadata must point at
  `https://github.com/magicdian/oh-my-versioning`.
- Git tags use `v<version>` and must match `Cargo.toml` package version.
- npm package name is `@magicdian/omv`; the installed binary command is `omv`.
- npm package version, Cargo package version, and GitHub release tag must refer
  to the same OMV version.
- GitHub Releases are the authoritative host for prebuilt binary artifacts.
- npm packages must install the binary from the matching GitHub Release tag, not
  from a mutable GitHub `latest` release.
- npm CI publishing must use Trusted Publishing/OIDC. Long-lived npm publish
  tokens are not an accepted fallback.
- When npm publishing is split into a reusable `workflow_call` workflow,
  configure npm Trusted Publishing with the caller workflow filename
  `release.yml`.
- First-time npm package bootstrap must happen outside the repository, under
  `/tmp`, and may use only an interactive npm login with 2FA.
- Protobuf contract generation requires `protoc`. Source-build environments,
  normal CI, and release CI must install `protoc` explicitly instead of hiding
  it behind a vendored Rust build dependency.
- `dist` release builds must install `protoc` through `.github/dist-build-setup.yml`
  so regenerated release workflows keep the setup step. In `dist-workspace.toml`,
  this file is referenced as `../dist-build-setup.yml` because `dist` resolves
  `github-build-setup` relative to `.github/workflows/`.
- `.github/dist-build-setup.yml` must call
  `bash ./.github/scripts/install-protoc.sh` because `dist` does not preserve a
  custom shell for injected setup steps; this keeps the same installer script
  valid on Linux, macOS, and Windows runners.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| `dist generate --mode ci --check` differs | fail before commit; regenerate from `dist-workspace.toml` |
| required target omitted from `dist-workspace.toml` | fail review; restore the full six-target matrix |
| workflow references `NPM_TOKEN`, `NODE_AUTH_TOKEN`, or npm auth token config | fail review; replace with OIDC trusted publishing |
| npm publish job runs before `host` succeeds | fail review; publish job must depend on `host` |
| npm package artifact missing or duplicated | `.github/workflows/npm-trusted-publish.yml` must exit non-zero |
| tag does not match package version | `dist` planning must fail or release operator must correct tag/version before publish |
| npm package does not exist yet | create placeholder only from `/tmp/npm-bootstrap-omv`, then configure Trusted Publishing |
| bootstrap package files appear in repository | fail review; remove them from source control |
| CI build fails with missing `protoc` | fail review; add/repair explicit `protoc` install in normal CI or `.github/dist-build-setup.yml` |

### 5. Good/Base/Bad Cases

Good:

```text
Cargo.toml version = 2605.2.1
git tag = v2605.2.1
npm package = @magicdian/omv@2605.2.1
release artifacts include all six required target archives
npm publish job uses id-token: write and no npm token secret
rust-quality.yml installs protobuf-compiler before cargo test/clippy
.github/dist-build-setup.yml invokes .github/scripts/install-protoc.sh before dist build
```

Base:

```text
pull_request runs dist plan only
tag push runs dist build/host, then custom npm trusted publish
README shows npm install -g @magicdian/omv
```

Bad:

```text
@magicdian/omv@2605.2.1 downloads GitHub Releases latest
release.yml references secrets.NPM_TOKEN
bootstrap package.json is committed to the repository
Linux aarch64 is treated as optional without an explicit product decision
```

### 6. Tests Required

- `dist generate --mode ci --check`
- `dist manifest --artifacts=all --output-format=json --no-local-paths`
  assertions:
  - package name is `@magicdian/omv`
  - release owner/repo is `magicdian/oh-my-versioning`
  - all six required targets appear in the artifacts matrix
  - npm installer artifact is `omv-npm-package.tar.gz`
- `dist build --artifacts=global --tag v<version> --output-format=json`
  assertions:
  - npm tarball is generated
  - generated package `bin.omv` points at the generated runner
  - generated package version matches `Cargo.toml`
- `rg "NPM_TOKEN|NODE_AUTH_TOKEN|secrets\\.NPM|npm_token" .github README.md docs dist-workspace.toml`
  should find only prohibition docs/comments, never executable secret usage.
- Standard Rust gates still pass:
  - `cargo fmt --check`
  - `cargo test --all-targets --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings`
- CI source-build assertion:
  - normal CI installs `protobuf-compiler` before Cargo builds on Ubuntu
  - release CI injects `.github/dist-build-setup.yml` before `dist build`
  - generated workflow calls `bash ./.github/scripts/install-protoc.sh`
  - generated release workflow still passes `dist generate --mode ci --check`

### 7. Wrong vs Correct

Wrong:

```yaml
- name: Publish npm
  env:
    NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
  run: npm publish --access public
```

Correct:

```yaml
permissions:
  contents: read
  id-token: write

steps:
  - run: npm publish "${{ steps.npm-package.outputs.path }}" --access public
```

Wrong:

```bash
mkdir npm-bootstrap-omv
cd npm-bootstrap-omv
npm publish --access public --tag bootstrap
git add package.json
```

Correct:

```bash
mkdir -p /tmp/npm-bootstrap-omv
cd /tmp/npm-bootstrap-omv
npm publish --access public --tag bootstrap
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
