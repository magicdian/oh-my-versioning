# OMV Contract Architecture

This document describes the next architecture direction for OMV as a
local-first version-management tool. OMV remains focused on personal projects
and projects that may later grow into small-team workflows. It is not intended
to replace large-company release platforms with bespoke governance.

## Goals

- Keep `.omv/state.toml` as the version truth.
- Make version propagation deterministic before files are written.
- Treat CLI commands, AI hooks, Git hooks, CI, and future release integrations
  as triggers only.
- Move target synchronization behind typed adapters and a shared planning
  engine.
- Introduce a protobuf-backed capability contract with generated Rust code.
- Preserve current `.omv/targets.toml` V1 compatibility during the first
  refactor stage.
- Save extension seams for future SDK/plugin and release-trigger integrations
  without implementing those runtimes in the first two stages.

## Non-Goals

- Do not build a public plugin runtime in the first two stages.
- Do not implement GitHub CI, release triggers, or CI workflow files in the
  first two stages.
- Do not force existing projects to migrate target files during Stage 1.
- Do not replace language-native package files; OMV orchestrates and syncs
  them as derived outputs.

## Core Principle

The trigger does not decide what OMV changes. A trigger only requests an
operation. The persisted OMV project contract determines the version, target
set, adapter behavior, compatibility status, and write plan.

```text
.omv/state + .omv/targets + adapter capabilities + filesystem
  -> omv plan
  -> deterministic target statuses and proposed writes
  -> omv sync applies that plan
```

## Architecture

```text
                       Trigger Layer
  ┌──────────────────────────────────────────────────────────────┐
  │ CLI: current / bump / plan / sync / check / migrate          │
  │ AI and spec-framework hooks                                  │
  │ Future: Git hooks, CI, release host integrations             │
  └──────────────────────────────┬───────────────────────────────┘
                                 │ requests operation
                                 ▼
                         App Orchestration
  ┌──────────────────────────────────────────────────────────────┐
  │ load .omv config, state, targets, adapters, contracts         │
  │ resolve project root and locale                              │
  │ call version engine, planner, sync executor, migration check  │
  │ render text or structured JSON                               │
  └───────────────┬─────────────────────┬────────────────────────┘
                  │                     │
                  ▼                     ▼
          Core Version Truth       Contract Registry
  ┌────────────────────────┐   ┌─────────────────────────────────┐
  │ .omv/state.toml         │   │ proto-defined capability IDs     │
  │ version engine          │   │ supported target kinds           │
  │ time validation         │   │ supported command/json contracts │
  │ release policy          │   │ migration compatibility matrix   │
  └────────────┬───────────┘   └────────────────┬────────────────┘
               │                                │
               └──────────────┬─────────────────┘
                              ▼
                         Plan Engine
  ┌──────────────────────────────────────────────────────────────┐
  │ input: version truth, target TOML, adapter capabilities       │
  │ read current filesystem                                      │
  │ compute deterministic target statuses and proposed writes     │
  │ output: plan JSON, diagnostics, sync operations               │
  └──────────────────────────────┬───────────────────────────────┘
                                 │ dispatch by adapter and kind
                                 ▼
                         Target Adapters
  ┌────────────┬───────────┬───────────┬────────────┬────────────┐
  │ Cargo      │ CMake     │ Markdown  │ YAML       │ C Header   │
  │ package    │ manifest  │ badge/blk │ scalar     │ macro      │
  ├────────────┼───────────┼───────────┼────────────┼────────────┤
  │ Text       │ TOML      │ Runtime exports        │ Future...  │
  └────────────┴───────────┴───────────┴────────────┴────────────┘
                                 │
                  plan only      │      apply writes
                  ┌──────────────┴──────────────┐
                  ▼                             ▼
           Check / Drift Output             Sync Executor
  ┌────────────────────────────┐   ┌──────────────────────────────┐
  │ no mutation                 │   │ atomic write where possible   │
  │ CI and AI friendly          │   │ update derived files          │
  │ non-zero on required drift  │   │ record summary                │
  └────────────────────────────┘   └──────────────────────────────┘

                Persisted Project Surface
  ┌──────────────────────────────────────────────────────────────┐
  │ .omv/config.toml       profile and policies                  │
  │ .omv/state.toml        version truth                         │
  │ .omv/targets.toml      human-editable target instances        │
  │ .omv/adapters.toml     installed AI/spec host projections     │
  │ .omv/contracts/*       generated/readable capability metadata │
  │ .omv/ai/*              generated agent/spec guidance          │
  └──────────────────────────────────────────────────────────────┘
```

