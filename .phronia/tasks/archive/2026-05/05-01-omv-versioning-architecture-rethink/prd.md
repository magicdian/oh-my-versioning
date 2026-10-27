# brainstorm: omv versioning architecture rethink

## Goal

Rethink OMV's product and architecture direction so it can support both local-first personal project versioning and more formal team release workflows, while staying extensible for complex projects like wiremux without hard-coding wiremux's repository shape.

## What I already know

* OMV was initially conceived as a personal project version-management tool.
* Personal projects may grow into team projects, so OMV needs a migration path from local developer-controlled version updates to team-controlled release governance.
* The user sees a meaningful difference between personal mode and team mode:
  * Personal mode can use formal team workflows, or a local developer mode where the developer updates versions when a change merits it.
  * Team mode likely centralizes version publication after multiple contributors merge changes, with possible triggers such as git merge or GitHub draft release.
* wiremux includes protobuf, ESP32, and a Rust workspace; an existing wiremux analysis suggests OMV would help, but OMV has missing capabilities.
* Missing capabilities should be generalized from wiremux needs instead of binding OMV to wiremux's exact structure.
* Future OMV versions will need migration support between old and new OMV schemas/capabilities.
* The user is considering a stable/frozen API-style mechanism, similar in spirit to wiremux proto definitions, so OMV can compare capability/API levels and identify which hooks or version files need rewrite or update.
* This is a major architectural rethink; the design should prefer a durable, extensible architecture over a minimal implementation.
* Current OMV code has `schema_version` on `.omv/config.toml`, `.omv/state.toml`, `.omv/targets.toml`, `.omv/adapters.toml`, and `.omv/finalizations.toml`, but there is no migration engine yet.
* Current OMV already has `project_profile = personal | oss`; this is an early profile concept, not enough for team release governance.
* Current OMV has an AI/spec adapter `CONTRACT_VERSION = 1` and `.omv/ai/contract.json`, but this contract is focused on agent guidance, not full OMV project capability/schema compatibility.
* Current OMV V1 target model is `language + root + manifest_path + runtime_export_path + strategy + enabled`; this cannot naturally model wiremux's docs, YAML, header macros, Cargo.lock, release notes, or check-only drift targets.
* Current `omv event finalize-task` provides an idempotent local automation hook with `.omv/finalizations.toml`, but it is task-completion oriented rather than team release-batch oriented.
* wiremux currently uses version `2604.30.3`, with tags from `2604.27.1` through `2604.30.3` observed in the public Git repository.
* wiremux root-level version surfaces include `VERSION`, README/README_CN badges, and release documentation.
* wiremux Rust host workspace has seven crates under `sources/host/wiremux`, each currently carrying the same package version, plus a generated `Cargo.lock`.
* wiremux ESP surface includes `sources/vendor/espressif/generic/components/esp-wiremux/idf_component.yml` and `ESP_WIREMUX_VERSION` in `esp_wiremux.h`.
* wiremux has a stable/frozen API pattern in `sources/api/proto/versions` and host enhanced API catalogs, with `current/` plus numbered frozen snapshots.

## Assumptions (temporary)

* This brainstorm will primarily produce architecture/product direction, PRD updates, and possibly code-spec changes before implementation.
* The immediate deliverable is not a narrow feature patch; it is a design-quality foundation for future implementation tasks.
* OMV should remain usable for solo local-first projects even if it gains team-release governance capabilities.
* The originally provided external project should be used only as private research input. OMV code and durable docs must not mention it by name; OMV remains a generic version-management tool.
* OMV's existing date-triplet version format can coexist with SemVer-style release intent; the architecture should separate version-number format from release decision policy.
* Team support should not require GitHub-only infrastructure; GitHub releases should be one host integration, not the core model.

## Open Questions

* Which architectural direction should OMV choose for personal-to-team evolution: modes, capabilities, policies, or another model?
* What is the right stable/frozen API abstraction for OMV: project API levels, integration capability contracts, migration manifests, or all of these separated?
* Which parts of the rethink belong in the next implementation slice, and which should remain as roadmap/spec work?

## Requirements (evolving)

