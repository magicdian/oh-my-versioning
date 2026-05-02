# Stage 1: Contract and Adapter Refactor

## Goal

Refactor OMV so current behavior is preserved while target synchronization is driven by a deterministic plan engine, protobuf-backed capability contracts, generated Rust contract types, and adapter-backed execution. Stage 1 must keep existing `.omv/targets.toml` V1 projects compatible.

## Parent Context

Parent task: `.trellis/tasks/05-01-omv-versioning-architecture-rethink`

Architecture document: `docs/OMV_CONTRACT_ARCHITECTURE.md`

## Requirements

* Add protobuf contract definitions under `proto/omv/contract/v1/`.
* Introduce build-time protobuf code generation in Stage 1.
* Do not commit generated Rust code.
* Treat generated code as a stub/contract layer; handwritten Rust owns business logic.
* Add a capability registry backed by generated contract types.
* Register existing Stage 1 capabilities as contract version 1:
  * current V1 target families: Rust/Cargo package, Python manifest, Go module, Java/Maven, C/CMake, runtime export
  * command/interface support: `current`, `bump`, `sync`, `adapter`, `event finalize-task`, `plan`
  * structured JSON envelope compatibility
  * AI/spec adapter projection compatibility
* Add a deterministic plan model:
  * input: `.omv/state.toml`, `.omv/targets.toml`, adapter capability registry, filesystem
  * output: target statuses and proposed operations
  * statuses include at least `ok`, `drift`, `missing`, `unsupported`, `error`, `skipped`
* Add a target adapter trait/boundary. Existing language sync must run through this boundary.
* Keep V1 target file shape externally compatible.
* Add `omv plan` as a first-class CLI command.
* Add `omv sync --check` using the same plan engine.
* `omv plan` and `omv sync --check` must not mutate files.
* Preserve current `omv bump` and `omv sync` user-visible behavior where possible.
* Add migration/status reporting sufficient to identify:
  * current project
  * compatible old project
  * missing capability
  * adapter refresh needed
  * target migration needed
* Update docs/specs/tests to reflect the new contract, plan, and adapter model.

## Non-Goals

* Do not implement Stage 2 V2 target kinds in this task.
* Do not implement public plugin runtime or external SDK support.
* Do not implement GitHub CI/release trigger code or workflow files.
* Do not require end users of released binaries to install protobuf tooling.
* Do not hard-code any external case-study repository structure or name.

## Implementation Steps

1. Prepare protobuf build chain
   * Add `build.rs`.
   * Add build dependencies for Rust protobuf generation, likely `prost-build`.
   * Add runtime dependency only if generated types require it.
   * Document local developer prerequisite for `protoc` in docs or README.
   * Ensure generated files live under `OUT_DIR` and are not committed.

2. Define contract proto v1
   * Create `proto/omv/contract/v1/contract.proto`.
   * Define target support enum values for existing capability families.
   * Define command/interface support enum values.
   * Define plan status enum.
   * Define plan target result and plan summary messages.
   * Reserve field numbers or enum values where useful for future evolution.

3. Add generated contract module boundary
   * Include generated code from `OUT_DIR`.
   * Keep generated module isolated, for example `src/contract/generated.rs` or `src/contract/mod.rs`.
   * Add handwritten wrapper/domain mapping where generated types are awkward.

4. Add capability registry
   * Implement a registry that reports supported contract version, target support, command support, and JSON contract support.
   * Replace or align hard-coded adapter contract constants with registry-backed values where practical.
   * Expose capability metadata internally for plan and migration checks.

5. Add plan domain model
   * Define handwritten internal plan types, mapping to/from generated contract types as needed.
   * Include target id, adapter/kind, paths, current value summary, expected value summary, status, operation summary, and diagnostics.
   * Ensure plan output avoids dumping full file contents by default.

6. Add target adapter boundary
   * Introduce trait(s) for planning and applying target operations.
   * Existing language adapters should compute planned changes before writing.
   * Existing write logic should be reused where possible but invoked through the new apply path.

7. Migrate existing V1 target execution
   * V1 `language + manifest_path + runtime_export_path` targets must still load and save.
   * Map Rust/Python/Go/Java/C-family V1 records to adapter-backed planners.
   * Ensure `omv sync` and `omv bump` still sync manifests/runtime exports and `.omv/skills`.

8. Add `omv plan`
   * Extend CLI parser.
   * Add localized help text and i18n keys.
   * Add text output for humans.
   * Add JSON output using existing structured envelope conventions.

9. Add `omv sync --check`
   * Extend CLI parser for sync check mode.
   * Use the same plan engine.
   * Do not mutate files.
   * Exit non-zero when required targets drift, are missing, unsupported, or error.
   * Return structured JSON with plan details.

10. Add migration/status reporting
   * Add minimal internal checks for project schema/capability compatibility.
   * If implemented as a command, keep it narrow; otherwise expose through `plan` diagnostics.
   * Report but do not apply V2 target migrations.

11. Update generated AI/spec guidance
   * Mention `omv plan` and `omv sync --check` in canonical `.omv/ai/*` guidance where appropriate.
   * Keep host adapter projections thin.

12. Update specs and durable docs
   * Update backend directory, persistence, error handling, quality, and cross-layer specs as needed.
   * Update `docs/OMV_CONTRACT_ARCHITECTURE.md` if implementation decisions differ from the current plan.

13. Tests and verification
   * Unit tests for capability registry.
   * Unit tests for plan status computation.
   * Integration tests proving existing V1 target sync still works.
   * Integration tests for `omv plan --json`.
   * Integration tests for `omv sync --check` success and drift failure.
   * Tests for generated AI/spec guidance updates.
   * Run `cargo fmt --check`, `cargo test --all-targets --all-features`, and `cargo clippy --all-targets --all-features -- -D warnings` if available.

## Expected Result

* OMV has a protobuf-backed contract layer with build-time generated Rust types.
* Generated code is not committed and is not manually edited.
* Current V1 target projects still work.
* `omv plan` can explain planned target changes before mutation.
* `omv sync --check` can be used by CI/AI/humans to detect drift without writing files.
* Existing `omv bump` and `omv sync` behavior remains compatible.
* Architecture docs/specs reflect the new contract and adapter model.

## Acceptance Criteria

* [ ] `proto/omv/contract/v1/contract.proto` exists and defines Stage 1 contract types.
* [ ] `build.rs` generates Rust contract code during build.
* [ ] Generated Rust code is not committed.
* [ ] Capability registry is backed by generated contract types or explicit generated-type mappings.
* [ ] Existing V1 targets still round-trip through storage.
* [ ] Existing V1 target sync passes through adapter-backed planning/apply flow.
* [ ] `omv plan` supports text and JSON output.
* [ ] `omv sync --check` supports text and JSON output and does not mutate files.
* [ ] Drift check has a non-zero failure path.
* [ ] `.omv/ai/*` guidance includes the new check/plan flow.
* [ ] Backend specs and `docs/OMV_CONTRACT_ARCHITECTURE.md` are updated if behavior differs from this PRD.
* [ ] Format, tests, and clippy pass or documented blockers are reported.

## Review Focus

* V1 compatibility must not regress.
* The plan engine must be deterministic and shared by `plan`, `sync --check`, and write sync.
* Generated contract types must not absorb business logic.
* Structured JSON output must remain stable and automation-safe.
* User-facing copy must be localized.