## Plan Command

`omv plan` is a first-class command in Stage 1. It exposes the deterministic
plan before mutation.

The plan should include:

- current version truth
- target id
- target kind and adapter
- affected project-relative paths
- current observed value
- expected value
- status: `ok`, `drift`, `missing`, `unsupported`, `error`, or `skipped`
- proposed write operation summary
- diagnostics and recovery hints

`omv sync --check` should share the same plan engine and provide a CI-friendly
drift gate. It should not mutate files and should return non-zero when required
targets drift or fail validation.

Stage 1 implementation note: the first implementation keeps the plan model
handwritten in Rust and maps its status values to generated protobuf enum
values. `omv plan` returns a successful plan even when drift exists;
`omv sync --check` returns a typed target error with the serialized plan in JSON
error details when required targets drift, are missing, unsupported, or errored.

## Target Adapter Model

Targets are human-editable TOML records. Adapters are Rust implementations that
understand one target kind or file family. The adapter computes status and
proposed writes; the sync executor applies those writes.

Stage 1 keeps V1 target files compatible while moving execution behind the
adapter boundary:

```toml
schema_version = 1

[[targets]]
id = "workspace-rust"
language = "rust"
root = "."
manifest_path = "Cargo.toml"
runtime_export_path = "src/generated/version.rs"
strategy = "intent-only"
enabled = true
```

Stage 2 formally enables V2 target kinds:

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
id = "readme-version-badge"
kind = "regex-replace"
adapter = "markdown"
path = "README.md"
pattern = "version-[0-9]+\\.[0-9]+\\.[0-9]+-blue"
template = "version-{version}-blue"
mode = "write"

[[targets]]
id = "component-manifest"
kind = "yaml-scalar"
adapter = "yaml"
path = "components/example/idf_component.yml"
key = "version"
template = "{version}"
mode = "write"

[[targets]]
id = "public-header-version"
kind = "c-header-macro"
adapter = "c-header"
path = "include/example_version.h"
macro = "EXAMPLE_VERSION"
template = "\"{version}\""
mode = "write"

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

Stage 2 implementation notes:

- Schema V1 target records remain compatible and continue to use
  `language`, `manifest_path`, and `runtime_export_path`.
- Schema V2 records use `kind` plus typed kind-specific fields. V1 and V2
  records may coexist in one schema V2 file.
- `yaml-scalar` intentionally supports simple mapping scalar paths only. It
  rejects sequences, anchors, aliases, and block scalars until OMV adopts a
  fuller YAML round-trip parser.
- `cargo-workspace` supports `members = "all"` with exact workspace members
  and one-level `prefix/*` globs. Its lockfile update strategy is narrow and
  deterministic: OMV updates matching workspace package version lines in
  `Cargo.lock` and does not run `cargo update`.

## Protobuf Contract

OMV uses protobuf as the source for stable machine-readable contracts. The
generated Rust code is an interface/stub layer. Handwritten Rust code owns
business behavior.

Proto should define:

- target capability IDs
- command/interface capability IDs
- structured plan and result shapes
- migration diagnostic shapes
- compatibility metadata

Example:

```proto
syntax = "proto3";

package omv.contract.v1;

enum OmvTargetSupport {
  OMV_TARGET_SUPPORT_UNSPECIFIED = 0;
  OMV_TARGET_SUPPORT_CARGO = 1;
  OMV_TARGET_SUPPORT_CMAKE = 2;
  OMV_TARGET_SUPPORT_MARKDOWN = 3;
  OMV_TARGET_SUPPORT_YAML = 4;
  OMV_TARGET_SUPPORT_TOML = 5;
  OMV_TARGET_SUPPORT_TEXT = 6;
  OMV_TARGET_SUPPORT_C_HEADER_MACRO = 7;
}

message OmvCapabilitySet {
  uint32 contract_version = 1;
  repeated OmvTargetSupport target_support = 2;
  repeated string command_support = 3;
  repeated string json_contract_support = 4;
}
```

