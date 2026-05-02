# Stage 2: Generalized Target Capabilities

## Goal

Add generalized target schema V2 capabilities for complex projects after Stage 1 has introduced protobuf-backed contracts, deterministic planning, and adapter-backed V1 execution.

Stage 2 must remain generic. It must not hard-code any external repository structure or mention the private research case study in code or durable docs.

## Dependency

This task is blocked until Stage 1 is implemented and reviewed:

* `.trellis/tasks/05-01-omv-contract-adapter-refactor`

Stage 2 assumes:

* `omv plan` exists.
* `omv sync --check` exists.
* The plan engine is shared by check and write flows.
* Target adapters can report status and proposed writes.
* The protobuf contract can represent target capabilities.

## Requirements

* Formally enable `.omv/targets.toml` schema V2.
* Keep V1 compatibility and migration reporting.
* Add generic V2 target kinds:
  * `text-scalar`
  * `regex-replace`
  * `markdown-managed-block`
  * `yaml-scalar`
  * `c-header-macro`
  * `cargo-workspace`
* Add adapter capabilities for those target kinds to the protobuf contract.
* Ensure each new target kind participates in:
  * `omv plan`
  * `omv sync --check`
  * `omv sync`
  * `omv bump` target synchronization
  * structured JSON output
  * migration/status diagnostics
* Add a generic complex-project adoption sample under docs or examples.
* Update AI/spec guidance so agents use `omv plan` and `omv sync --check` before release-sensitive changes.
* Do not implement public plugin runtime or external SDK support.
* Do not implement GitHub CI/release trigger code or workflow files.

## Non-Goals

* Do not redo Stage 1's contract architecture.
* Do not add project-specific target presets.
* Do not mention the private research case-study project name in code, docs, tests, examples, or comments.
* Do not implement team release batching or GitHub release automation.

## Target Kind Details

### `text-scalar`

Use cases:

* Root `VERSION`-style files.
* Generated one-value text files.

Required behavior:

* Support whole-file replacement.
* Preserve or intentionally normalize trailing newline according to target config.
* Detect missing files.
* Plan output should show current scalar and expected scalar.

Example:

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

### `regex-replace`

Use cases:

* Badges.
* Simple inline documentation values.

Required behavior:

* Support one or many replacements as explicitly configured.
* Fail on zero matches unless `missing = "insert"` or equivalent future behavior is defined.
* Fail on ambiguous multiple matches unless target config permits multiple replacements.
* Avoid regex-based parsing where a structured adapter exists.

Example:

```toml
[[targets]]
id = "readme-version-badge"
kind = "regex-replace"
adapter = "markdown"
path = "README.md"
pattern = "version-[0-9]+\\.[0-9]+\\.[0-9]+-blue"
template = "version-{version}-blue"
mode = "write"
```

### `markdown-managed-block`

Use cases:

* Generated release snippets.
* Version policy snippets.
* Documentation sections owned by OMV.

Required behavior:

* Use explicit begin/end markers.
* Replace only the managed block.
* Fail if markers are missing, duplicated, or inverted.
* Do not alter unmanaged markdown content.

### `yaml-scalar`

Use cases:

* Simple package/component manifests with top-level or nested scalar version fields.

Required behavior:

* Support scalar key paths.
* Preserve simple common formatting where practical.
* Define unsupported YAML features clearly if the implementation uses a limited parser.
* Prefer structured YAML parsing over regex.

Example:

```toml
[[targets]]
id = "component-manifest"
kind = "yaml-scalar"
adapter = "yaml"
path = "components/example/idf_component.yml"
key = "version"
template = "{version}"
mode = "write"
```

### `c-header-macro`

Use cases:

* Public C/C++ header version macros.

Required behavior:

* Update string and numeric `#define` values.
* Fail on missing macro unless insert behavior is explicitly configured.
* Fail on duplicate macro definitions.
* Preserve surrounding header content.

Example:

```toml
[[targets]]
id = "public-header-version"
kind = "c-header-macro"
adapter = "c-header"
path = "include/example_version.h"
macro = "EXAMPLE_VERSION"
template = "\"{version}\""
mode = "write"
```

### `cargo-workspace`

Use cases:

* Multiple Rust crates that must share one release version.

Required behavior:

* Discover workspace members from a workspace root.
* Support `members = "all"` initially.
* Support same-version policy.
* Update member package versions or `[workspace.package] version` depending on configuration.
* Check and optionally update `Cargo.lock`.
* Plan output should list affected manifests and lockfile action.