* Analyze wiremux's OMV adoption needs from the provided local analysis file and repository.
* Research more formal version-management and release-governance approaches used by comparable tools or ecosystems.
* Define a general architecture that can support personal local workflows and team release workflows.
* Define a migration/versioned-contract model for OMV itself so old OMV projects can evolve safely.
* Avoid hard-coding wiremux-specific paths, languages, or release structure into OMV core concepts.
* Prefer a robust, extensible architecture over a minimal implementation mindset.
* Separate version truth, release intent, target sync, drift checks, host integrations, and OMV project capability/migration contracts.
* Preserve date-triplet versioning as a first-class output while allowing SemVer-like release-impact classification.
* Support check-only adoption for complex existing projects before OMV writes managed targets.
* Support team release batches where multiple completed changes are accumulated and one governed release event advances the version.
* Keep OMV's core product focus on personal projects and projects growing from personal to small-team workflows; do not optimize primarily for large-company release systems with bespoke governance.
* Treat CLI commands, AI framework injections, hooks, and host integrations as triggers only. The set of files to inspect or mutate must be determined by OMV's persisted project contract, not by the trigger.
* Make version propagation deterministic: given the same `.omv` state, target definitions, adapter capabilities, and filesystem input, OMV should produce the same write/check plan.
* Define target records with typed target kind, adapter kind/capability, match key or selector, expected replacement/rendering rule, and project-relative path.
* Support code and documentation version surfaces uniformly where possible, including Markdown, YAML, TOML, C headers, generated runtime exports, and package manifests.
* Define `plan` as the deterministic intermediate representation between OMV truth/config and mutation: triggers create or request a plan; adapters compute target status and proposed writes; sync applies the plan.
* Treat a future proto capability contract as OMV's stable machine-readable support matrix, for example target support enum values such as Cargo, CMake, Markdown, YAML, TOML, text, and C header macros.
* Use proto capability versions to identify when an existing `.omv` project uses unsupported, deprecated, or newly available target/interface features.
* Split delivery into two major stages:
  * Stage 1: refactor current OMV around proto-backed contracts, register existing capabilities as contract version 1, and migrate existing target sync to adapter-based deterministic planning.
  * Stage 2: add the target and version-management capabilities needed by wiremux, generalized for other complex projects.
* Save the resulting architecture design as a durable document under `docs/`, not only inside this task PRD.
* Include `omv plan` as a Stage 1 first-class command.
* Keep `.omv/targets.toml` V1 externally compatible in Stage 1. Design target schema V2 during Stage 1, but formally enable new V2 target kinds in Stage 2.
* Reserve architecture seams for future external SDK/plugin integration, but do not implement SDK/plugin runtime in either Stage 1 or Stage 2.
* Reserve architecture seams for future team release triggers such as GitHub CI/release flows, but do not implement related code or CI configuration in either Stage 1 or Stage 2.
* Do not mention the research case-study project name in OMV code or durable docs.
* Use proto codegen in Stage 1, do not commit generated code, and separate generated contract/stub types from handwritten implementation logic in a style similar to generated interface plus implementation layering.

## Acceptance Criteria (evolving)

* [x] PRD captures the product direction, requirements, trade-offs, and explicit out-of-scope areas.
* [x] Research notes summarize case-study needs and generalize them into OMV concepts.
* [x] Research notes summarize formal team version/release management patterns and map them to OMV.
* [x] Proposed architecture includes personal mode, team mode, migration between them, and future OMV schema/capability evolution.
* [x] Proposed implementation plan is decomposed into small PR-sized tasks.
* [x] Proposed target model can express at least Cargo workspace, root text file, YAML scalar, C header macro, markdown badge/managed block, and check-only drift targets.
* [x] Proposed release model can express local developer bump, task finalize bump, merge/release-batch bump, GitHub draft/release integration, and no-op changes.
* [x] Proposed migration model can report which `.omv` files, host adapters, target definitions, and hooks need rewrite or refresh.
* [x] Architecture document is added under `docs/` with the refactored architecture diagram, staged delivery plan, target/adapter model, proto contract role, and open decisions.

## Definition of Done (team quality bar)

