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
- Model host integrations as provider/capability state with deterministic
  status and apply behavior.
- Keep `.omv/ai/*` canonical for generated guidance while treating installed
  host files as replaceable projections.
- Preserve current `.omv/targets.toml` V1 compatibility during the first
  refactor stage.
- Save extension seams for future SDK/plugin and release-trigger integrations
  without implementing those runtimes in the first two stages or the host
  integration MVP.

## Non-Goals

- Do not build a public plugin runtime in the first two stages or the host
  integration MVP. MVP providers are internal registry entries.
- Do not implement GitHub CI, release triggers, or CI workflow files in the
  first two stages.
- Do not force existing projects to migrate target files during Stage 1.
- Do not replace language-native package files; OMV orchestrates and syncs
  them as derived outputs.

## Core Principle

The trigger does not decide what OMV changes. A trigger only requests an
operation. The persisted OMV project contract determines the version, target
set, integration capability state, adapter behavior, compatibility status, and
write plan.

```text
.omv/state + .omv/targets + .omv/integrations + capabilities + filesystem
  -> omv plan
  -> deterministic target/integration statuses and proposed writes
  -> omv sync or omv integrate apply applies the relevant plan
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
  │ load .omv config, state, targets, integrations, adapters      │
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
  │ .omv/adapters.toml     legacy AI/spec projection recovery     │
  │ .omv/integrations.toml selected providers, snapshots, status  │
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

Language-based target records remain compatible while execution moves behind
the adapter boundary:

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

Generalized target records use `kind`. Operators should choose the concrete
target kind they need; `schema_version` is internal compatibility metadata, not
a user-facing feature gate:

```toml
schema_version = 1

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

Kind-based implementation notes:

- Language-based target records remain compatible and continue to use
  `language`, `manifest_path`, and `runtime_export_path`.
- Kind-based records use `kind` plus typed kind-specific fields and may coexist
  with language-based records regardless of the file's `schema_version`.
- Unknown `kind` values do not block parsing known targets. The current binary
  reports those records as `unsupported`, does not execute them, and tells the
  operator to update OMV for that capability.
- `yaml-scalar` intentionally supports simple mapping scalar paths only. It
  rejects sequences, anchors, aliases, and block scalars until OMV adopts a
  fuller YAML round-trip parser.
- `cargo-workspace` supports `members = "all"` with exact workspace members
  and one-level `prefix/*` globs. Its lockfile update strategy is narrow and
  deterministic: OMV updates matching workspace package version lines in
  `Cargo.lock` and does not run `cargo update`.

## Platformized Host Integration Model

Host integration is distinct from target synchronization and from legacy
adapter projection. The forward product surface is provider/capability based:

- providers are host frameworks such as `codex` or `trellis`
- capabilities are medium-grained installable behaviors such as
  `project-instructions`, `host-skill`, `spec-guide`,
  `spec-index-snippet`, and `finalize-boundary`
- `.omv/integrations.toml` persists selected providers/capabilities plus the
  last provider-level detection snapshot and capability status
- `.omv/adapters.toml` remains legacy projection recovery metadata for
  compatibility commands
- `.omv/ai/*` remains generated canonical guidance; installed host files are
  derived projections and must not become authority

MVP provider support is intentionally small:

| Provider | Type | MVP behavior |
| --- | --- | --- |
| `codex` | agent | supported; may bootstrap lightweight instruction files |
| `trellis` | spec/workflow | supported; requires existing Trellis installation before mutation |
| `claude` | agent | future; hidden from init UI in MVP |
| `openspec` | spec | future; hidden from init UI in MVP |

Capability status is capability-granular:

- `selected`
- `pending`
- `installed`
- `failed`

Failures include both a stable machine reason code and a human-readable display
message. `omv integrate apply` is best-effort per selected capability: it
preserves successful installs, records failed capabilities, and returns
non-zero if any selected capability failed.

`omv integrate apply` must always re-detect the workspace before mutation and
must run targeted worktree-safety checks over only the files it would affect.
Codex can bootstrap lightweight instruction host files. Trellis and future
framework-style providers require an existing host installation before OMV
mutates framework files.

### Adapter Compatibility Transition

`omv integrate status` and `omv integrate apply` are the forward commands for
host provider workflows. Existing commands remain available during the MVP
transition:

- `omv adapter list`
- `omv adapter status`
- `omv adapter install`
- `omv adapter refresh`

Where behavior overlaps, legacy adapter commands should be wrappers or aliases
over the same projection/status helpers used by integration apply/status. They
must not grow a separate provider/capability model.

### Finalize Boundary Capability

`finalize-boundary` is a host integration capability, not a target adapter.
The first MVP boundary is Trellis finish-work:

- the host/agent supplies semantic `change_type`
- OMV supplies deterministic fields and invocation wiring
- missing `change_type` returns pending/manual-action rather than guessing
- helper identity is structured as provider + boundary name and flattened to
  the legacy finalize-task `source` string internally
- the helper delegates to the existing `omv event finalize-task` path
- idempotency is based on task identity, boundary identity, and a normalized
  workspace snapshot hash

The helper updates the active platform-resolved completion surface of each
*selected* in-scope agent through an OMV-managed block. It must not take over
every sibling command or make host files authoritative.

Trellis distributes per-agent copies of finish-work; each agent reads only its
own entrypoint. The `finalize-boundary` capability is owned by the Trellis
provider, but its target set is derived from the selected agent providers in
`.omv/integrations.toml`:

- claude   → `.claude/commands/trellis/finish-work.md`
- opencode → `.opencode/commands/trellis/finish-work.md`
- codex    → `.agents/skills/trellis-finish-work/SKILL.md` (v0.5) or
  `.agents/skills/finish-work/SKILL.md` (v0.4)

At `omv integrate apply` the block is upserted (idempotently) into every
selected agent's finish-work entrypoint that exists on disk. Unselected agents
are never touched. A selected agent with no finish-work surface produces an
actionable `finish-work-surface-missing` failure rather than being silently
skipped. `omv integrate status` reports `finalize-boundary` `installed` only
when every required (selected-agent + file-exists) entrypoint carries the block;
otherwise it reports a repairable pending/mismatch naming the offending path(s).
This is the one intentional cross-provider read (Trellis capability ← agent
selection). When no in-scope agent is selected, OMV preserves the legacy
codex-only `.agents/skills/...` behavior for backward compatibility.

Trellis finish-work path compatibility for the codex surface is capability-based,
not version-string based. OMV prefers `.agents/skills/trellis-finish-work/SKILL.md`
for Trellis 0.5+ and preserves `.agents/skills/finish-work/SKILL.md` for Trellis
0.4. If a project has both files but the OMV managed block exists only in the
legacy path, or when the block exists only in a Trellis-created `.backup` file,
`omv integrate status` reports a repairable mismatch because Trellis may now run
an active skill file without OMV guidance. Status must not migrate host files;
the recovery action is an explicit `omv integrate apply`.

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
- Compile `proto/omv/contract/versions/current/*.proto` for source builds.
- Keep numbered directories under `proto/omv/contract/versions/` as frozen API
  snapshots.
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
proto/omv/contract/versions/current/*.proto
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
changes, update `versions/current/*.proto`, freeze a new numbered snapshot,
update handwritten implementation and tests, then regenerate during build.

Stable/frozen API rules:

- `versions/current/contract.proto` is the editable latest contract.
- `versions/<n>/contract.proto` files are frozen snapshots.
- after a release or bootstrap, `current` must match the newest frozen
  snapshot.
- `src/contract/registry.rs::CONTRACT_VERSION` must match the newest frozen
  snapshot compiled into the binary.
- protobuf field tags and enum numeric values must not be reused; removed
  fields or enum values must be reserved.
- version 1 is the original language-native target contract.
- version 2 is the current runtime contract, including generalized target kinds
  and host integration capability metadata.

## Compatibility Domains

OMV should keep these contracts distinct:

- project schema version: `.omv/*.toml` persisted format
- target capability contract: supported target kinds and adapter options
- automation JSON contract: command output consumed by scripts and AI tools
- AI/spec adapter contract: generated `.omv/ai/*` and host projections
- host integration contract: `.omv/integrations.toml`, provider descriptors,
  capability statuses, and integration apply/failure semantics

Migration tooling should compare these domains and report whether a project is:

- current
- compatible but old
- missing capabilities
- using deprecated target records
- needing adapter refresh
- needing integration apply/retry
- using temporary adapter compatibility commands
- needing target migration

Stage 1 reports these statuses through `omv plan` diagnostics and summary
fields. It does not persist a `.omv/contracts/*` metadata directory yet; that
directory remains a future readable projection of the same registry data.

## Future Extension Seams

The first two stages and the host integration MVP do not implement external
SDKs, public plugin runtimes, or release triggers. The architecture still
reserves these seams:

- external target adapters can later map to the same plan operation contract
- CI and AI tools can consume plan JSON and capability metadata
- future integration providers can map to the same provider/capability state
  model after the internal registry is stable
- future release triggers can request the same core operations without owning
  target behavior
- migration analyzers can compare project-required capabilities with the
  installed OMV binary

## Staged Delivery

### Stage 1: Contract And Adapter Refactor

Goal: preserve existing behavior while replacing implicit language sync with
deterministic target planning and protobuf-backed capability contracts.

Subtasks:

1. Define OMV contract proto and stable/frozen snapshot rules.
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
