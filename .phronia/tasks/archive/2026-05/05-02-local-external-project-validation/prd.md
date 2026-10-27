# Brainstorm: Local External Project Validation

## Goal

Design a local validation workflow for `omv` that can exercise real projects,
integrate OMV-generated components, and verify that `plan` and `sync` update
versions correctly in production-like scenarios.

## What I already know

* `omv` is primarily a helper tool for other projects, so isolated unit tests do
  not fully prove real-world behavior.
* The desired validation flow is similar in spirit to `ctest`/`gtest` test
  orchestration or a small local CI/CD runner like Jenkins/GitHub Actions, but
  running locally.
* `wiremux` is a candidate external project for the first integration fixture.
* The proposed fixture version is
  `https://github.com/magicdian/wiremux/releases/tag/2604.30.3`.
* The current repo already has deterministic Rust integration tests in
  `tests/integration/target_sync.rs` that create temporary project roots, seed
  `.omv` state/targets, call `app::run`, and assert file contents.
* Current tests cover `plan`, `sync`, `sync --check`, mixed V2 target kinds,
  and `integrate status/apply`, but they do not execute the packaged CLI binary
  against a real checked-out repository.
* Current CI runs `cargo fmt --check`, `cargo test --all-targets --all-features`,
  and `cargo clippy --all-targets --all-features -- -D warnings`.
* The validation framework should support multiple external projects over time.
  `wiremux` is only the first fixture.
* The validation goal is not to build downstream projects such as via
  `idf.py build`; it is to verify that all configured OMV-observed files inside
  the test project are synchronized to the expected new version without gaps.
* Scenario definitions should use TOML.
* The `wiremux` MVP fixture should record the tag URL and the resolved commit
  id for stability:
  * tag: `2604.30.3`
  * commit: `207fb016c28f82cde971ab4e4ab175a274832ee9`
* Test runtime artifacts must not be committed to this repository. Runtime
  clones, working copies, caches, logs, and generated outputs should live under
  an ignored directory such as `target/`.
* OMV already has a `TimeSource` trait in `src/core/time/mod.rs` and unit tests
  with fixed/failing time sources.
* The remaining testability gap is the app/CLI boundary: `app::run` constructs
  real `NtpTimeSource::default()` and `SystemTimeSource`, so binary-like flows
  cannot inject fixed dates for deterministic bump tests.

## Assumptions (temporary)

* The validation workflow should be deterministic and suitable for local
  developer use first, not a remote CI service.
* The first MVP should validate one real external fixture before generalizing
  to many project types.
* Network access during validation should be controlled or made explicit.
* Building `wiremux` itself should be optional at first because ESP-IDF and
  hardware/toolchain setup are heavier than OMV's target-sync validation need.
* Scenario definitions should be data-driven enough that adding a second
  project does not require rewriting the runner.
* Scenario configuration files are committed, but scenario runtime outputs are
  not.
* Version bump tests need deterministic same-day and next-day date control
  without depending on live NTP or the developer machine date.
* Scenario cleanup should default to clean-on-success and preserve-on-failure.
* `OMV_EXTERNAL_KEEP=1` should preserve the runtime workspace regardless of
  success or failure.

## Open Questions

* Which MVP scope should be chosen: a Rust integration test harness, a local
  scenario runner command/script, or a full local CI-style workflow?

## Requirements (evolving)

* Validate `omv plan` and `omv sync` against at least one real external project
  fixture.
* Use a pinned external project version for reproducibility.
* Avoid mutating the developer's real working copy.
* Separate fast deterministic tests from slower network/toolchain scenario
  validation.
* Model external validation as a suite of named project scenarios, even if MVP
  ships with only `wiremux`.
* Each scenario must declare the upstream source/tag, the OMV fixture state, the
  expected new version, and the files/assertions that prove sync coverage.
* Each scenario must declare the resolved commit id in addition to the human
  readable tag so tag movement or accidental retagging is detectable.
* Scenario runs must execute in a gitignored workspace, preferably under
  `target/external-scenarios/`.
* Downstream project builds are out of the default MVP path.
* Time-source injection should be available for tests without making NTP mocks a
  user-facing production feature.
* Scenario cleanup behavior:
  * success: clean runtime workspace by default
  * failure: preserve runtime workspace and print the path
  * `OMV_EXTERNAL_KEEP=1`: preserve runtime workspace regardless of result

## Acceptance Criteria (evolving)

* [x] A documented local workflow can fetch or prepare the external fixture.
* [x] The workflow can apply OMV integration steps in an isolated workspace.
* [x] The workflow can run `plan` and `sync`.
* [x] The workflow can assert expected version changes in target manifests.
* [x] Failure output is actionable enough to debug broken integration behavior.
* [x] Adding another project scenario requires adding a scenario definition plus
      assertions, not duplicating the entire runner.
* [x] Scenario success proves all declared OMV-observed files reached the
      expected version after sync.
* [x] Runtime clones, generated `.omv` state, logs, and modified fixture working
      trees are created only under ignored paths.