* Tests added/updated where implementation changes are made.
* Lint / typecheck / CI green where implementation changes are made.
* Docs/specs updated for product or architecture decisions.
* Rollout/rollback and migration behavior considered for risky changes.

## Out of Scope (explicit)

* Hard-coding wiremux repository structure into OMV.
* Mentioning the research case-study project name in OMV code or durable docs.
* Implementing all future team governance features in one pass.
* Replacing existing language/package-native versioning tools; OMV should orchestrate and synchronize rather than erase native manifests.
* Building a public external SDK or third-party plugin ecosystem in the first implementation stage.
* Implementing GitHub CI/release trigger code or CI workflow files in the first two stages.

## Technical Notes

* User-provided wiremux repository: https://github.com/magicdian/wiremux
* User-provided local analysis file: `/Users/magicdian/Downloads/wiremux接入omv.txt`
* Current OMV specs already describe a local-first Rust CLI with `.omv/` as the source of truth and language-native manifests as derived outputs.
* wiremux was cloned for read-only research at `/private/tmp/wiremux-omv-research`.
* OMV current implementation files inspected:
  * `README.md`
  * `src/core/schema.rs`
  * `src/core/target/mod.rs`
  * `src/storage/targets.rs`
  * `src/sync/mod.rs`
  * `src/sync/rust.rs`
  * `src/core/finalization.rs`
  * `src/app/mod.rs`
  * `src/adapter.rs`
  * `tests/integration/target_sync.rs`
  * `.trellis/spec/backend/database-guidelines.md`
  * `.trellis/spec/backend/directory-structure.md`
  * `.trellis/spec/backend/quality-guidelines.md`
  * `.trellis/spec/guides/cross-layer-thinking-guide.md`
* Durable architecture document added: `docs/OMV_CONTRACT_ARCHITECTURE.md`

## Research Notes

### wiremux needs generalized into OMV concepts

* Multi-surface version truth: root `VERSION`, badges, docs, Rust crates, Cargo.lock, ESP-IDF YAML, C header macros, generated package metadata, and release docs need one authoritative version.
* Multi-package same-version policy: wiremux's Rust workspace needs a native `cargo-workspace` style target with member selection, same-version policy, and lockfile refresh/check behavior.
* Non-manifest targets: README badges, managed markdown blocks, YAML scalars, and C macros should be first-class target kinds, not forced through language adapters.
* Check-only adoption: existing projects need `omv sync --check` or equivalent drift reporting before OMV writes files.
* API/capability snapshots: wiremux's `current/` plus numbered frozen snapshots are not just version strings; they are compatibility contracts that tooling can compare.
* AI workflow safety: agent/spec integrations should say exactly which OMV commands are authoritative and which manual edits are forbidden, and those rules must refresh as OMV contracts evolve.

### What similar tools and ecosystems do

* SemVer ties version increments to a declared public API: major for incompatible API changes, minor for compatible functionality, patch for compatible fixes. It also says released version contents should not be modified.
* Conventional Commits gives commit messages machine-readable release intent: `fix` maps to patch, `feat` maps to minor, and breaking changes map to major.
* semantic-release automates version calculation, release notes, and publishing from CI/merge signals, reducing direct human version-number editing.
* Changesets is closer to a team/monorepo model: contributors record release intent during development, then a release step combines many change intents into package version/changelog updates.
* GitHub Releases are tag-based deployable software iterations; draft releases are a formal staging point before publication and can be managed by UI, CLI, or API.
* Release Drafter updates draft release notes as PRs merge, which is a strong precedent for team mode accumulating release intent before publishing.
* Protobuf compatibility practice emphasizes not reusing field/tag numbers and reserving deleted fields; Buf generalizes this into mechanical breaking-change detection against a previous schema/source with selectable strictness levels.

Source links:

* https://semver.org/
* https://www.conventionalcommits.org/en/v1.0.0/
* https://semantic-release.gitbook.io/semantic-release
* https://github.com/changesets/changesets/blob/main/docs/intro-to-using-changesets.md
* https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository
* https://github.com/apps/release-drafter
* https://protobuf.dev/best-practices/dos-donts/
* https://buf.build/docs/breaking/
* https://protobuf.dev/reference/rust/building-rust-protos/
* https://docs.rs/prost/latest/prost/
* https://buf.build/docs/generate/