Stage 2 registers generalized target capabilities for text scalars, regex
replacements, Markdown managed blocks, YAML scalars, C header macros, and Cargo
workspaces in the same contract registry used by `omv plan`, `omv sync
--check`, `omv sync`, and `omv bump`.

## Code Generation Policy

Stage 1 introduces protobuf code generation.

Rules:

- Use `build.rs` with Rust protobuf code generation, likely `prost-build`.
- Do not commit generated Rust code.
- Do not manually edit generated code.
- Treat `protoc` as an OMV developer and source-build dependency, not an
  end-user dependency.
- Released GitHub binaries must not require users to install protobuf tooling.
- Document the required `protoc` version or supported version range for
  contributors.
- Pin or install a known-good `protoc` version in CI.

The Stage 1 source build path is tested with `protoc` 34.0. Generated Rust is
emitted under Cargo `OUT_DIR` and included from `src/contract/mod.rs`; it is not
checked into the repository.

Layering:

```text
proto/omv/contract/v1/*.proto
        |
        v
build.rs / codegen
        |
        v
OUT_DIR generated contract/stub types
        |
        v
handwritten Rust implementation
  - capability registry construction
  - target adapter dispatch
  - plan engine
  - migration checks
  - JSON/text rendering
```

The generated layer must not contain OMV business logic. If contract behavior
changes, update `.proto`, handwritten implementation, and tests, then
regenerate during build.

## Compatibility Domains

OMV should keep these contracts distinct:

- project schema version: `.omv/*.toml` persisted format
- target capability contract: supported target kinds and adapter options
- automation JSON contract: command output consumed by scripts and AI tools
- AI/spec adapter contract: generated `.omv/ai/*` and host projections

Migration tooling should compare these domains and report whether a project is:

- current
- compatible but old
- missing capabilities
- using deprecated target records
- needing adapter refresh
- needing target migration

Stage 1 reports these statuses through `omv plan` diagnostics and summary
fields. It does not persist a `.omv/contracts/*` metadata directory yet; that
directory remains a future readable projection of the same registry data.

## Future Extension Seams

The first two stages do not implement external SDKs, plugin runtimes, or
release triggers. The architecture still reserves these seams:

- external target adapters can later map to the same plan operation contract
- CI and AI tools can consume plan JSON and capability metadata
- future release triggers can request the same core operations without owning
  target behavior
- migration analyzers can compare project-required capabilities with the
  installed OMV binary

## Staged Delivery

### Stage 1: Contract And Adapter Refactor

Goal: preserve existing behavior while replacing implicit language sync with
deterministic target planning and protobuf-backed capability contracts.

Subtasks:

1. Define OMV contract proto v1.
2. Add build-time protobuf code generation.
3. Add contract registry and capability reporting.
4. Introduce target adapter trait and plan model.
5. Migrate existing V1 targets into adapter-backed execution.
6. Add `omv plan`.
7. Add `omv sync --check` using the same plan engine.
8. Add migration/status reporting.
9. Update specs and tests.

Stage 1 must keep existing V1 target files compatible.

### Stage 2: Generalized Target Capabilities

Goal: add generalized target kinds for complex projects without hard-coding any
specific repository structure.

Subtasks:

1. Add generic text target.
2. Add Markdown badge and managed-block target.
3. Add YAML scalar target.
4. Add C header macro target.
5. Add Cargo workspace target with lockfile strategy.
6. Split broad C-family behavior into explicit CMake, header, and runtime
   export targets where needed.
7. Add a generic complex-project adoption sample.
8. Strengthen AI/spec guidance around `omv plan` and `omv sync --check`.

Team release triggers, public plugins, and external SDKs remain future work
after these two stages.