* [x] The `wiremux` scenario refuses to run, or reports a clear error, if the
      checked-out commit does not match
      `207fb016c28f82cde971ab4e4ab175a274832ee9`.
* [x] Tests can verify same-day bump increments and next-day daily-reset
      behavior with fixed dates.
* [x] External sync scenarios do not depend on live NTP.
* [x] Successful scenario runs clean their workspace by default.
* [x] Failed scenario runs preserve their workspace for debugging.
* [x] `OMV_EXTERNAL_KEEP=1` preserves scenario workspaces even on success.

## Definition of Done (team quality bar)

* Tests added/updated where appropriate.
* Lint / typecheck / CI green.
* Docs/notes updated if behavior changes.
* Rollout/rollback considered if risky.

## Out of Scope (explicit)

* Hosting a remote CI/CD service.
* Supporting arbitrary third-party projects without explicit scenario
  definitions in the first iteration.
* Publishing or mutating remote repositories.
* Running downstream project builds such as `idf.py build` by default.
* Committing checked-out external project trees or scenario run outputs.

## Technical Notes

* Initial task created from brainstorm discussion on local real-project
  validation for `omv`.
* Relevant local files inspected:
  * `Cargo.toml`
  * `.github/workflows/rust-quality.yml`
  * `.gitignore`
  * `src/core/time/mod.rs`
  * `src/core/time/ntp.rs`
  * `src/core/versioning/engine.rs`
  * `src/app/mod.rs`
  * `tests/integration/target_sync.rs`
  * `docs/examples/complex-project-targets-v2.md`
  * `.trellis/spec/backend/quality-guidelines.md`
  * `.trellis/spec/backend/directory-structure.md`
* `git ls-remote https://github.com/magicdian/wiremux.git refs/tags/2604.30.3`
  returned `207fb016c28f82cde971ab4e4ab175a274832ee9`.
* Current time design:
  * `TimeSource` already abstracts date lookup.
  * `validate_current_date` already accepts `ntp_source` and `system_source`.
  * `execute_bump`, `execute_finalize_task`, and `execute_finalize_boundary`
    already accept injected sources internally.
  * `run_bump` and event dispatch paths construct real sources, so public
    `app::run` is not fully injectable.
* `wiremux` fixture analysis:
  * checked out `2604.30.3` under
    `target/external-scenarios/source-cache/wiremux-2604.30.3`
  * verified HEAD is `207fb016c28f82cde971ab4e4ab175a274832ee9`
  * current version appears in root `VERSION`, README badges, ESP component
    metadata/header, Rust workspace crate manifests, Cargo.lock, and release
    docs
  * release history and tag documentation intentionally contain historical
    `2604.30.3` references and must not be treated as sync failures
* Manual wiremux config check:
  * scratch worktree:
    `target/external-scenarios/runs/manual-wiremux-config-check`
  * test version: `2605.1.1`
  * `omv plan --json`: 7 drift, 0 missing/unsupported/error
  * `omv sync --json`: synced 7, skipped 0
  * `omv sync --check --json`: 7 ok, 0 drift
  * changed files matched the declared target set plus untracked `.omv/`

## Wiremux MVP Scenario Draft

Recommended scenario metadata:

```toml
id = "wiremux-2604.30.3"
repo = "https://github.com/magicdian/wiremux.git"
tag = "2604.30.3"
commit = "207fb016c28f82cde971ab4e4ab175a274832ee9"
expected_version = "2605.1.1"
```

Recommended `.omv/state.toml` fixture:

```toml
schema_version = 1
logical_date = "2026-05-01"
build_number = 1
last_issued_version = "2605.1.1"
last_time_source = "system"
```

Recommended `.omv/targets.toml` fixture:

```toml
schema_version = 2

[[targets]]
id = "root-version-file"
kind = "text-scalar"
adapter = "text"
path = "VERSION"
selector = "whole-file"
template = "{version}\n"
mode = "write"

[[targets]]
id = "readme-badge-en"
kind = "regex-replace"
adapter = "markdown"
path = "README.md"
pattern = "version-[0-9]+\\.[0-9]+\\.[0-9]+-blue"
template = "version-{version}-blue"
mode = "write"

[[targets]]
id = "readme-badge-zh"
kind = "regex-replace"
adapter = "markdown"
path = "README_CN.md"
pattern = "version-[0-9]+\\.[0-9]+\\.[0-9]+-blue"
template = "version-{version}-blue"
mode = "write"

[[targets]]
id = "esp-component-version"
kind = "yaml-scalar"
adapter = "yaml"
path = "sources/vendor/espressif/generic/components/esp-wiremux/idf_component.yml"
key = "version"
template = "\"{version}\""
mode = "write"

[[targets]]
id = "esp-public-header-version"
kind = "c-header-macro"
adapter = "c-header"
path = "sources/vendor/espressif/generic/components/esp-wiremux/include/esp_wiremux.h"
macro = "ESP_WIREMUX_VERSION"
template = "\"{version}\""
mode = "write"

[[targets]]
id = "host-rust-workspace"
kind = "cargo-workspace"
adapter = "cargo"
root = "sources/host/wiremux"
members = "all"
version_policy = "same"
version_location = "member-packages"
lockfile = "update"
mode = "write"

[[targets]]
id = "release-doc-current-version"
kind = "regex-replace"
adapter = "markdown"
path = "docs/esp-registry-release.md"
pattern = "Current release: `[^`]+`\\."
template = "Current release: `{version}`."
mode = "write"
```

Recommended assertions:

* `VERSION` contains `2605.1.1`
* `README.md` contains `version-2605.1.1-blue`
* `README_CN.md` contains `version-2605.1.1-blue`
* ESP `idf_component.yml` contains `version: "2605.1.1"`
* ESP public header contains `#define ESP_WIREMUX_VERSION "2605.1.1"`
* each host Rust crate `Cargo.toml` contains `version = "2605.1.1"`
* host `Cargo.lock` contains workspace package versions at `2605.1.1`
* `docs/esp-registry-release.md` contains the current-release line for
  `2605.1.1`
* `docs/esp-registry-release.md` may still contain historical `2604.30.3`
  references in release history and tag documentation

## Research Notes

### What similar tools do

* CTest is a test driver for CMake-generated build trees and also has
  build-and-test/dashboard script modes. This maps well to a scenario-runner
  idea: prepare source/build dirs, run steps, collect results.
* GoogleTest is a unit/integration test framework, useful for code-level
  assertions but not enough by itself for cross-repository CLI workflows.
* Rust CLI projects commonly use `assert_cmd` for invoking the built binary,
  `trycmd` for many snapshot-like command fixtures, and `snapbox` when custom
  filesystem/output assertions are needed.
* `act` can run GitHub Actions locally through Docker, but its runner images
  intentionally differ from GitHub-hosted runners in some ways.
* Dagger is a local-first CI/CD automation engine that runs locally or in CI
  with a container runtime, but it is a larger platform choice than OMV needs
  for the first fixture.

### Constraints from this repo/project

* OMV's quality specs require `plan`, `sync --check`, `sync`, and post-`bump`
  sync to share one deterministic plan engine.
* Target writes must flow through target adapters, not scenario-specific
  patching.
* `.omv/` remains the source of truth; native manifests are derived outputs.
* Network-dependent checks should not be part of every `cargo test` run unless
  they are explicitly gated.

### Feasible approaches here

**Approach A: Ignored Cargo external scenario tests** (Recommended)

* How it works: add a test module that downloads/checks out pinned fixtures into
  a temp/cache directory, loads named scenario definitions, overlays `.omv`
  scenario config, runs the built `omv` binary, and asserts JSON plus declared
  file expectations. Mark it `#[ignore]` or gate with an env var.
* Pros: fits current Rust test workflow, easy to run locally, can later enter CI
  as a scheduled/manual job, and can support multiple projects by iterating over
  scenario definitions.
* Cons: requires careful fixture cache/network handling and can become slow.
* Refined decision: scenario definitions should be committed TOML files; runtime
  workspaces should live under `target/external-scenarios/` or another ignored
  directory.

**Approach B: `cargo xtask` / local scenario runner**

* How it works: add a small Rust automation command such as
  `cargo xtask scenario wiremux-2604.30.3` or `cargo xtask scenario --all` that
  performs checkout, OMV setup, plan/sync/check, declared assertions, optional
  downstream build, and artifact cleanup.
* Pros: better UX for local CI-like workflows and future multi-project suites.
* Cons: more scaffolding than the first fixture strictly needs.

**Related approach: App runtime injection for deterministic time**

* How it works: introduce an internal app runtime/context wrapper, for example
  `app::run_with_runtime(cli, &runtime)`, where `runtime` provides NTP and
  system `TimeSource` references. Keep `app::run(cli)` as the production
  wrapper that uses real sources.
* Pros: no production CLI test flags, deterministic same-day/next-day bump
  tests, reuses the existing `TimeSource` trait.
* Cons: binary-only tests still cannot pass an in-process mock; external
  scenario tests should focus on `sync` unless an explicit internal test hook is
  later added.

**Approach C: Use `act` or Dagger as the local CI layer**

* How it works: encode OMV validation as a local/CI pipeline and let the runner
  orchestrate checkout/build/test steps.
* Pros: closer to CI/CD semantics and containerized reproducibility.
* Cons: Docker dependency, more moving parts, and likely overkill before OMV has
  several external fixtures.

## Decision (ADR-lite)

**Context**: OMV needs production-like validation against external repositories,
but must remain deterministic, local-first, and lightweight.

**Decision**: Build a TOML-driven external scenario test suite. MVP includes one
`wiremux-2604.30.3` scenario pinned by tag and commit. Runtime clones and
generated artifacts live under ignored `target/external-scenarios/`. Scenario
tests validate `plan`, `sync`, and `sync --check` coverage only; downstream
project builds are not part of the default path. Add an app runtime injection
point for deterministic time-source tests rather than exposing test-only CLI
flags.

**Consequences**: The suite can grow to multiple projects without rewriting the
runner, but network checkout remains a slow explicit test path. Commit pinning
and workspace cleanup rules are required to keep scenarios stable and keep the
repository clean.