### Constraints from OMV

* `.omv/state.toml` should remain the version truth; native files and host guidance remain derived outputs.
* Current target storage is hand-parsed TOML with a V1 flat shape; a V2 target model will require a migration path rather than a silent shape change.
* Current structured JSON envelope has `contract_version = "1"`; automation compatibility must be versioned separately from human CLI copy.
* Current adapter contract version only covers `.omv/ai/*` projection; it should not be overloaded to mean project schema version, target capability version, or release policy version.
* Current finalization hook has useful idempotency mechanics, but team release batches need a distinct release-intent ledger rather than overloading task finalization records.
* Current Rust crate has no `build.rs`, no protobuf/prost/tonic dependencies, and no code generation step. Introducing proto codegen would be a real build-chain decision, but it can be justified if generated types become the single source for OMV capability, migration, and plan/result contracts.
* Rust protobuf options include official protobuf Rust codegen and `prost`; `prost` is idiomatic and commonly used in Cargo projects, but `prost-build` requires `protoc` unless configured otherwise.
* Buf can generate code from `.proto` files and run breaking-change checks. It could be useful for OMV's own contract evolution, but adding Buf as a required build tool may be too heavy for OMV's personal-project core.

### Feasible approaches here

**Approach A: Profile-first modes**

* How it works: keep `project_profile` central and expand it from `personal | oss` to `personal | team`; commands branch behavior by mode.
* Pros: easy mental model and easy CLI prompts.
* Cons: too coarse; personal projects may need team-style release gates, and team projects may still need local check-only flows. Migration becomes mode-switching instead of capability evolution.

**Approach B: Policy and capability graph** (Recommended)

* How it works: model OMV as independent policy/capability domains: version scheme, release governance, target capabilities, host integrations, and migration compatibility. Profiles become presets over capabilities, not hard modes.
* Pros: supports personal-to-team migration naturally, lets wiremux-like needs land as generalized target/capability additions, and avoids GitHub or wiremux-specific coupling.
* Cons: requires stronger schema design and migration reporting up front.

**Approach C: Release-pipeline first**

* How it works: prioritize team release mechanics: release intents, merge aggregation, draft release integration, and CI hooks; target model improvements are added as needed.
* Pros: directly addresses team governance and formal release workflows.
* Cons: risks leaving sync coverage incomplete, so projects like wiremux still cannot trust `omv sync` end to end.

### Recommended architecture direction

Use Approach B as the core architecture, with profiles as presets:

* `profile = personal-local`: default local developer version truth and manual/local bump.
* `profile = personal-governed`: solo project using release intent and check gates before bump.
* `profile = team-governed`: release intent is accumulated from tasks/PRs/merges and published through explicit release events.

These profiles should configure policies rather than own behavior directly:

* `version_policy`: date-triplet, semver, or future strategies.
* `release_policy`: local bump, task finalize, changeset-like release intent, GitHub draft/release, or CI-only.
* `target_policy`: write, check-only, required, optional, generated-only, lockfile refresh behavior.
* `compatibility_policy`: OMV project schema version, automation JSON contract version, AI adapter contract version, target capability set, and migration status.

## Decision (ADR-lite)

**Context**: OMV needs to support deterministic version propagation across code, manifests, docs, generated files, and AI/spec guidance. The user's priority is still personal projects, with an upgrade path into small-team governance. Large organizations will likely have existing release systems, so OMV should not become a heavyweight enterprise release platform.

**Decision**: Choose the policy/capability graph architecture. Profiles are presets over independent policies, not hard-coded modes. CLI, AI, Git hooks, Trellis finish-work, and future GitHub integrations are triggers. The authoritative behavior comes from `.omv` project contracts: version truth, target definitions, adapter capabilities, release policy, and compatibility/migration metadata.

**Consequences**:

* OMV can stay local-first and personal-project friendly while still supporting governed workflows.
* A deterministic write/check plan becomes central: target sync should be explainable before it mutates files.
* Target adapters become a primary extension point. Different file formats need different adapters, but all adapters should share the same version truth and plan/result model.
* Proto can be useful for stable machine-readable contracts and generated adapter/capability types, but TOML should remain the human-editable project configuration surface.
* OMV should separate at least four versioned contracts:
  * `.omv` project schema version for persisted files.
  * target capability contract for supported target kinds and adapter options.
  * automation JSON contract for command output.
  * AI/spec adapter contract for generated `.omv/ai/*` and host projections.
* Migration tooling should compare these contracts and report which `.omv` files, targets, hooks, host adapters, or generated guidance need refresh.

### Deterministic target contract sketch

Human-editable TOML can define project-specific target instances:

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
id = "esp-idf-component"
kind = "yaml-scalar"
adapter = "yaml"
path = "sources/vendor/espressif/generic/components/esp-wiremux/idf_component.yml"
key = "version"
template = "{version}"
mode = "write"

[[targets]]
id = "esp-header-version"
kind = "c-header-macro"
adapter = "c-header"
path = "sources/vendor/espressif/generic/components/esp-wiremux/include/esp_wiremux.h"
macro = "ESP_WIREMUX_VERSION"
template = "\"{version}\""
mode = "write"

[[targets]]
id = "wiremux-rust-workspace"
kind = "cargo-workspace"
adapter = "cargo"
root = "sources/host/wiremux"
members = "all"
version_policy = "same"
lockfile = "update"
mode = "write"
```

A stable machine-readable contract can define supported target kinds, adapter option schemas, plan/result payloads, and compatibility levels. This could be protobuf if OMV needs generated types and stable API comparison, but the project author should still mostly edit TOML.

Example capability-contract sketch:

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

Decision: introduce proto codegen in Stage 1. Generated code is not manually edited and does not need to be committed to the repository. `protoc` is an OMV developer/source-build dependency, not an end-user runtime dependency. End users are expected to install released OMV binaries from GitHub Releases and will not need protobuf tooling.

Implementation policy:

* Use `build.rs` plus Rust protobuf code generation, likely `prost-build`, unless a better Rust protobuf generator is selected during implementation.
* Do not commit generated Rust code.
* Document the required `protoc` version or supported version range for OMV contributors.
* Pin or install a known-good `protoc` version in CI so generated code and contract checks are reproducible.
* Treat generated code as build output. Review and edit `.proto` files, Rust adapter logic, and contract tests instead.
* Do not add codegen only as decorative documentation. Generated types should actively drive capability registry construction, migration checks, plan/result contracts, and structured output tests.

The plan model should be explicit:

```text
.omv/state + .omv/targets + adapter capability registry + filesystem
  -> omv plan/sync --check
  -> deterministic list of target statuses and proposed writes
  -> omv sync applies the same plan atomically per target where possible
