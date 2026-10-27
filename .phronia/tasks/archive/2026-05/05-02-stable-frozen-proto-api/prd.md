# brainstorm: stable frozen proto api

## Goal

Evaluate whether `oh-my-versioning` needs a stable/frozen protobuf API mechanism now that proto has been introduced, so future version-to-version migrations can be managed safely instead of relying on a mutable current schema.

## What I already know

* The project has introduced protobuf artifacts, but the current mechanism may not yet distinguish mutable current API from frozen historical API versions.
* The user pointed to `wiremux` as a reference implementation with `sources/api/proto/versions/current/` and frozen numbered versions under `sources/api/proto/versions/<n>/`.
* The concern is forward compatibility and migration across OMV versions.
* OMV currently has exactly one proto source: `proto/omv/contract/v1/contract.proto`.
* `build.rs` compiles that one proto path directly, so there is no separate mutable `current` schema or frozen snapshot validation.
* `src/contract/registry.rs` exposes `CONTRACT_VERSION: u32 = 1`, but that value is handwritten and not tied to a frozen proto snapshot.
* `docs/OMV_CONTRACT_ARCHITECTURE.md` already says protobuf is the source for stable machine-readable contracts and lists migration/compatibility metadata as part of the intended contract.

## Assumptions (temporary)

* The stable/frozen API mechanism should protect persisted or externally consumed contracts, not every internal Rust type.
* The first useful scope is likely documentation plus validation/scaffolding around proto version directories, before adding a full migration runtime.
* Wiremux can be used as a local reference because its source cache is present under `target/external-scenarios/source-cache/`.

## Open Questions

* None.

## Requirements (evolving)

* Inspect current OMV proto usage and identify whether mutable schemas are used as durable contracts.
* Compare OMV's needs against wiremux's stable/frozen API convention.
* Propose a concrete MVP mechanism with explicit compatibility boundaries.
* Implement a wiremux-style layout plus guard tests.
* Backfill frozen `versions/1/contract.proto` and `versions/2/contract.proto` instead of treating the current proto as v1.
* Keep `versions/current/contract.proto` identical to `versions/2/contract.proto` after bootstrapping.

## Acceptance Criteria (evolving)

* [x] The PRD records current repo constraints and wiremux reference findings.
* [x] The selected approach defines where `current` proto files live and how frozen versions are produced.
* [x] The selected approach defines how breaking changes are detected or reviewed.
* [x] The selected approach states what migration/runtime behavior is in scope for MVP.
* [x] Frozen v1 omits generalized target-kind capabilities that were introduced with OMV target schema v2.
* [x] Frozen v2 includes generalized target-kind capabilities such as Markdown managed blocks, YAML scalar targets, C header macros, and Cargo workspaces.
* [x] Frozen v2 also includes current host integration capability fields because this bootstrap treats the current runtime contract as v2.
* [x] `versions/current/contract.proto` and `versions/2/contract.proto` are identical.

## Definition of Done (team quality bar)

* Tests added/updated when implementation changes behavior.
* Lint / typecheck / CI green for changed code.
* Docs/notes updated if behavior changes.
* Rollout/rollback considered if risky.

## Out of Scope (explicit)

* Public network API stability guarantees unless the repo already exposes one.
* Full historical migration runtime until we confirm persisted or generated artifacts need it.

## Technical Notes

* Initial reference paths from user:
  * `target/external-scenarios/source-cache/wiremux-2604.30.3/sources/api/proto/versions/README.md`
  * `target/external-scenarios/source-cache/wiremux-2604.30.3/sources/api/proto/versions/current/wiremux.proto`
  * `target/external-scenarios/source-cache/wiremux-2604.30.3/sources/api/proto/versions/1/wiremux.proto`

## Research Notes

### Current OMV shape

* `proto/omv/contract/v1/contract.proto` declares package `omv.contract.v1` and contains capability enums plus plan/capability summary messages.
* `build.rs` runs `prost_build::Config::new().compile_protos(&["proto/omv/contract/v1/contract.proto"], &["proto"])`.
* `src/contract/mod.rs` includes generated `omv.contract.v1.rs` from Cargo `OUT_DIR`.
* `src/contract/registry.rs` maps handwritten Rust capability enums to generated protobuf enum values and exposes `CONTRACT_VERSION = 1`.
* Existing docs distinguish compatibility domains: `.omv/*.toml` schema, target capability contract, automation JSON contract, AI/spec adapter contract, and host integration contract.
* Git history shows `a7a3ae8 feat(omv): add contract-driven target planning` introduced the proto file, v2 target storage/types, generalized adapters, and `docs/examples/complex-project-targets-v2.md` together.
* Git history shows `93ebb74 feat: platformize host integrations` later added `OmvIntegrationSupport` and `integration_support = 6` to the proto contract.
* There is no checked-in pre-v2 proto source to copy as v1; frozen v1 must be reconstructed from the semantic boundary that existed before generalized target kinds.

### Wiremux reference pattern

* Wiremux uses `sources/api/proto/versions/current/` as the latest editable schema and numbered directories such as `versions/1/` and `versions/2/` as frozen snapshots.
* Wiremux rules require every release that ships a changed `current/` schema to freeze a numbered snapshot.
* Wiremux forbids protobuf tag and enum numeric reuse and requires deleted fields/values to be reserved.
* Wiremux has compile-time `WIREMUX_PROTOCOL_API_VERSION_CURRENT` and `WIREMUX_PROTOCOL_API_VERSION_MIN_SUPPORTED` values.
* Wiremux has runtime compatibility classification for unsupported-old, supported, and unsupported-new API versions.
* Wiremux tests assert that `versions/current/wiremux.proto` matches the newest frozen snapshot and differs from older snapshots when expected.