Example:

```toml
[[targets]]
id = "rust-workspace"
kind = "cargo-workspace"
adapter = "cargo"
root = "tools/example"
members = "all"
version_policy = "same"
lockfile = "update"
mode = "write"
```

## Implementation Steps

1. Finalize target schema V2
   * Extend storage model to parse V2 target records.
   * Preserve V1 parsing and V1-to-plan compatibility.
   * Add clear validation errors for malformed V2 targets.

2. Extend protobuf contract
   * Add capability enum values for Stage 2 target kinds.
   * Add plan/result fields if Stage 2 needs richer operation details.
   * Regenerate code through build-time codegen.

3. Add target config domain types
   * Introduce typed Rust target config variants for each V2 kind.
   * Avoid stringly-typed dispatch after parsing.

4. Implement `text-scalar` and `regex-replace`
   * Add planner and apply logic.
   * Add unit tests for ok, drift, missing, ambiguous, and apply behavior.

5. Implement `markdown-managed-block`
   * Add marker parser.
   * Add replacement planner/apply logic.
   * Add tests for missing/duplicate/inverted markers.

6. Implement `yaml-scalar`
   * Choose parser strategy.
   * Add key-path update support.
   * Add tests for simple top-level and nested scalar cases.

7. Implement `c-header-macro`
   * Add macro detection and replacement.
   * Add tests for string macro, numeric macro, missing macro, duplicate macro.

8. Implement `cargo-workspace`
   * Parse workspace root manifest.
   * Discover members.
   * Update or check member package versions.
   * Implement lockfile `check` and `update` behavior.
   * Add integration tests with temporary workspaces.

9. Integrate all new adapters with plan/check/sync/bump
   * All target kinds must report deterministic plan status.
   * `sync --check` must fail on required drift.
   * `sync` and `bump` must apply planned writes.

10. Add migration/status support
   * Report V1 projects as compatible.
   * Suggest V2 target records where a generic target is a better fit.
   * Do not auto-migrate without explicit operator command.

11. Add generic complex-project sample
   * Use neutral names and paths.
   * Cover docs, Cargo workspace, YAML, C header macro, and text scalar targets.
   * Do not mention the private research case-study project name.

12. Update AI/spec guidance and docs
   * Guidance should instruct agents to run `omv plan` or `omv sync --check` before release-sensitive edits.
   * Update backend specs and `docs/OMV_CONTRACT_ARCHITECTURE.md` as needed.

13. Tests and verification
   * Unit tests for each adapter.
   * Integration tests for mixed V2 target sync.
   * Integration tests for JSON plan output with mixed V2 targets.
   * Regression tests proving V1 targets still work.
   * Run `cargo fmt --check`, `cargo test --all-targets --all-features`, and `cargo clippy --all-targets --all-features -- -D warnings` if available.

## Expected Result

* `.omv/targets.toml` schema V2 is supported.
* Generic target kinds cover text, markdown, YAML, C header macros, and Cargo workspaces.
* New capabilities are represented in the protobuf contract.
* `omv plan`, `omv sync --check`, `omv sync`, and `omv bump` all work with V2 targets.
* Existing V1 projects remain compatible.
* Docs include a generic complex-project target sample.

## Acceptance Criteria

* [ ] V2 target parsing and validation are implemented.
* [ ] V1 target compatibility remains tested.
* [ ] Protobuf contract includes Stage 2 capability values.
* [ ] `text-scalar` target is implemented and tested.
* [ ] `regex-replace` target is implemented and tested.
* [ ] `markdown-managed-block` target is implemented and tested.
* [ ] `yaml-scalar` target is implemented and tested.
* [ ] `c-header-macro` target is implemented and tested.
* [ ] `cargo-workspace` target is implemented and tested.
* [ ] Mixed V2 target `omv plan --json` is tested.
* [ ] Mixed V2 target `omv sync --check` drift behavior is tested.
* [ ] Mixed V2 target `omv sync` write behavior is tested.
* [ ] `omv bump` sync behavior includes V2 targets.
* [ ] Generic complex-project sample is added without project-specific names.
* [ ] Format, tests, and clippy pass or documented blockers are reported.

## Review Focus

* No project-specific hard-coding.
* Structured parsers should be preferred over regex for structured formats.
* Target plans should be deterministic and explainable.
* V1 compatibility must remain intact.
* V2 target schema should be typed enough to avoid ambiguous adapter behavior.