```

### External SDK and plugin scenarios

OMV was not originally designed as an SDK or plugin host. That is acceptable for the near term. However, the proto/capability contract can keep the door open for valuable future extension points without implementing them immediately.

Potential valuable scenarios:

* Project-local custom target adapter: a project defines a target kind that OMV core does not know yet, such as a vendor-specific manifest or generated release file. A plugin can return plan operations instead of directly mutating files.
* Ecosystem-specific adapters outside OMV core: Cargo, npm, Maven, ESP-IDF, CMake, Helm, Docker labels, or VS Code extension manifests could grow independently without bloating OMV core.
* CI and AI tool integration: external agents can read OMV's capability/plan contract and know whether their installed OMV binary supports required target kinds before making changes.
* Migration analyzers: a tool can compare a project's required capability version with the installed OMV support matrix and report which targets, hooks, or adapter projections need upgrade.
* Read-only inspection tools: IDE extensions, GitHub Actions, or dashboards can render OMV target drift without embedding OMV internals.

Near-term stance:

* Do not build a public plugin runtime yet.
* Design adapters inside OMV core around a trait boundary that would later map cleanly to plugins.
* Define plan input/output and capability IDs in stable proto so an external SDK can be added later without changing core concepts.

### Two-stage delivery plan

#### Stage 1: Contract and adapter refactor

Goal: preserve existing behavior while replacing implicit language sync with deterministic target planning and proto-backed capability contracts.

Subtasks:

1. Define OMV contract proto v1
   * Include capability IDs for current target support: Rust/Cargo package, Python manifest, Go module, Java/Maven, C/CMake, runtime export, AI/spec adapter projection, structured JSON envelope.
   * Include command/interface support IDs for `current`, `bump`, `sync`, `adapter`, and `event finalize-task`.
   * Include plan/result message shapes or at least stable IDs for them.
   * Decide whether Stage 1 uses hand-written Rust mirrors or introduces codegen.

2. Add contract registry and capability reporting
   * Add an internal capability registry that reports what the current OMV binary supports.
   * Generate or expose `.omv/ai/contract.json` from the same registry rather than ad hoc constants.
   * Keep existing JSON envelope contract compatible.

3. Introduce target adapter trait and plan model
   * Add `TargetAdapter` / `TargetPlanner` boundary.
   * Each adapter computes target status and proposed write operations from `.omv/state.toml`, target config, and current filesystem content.
   * Plan results should include target id, adapter, path(s), current value, expected value, status, and diagnostics.

4. Migrate existing V1 targets into adapter-backed execution
   * Keep `.omv/targets.toml` V1 loading compatible.
   * Map existing `language + manifest_path + runtime_export_path` records to current adapters.
   * Ensure existing `omv sync` and `omv bump` behavior remains compatible.

5. Add `omv plan` and/or `omv sync --check`
   * `omv plan` is a first-class Stage 1 command and produces deterministic dry-run output.
   * `sync --check` exits non-zero when required targets drift.
   * JSON output becomes the primary automation contract.

6. Add migration/status reporting
   * Detect old `.omv` schema and target capabilities.
   * Report whether the project is current, compatible, needs adapter refresh, or needs target migration.
   * Do not force project files into a new shape until migration apply is explicitly introduced.
   * Design target schema V2, but keep V1 target files externally compatible in Stage 1.

7. Update docs/specs/tests
   * Update backend persistence, target, adapter, structured JSON, and cross-layer specs.
   * Add tests proving old V1 targets still sync and new plan output is deterministic.
   * Add a durable `docs/` architecture document that uses generic examples only.

#### Stage 2: wiremux-needed generalized capabilities

Goal: add generalized target kinds and workflows identified from the research case study, without naming or hard-coding any specific external project.

Subtasks:

1. Add generic text target
   * Whole-file scalar target for files like `VERSION`.
   * Regex replace target for badges and simple inline text.

2. Add Markdown target
   * Badge replacement.
   * Managed block replacement for release docs or generated snippets.

3. Add YAML scalar target
   * Update keys such as `version` in `idf_component.yml`.
   * Preserve formatting enough for common simple files; define limits explicitly.

4. Add C header macro target
   * Update string/numeric `#define` macros such as `ESP_WIREMUX_VERSION`.
   * Detect missing, duplicated, or malformed macros.

5. Add Cargo workspace target
   * Support same-version policy across workspace members.
   * Optionally suggest or support `[workspace.package] version`.
   * Support `Cargo.lock` check/update strategy.

6. Add CMake/package metadata improvements if needed
   * Rework current C-family behavior into explicit CMake target and header-export target rather than one broad C-family adapter.

7. Add wiremux adoption sample
7. Add complex-project adoption sample
   * Provide a generalized `.omv/targets.toml` example that covers a representative complex project with docs, Cargo workspace, YAML, header macros, and generated files.
   * Keep it as a sample/case study, not baked into OMV.

8. Strengthen AI/Trellis enforcement
   * Host guidance should say `omv plan` / `omv sync --check` before release-sensitive changes.
   * `$finish-work` integration should fail or warn when OMV target drift exists.

9. Add release intent only after target coverage is trustworthy
   * Local personal release remains primary.
   * Team-style release batching can build on the same deterministic target plan later.
   * GitHub CI/release triggers remain architectural extension points only in these two stages.

### Generated contract layering

Proto-generated code should be treated as an interface/stub layer, not as the place where OMV business logic lives.

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
handwritten Rust impl layer
  - capability registry construction
  - target adapter dispatch
  - plan engine
  - migration checks
  - JSON/text rendering