### Constraints from OMV

* OMV is a local-first CLI, not a host/device protocol with two independently deployed binaries.
* The proto contract currently describes capability and automation-facing shapes, not serialized project storage; project storage is still `.omv/*.toml`.
* Future migration behavior likely needs to compare multiple domains rather than a single protocol number.
* The stable/frozen mechanism should prevent accidental breaking contract changes before it tries to implement full migration runtime.

### Feasible approaches here

**Approach A: Policy-only documentation**

* How it works: update architecture/spec docs to require frozen proto snapshots later, but leave layout and build unchanged.
* Pros: fast, low risk.
* Cons: does not actually prevent accidental changes; easy for future edits to mutate `v1` in place.

**Approach B: Wiremux-style layout plus guard tests** (Recommended)

* How it works: introduce `proto/omv/contract/versions/current/contract.proto` and `proto/omv/contract/versions/1/contract.proto`; compile latest frozen/current; add README rules; add tests or checks that `current` equals newest frozen, `CONTRACT_VERSION` equals newest frozen version, and tags/enum numbers are not reused without `reserved`.
* Pros: creates a real freeze workflow now, catches regressions, and keeps runtime migration separate.
* Cons: requires moving proto paths and updating build/include docs/tests.

**Approach C: Full compatibility runtime now**

* How it works: add layout plus min/current supported versions, API compatibility enum, and CLI diagnostics for old/new contract versions; start exposing these through plan/status.
* Pros: closest to wiremux, useful if OMV soon has independently versioned consumers.
* Cons: likely overbuilt before there is a concrete external SDK or persisted proto payload to migrate.

### Initial recommendation

Adopt Approach B for MVP. It gives OMV a real stable/frozen API discipline without prematurely building migration runtime. It should explicitly separate:

* proto contract API version: frozen snapshots under `proto/omv/contract/versions/<n>/`
* project schema version: `.omv/*.toml` compatibility
* structured JSON contract version: existing `STRUCTURED_JSON_CONTRACT_VERSION`
* AI adapter contract version: existing adapter contract version

### Proposed v1/v2 split

Backfill snapshots as contract snapshots, not storage schema files:

**Frozen proto v1**

* Package remains `omv.contract.v1` to avoid a Rust module namespace churn in this task.
* Target capabilities include only the original language/native manifest surface:
  * `RUST_CARGO_PACKAGE = 1`
  * `PYTHON_MANIFEST = 2`
  * `GO_MODULE = 3`
  * `JAVA_MAVEN = 4`
  * `C_CMAKE = 5`
  * `RUNTIME_EXPORT = 6`
* Command capabilities include the command surface that existed around the first plan-capable contract:
  * `CURRENT = 1`
  * `BUMP = 2`
  * `SYNC = 3`
  * `ADAPTER = 4`
  * `EVENT_FINALIZE_TASK = 5`
  * `PLAN = 6`
* Plan messages remain in v1 because `omv plan` and plan output are the first protobuf-backed contract surface.
* Tags/enum numbers 7+ are not reused.

**Frozen proto v2**

* Adds generalized target-kind capabilities:
  * `TEXT_SCALAR = 7`
  * `REGEX_REPLACE = 8`
  * `MARKDOWN_MANAGED_BLOCK = 9`
  * `YAML_SCALAR = 10`
  * `C_HEADER_MACRO = 11`
  * `CARGO_WORKSPACE = 12`
* Includes current host integration support:
  * `OmvIntegrationSupport`
  * `OmvCapabilitySet.integration_support = 6`
* Keeps the same message tags for existing fields.
* Updates runtime `CONTRACT_VERSION` to `2`.
* `versions/current/contract.proto` matches `versions/2/contract.proto` after bootstrapping.

**Host integration decision**

`OmvIntegrationSupport` and `OmvCapabilitySet.integration_support = 6` were added after generalized target kinds. Decision from user: include them in frozen v2 because this is the first formal stable/frozen API bootstrap and the current runtime contract should become v2. Do not create a v3 snapshot in this task.

## Decision (ADR-lite)

**Context**: OMV introduced protobuf after v2 target semantics had already entered the implementation, so the first proto file is semantically newer than a true v1 contract. The repo also has current host integration capabilities in the runtime contract.

**Decision**: Use a wiremux-style stable/frozen proto layout and backfill two frozen contract snapshots: v1 for original language-native targets and v2 for the current runtime contract, including generalized target kinds and host integration support. The editable/current schema should match `versions/2/contract.proto` after bootstrap.

**Consequences**: `CONTRACT_VERSION` should become `2`, `current` should match `versions/2`, and migration/runtime compatibility can remain limited to guard checks and contract metadata in this MVP. Future contract changes must create v3+ snapshots instead of mutating prior versions.

## Implementation Summary

* Added `proto/omv/contract/versions/README.md`.
* Backfilled frozen `proto/omv/contract/versions/1/contract.proto`.
* Moved the current runtime contract to `proto/omv/contract/versions/2/contract.proto`.
* Added `proto/omv/contract/versions/current/contract.proto` matching version 2.
* Updated `build.rs` to compile from `versions/current`.
* Updated `src/contract/registry.rs::CONTRACT_VERSION` to `2`.
* Added guard tests for current/latest equality, latest-version constant parity, and v1/v2 contract boundaries.
* Updated architecture and backend directory specs for the new stable/frozen proto layout.

## Verification

* `cargo fmt --check`
* `cargo test contract::registry --lib`
* `cargo test --all-targets --all-features`
* `cargo clippy --all-targets --all-features -- -D warnings`