```

Rules:

* Generated code is never manually edited.
* Generated code is not committed.
* Handwritten code wraps or maps generated types into ergonomic internal domain types where needed.
* If generated contracts need behavior changes, edit `.proto` plus handwritten impl/tests, then regenerate.
* This mirrors the generated-interface plus implementation separation used by systems such as Android AIDL, without requiring OMV to adopt AIDL itself.

### Refactored architecture sketch

```text
                       Trigger Layer
  ┌──────────────────────────────────────────────────────────────┐
  │ CLI: current/bump/plan/sync/check/migrate                    │
  │ AI/Trellis/Codex hooks                                       │
  │ Future: Git hook / GitHub release / CI                       │
  └──────────────────────────────┬───────────────────────────────┘
                                 │ requests operation
                                 ▼
                         App Orchestration
  ┌──────────────────────────────────────────────────────────────┐
  │ load .omv config/state/targets/adapters/contracts            │
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
  │ input: version truth + target TOML + adapter capabilities     │
  │ read current filesystem                                      │
  │ compute deterministic target statuses and proposed writes     │
  │ output: plan JSON / diagnostics / sync operations             │
  └──────────────────────────────┬───────────────────────────────┘
                                 │ dispatch by adapter/kind
                                 ▼
                         Target Adapters
  ┌────────────┬───────────┬───────────┬────────────┬────────────┐
  │ Cargo      │ CMake     │ Markdown  │ YAML       │ C Header   │
  │ workspace  │ manifest  │ badge/blk │ scalar     │ macro      │
  ├────────────┼───────────┼───────────┼────────────┼────────────┤
  │ Text       │ TOML      │ Runtime exports        │ Future...  │
  └────────────┴───────────┴───────────┴────────────┴────────────┘
                                 │
                  plan only      │      apply writes
                  ┌──────────────┴──────────────┐
                  ▼                             ▼
           Check/Drift Output              Sync Executor
  ┌────────────────────────────┐   ┌──────────────────────────────┐
  │ no mutation                 │   │ atomic write where possible   │
  │ CI/AI/review friendly       │   │ update derived files          │
  │ non-zero on required drift  │   │ record summary                │
  └────────────────────────────┘   └──────────────────────────────┘

                Persisted Project Surface
  ┌──────────────────────────────────────────────────────────────┐
  │ .omv/config.toml       profile/policies                      │
  │ .omv/state.toml        version truth                         │
  │ .omv/targets.toml      human-editable target instances        │
  │ .omv/adapters.toml     installed AI/spec host projections     │
  │ .omv/contracts/*       generated/readable capability metadata │
  │ .omv/ai/*              generated agent/spec guidance          │
  └──────────────────────────────────────────────────────────────┘
```

### Expansion sweep

Future evolution:

* OMV may become a release-governance layer, not just a version-number writer.
* OMV may need pluggable host integrations for GitHub, Trellis, Codex, OpenSpec, Cargo, ESP-IDF, and generic file formats.

Related scenarios:

* `omv init`, `omv sync`, `omv sync --check`, `omv bump`, `omv event finalize-task`, and future release commands must share the same target registry and drift engine.
* Target discovery and migration should be able to suggest config changes for existing projects without writing them immediately.

Failure and edge cases:

* Manual edits can create drift; check mode must report exact target id, expected value, observed value, and write plan.
* Team release triggers can replay or race; release events need idempotency like finalize-task fingerprints.
* Migration can partially update `.omv` and host adapters; migration planning should be dry-run first and atomic per file when applied.

### Candidate PR decomposition

* PR1: Docs/spec architecture alignment for capability-based OMV direction, target V2 concepts, release governance, and migration contracts.
* PR2: Add `omv check` / `omv sync --check` drift engine and target status JSON without changing existing write behavior.
* PR3: Introduce target V2 schema and migration plan/reporting, initially supporting V1 target compatibility.
* PR4: Add generic target kinds: text replace, markdown managed block, YAML scalar, C header macro.
* PR5: Add Cargo workspace target with same-version policy and lockfile check/update strategy.
* PR6: Add release-intent ledger and team-governed release event model.
* PR7: Add host integrations such as GitHub draft release/release notes and Trellis finish-work enforcement on top of the release-intent model.
