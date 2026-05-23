# Backend Development Guidelines

> Backend code-specs for the `omv` Rust CLI runtime.

---

## Overview

`omv` is a local-first Rust CLI. In V1 there is no database server, queue, or
remote control plane. The backend is responsible for:

- version calculation from date/time plus `BuildNumber`
- `.omv/` file loading, validation, and atomic persistence
- NTP-backed time validation without changing system time
- target synchronization into language-native manifests and runtime exports
- adapter projection from `.omv/ai/*` into agent/spec host frameworks
- platformized host integration state, provider capabilities, and safe
  plan/apply orchestration
- structured JSON output for automation-safe reads and writes
- localized CLI/TUI output driven by shared i18n catalogs

These docs are bootstrapped from the current product definition and should be
treated as executable implementation contracts until the codebase grows enough
to provide stronger examples.

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | Rust module boundaries for core logic, persistence, sync, adapter projection, and automation output | Bootstrapped |
| [Database Guidelines](./database-guidelines.md) | Persistent state contracts for `.omv/*.toml` and generated OMV AI artifacts | Bootstrapped |
| [Error Handling](./error-handling.md) | Error taxonomy, exit behavior, structured JSON failures, and validation flow | Bootstrapped |
| [Logging Guidelines](./logging-guidelines.md) | Structured logging and secret-safe observability rules | Bootstrapped |
| [Localization Guidelines](./localization-guidelines.md) | i18n contracts for CLI and TUI output | Bootstrapped |
| [Quality Guidelines](./quality-guidelines.md) | Testing bar, forbidden patterns, and review checklist | Bootstrapped |

## Pre-Development Checklist

Before writing backend code for `omv`, read:

1. [Directory Structure](./directory-structure.md)
2. [Database Guidelines](./database-guidelines.md)
3. [Error Handling](./error-handling.md)
4. [Localization Guidelines](./localization-guidelines.md) when changing any
   user-facing copy or locale behavior
5. [Quality Guidelines](./quality-guidelines.md)

Also read:

- [Cross-Layer Thinking Guide](../guides/cross-layer-thinking-guide.md) for
  version-flow, sync, adapter-projection, or i18n changes that cross
  CLI/TUI/storage boundaries
- [Menuconfig Style Matrix](/Users/magicdian/Documents/personal_project/oh-my-versioning/docs/matrix/MENUCONFIG_STYLE_MATRIX.md)
  when changing `omv init` or future menuconfig-style flows

## Current Backend Design Decisions

- `.omv/` is the only source of truth; language-native manifests are derived
  outputs.
- V1 stores configuration, mutable state, and targets in separate TOML files.
- V1 stores legacy adapter projection recovery state in `.omv/adapters.toml`.
- MVP host integration selection, detection snapshots, capability status, and
  failure recovery live in `.omv/integrations.toml`.
- `.omv/ai/*` is the canonical generated guidance surface projected into
  agent/spec hosts; installed host files are derived outputs.
- `omv integrate status/apply` is the forward command family for provider and
  capability workflows. Existing `omv adapter install/refresh/list/status`
  commands remain temporary MVP compatibility commands where behavior overlaps.
- `.omv/targets.toml` uses a flat target list in V1.
- MVP integration providers are internal registry entries, not a public plugin
  runtime. Codex, OpenCode, and Trellis are the supported MVP providers; Claude
  and OpenSpec remain outside the init UI support matrix.
- `ProjectInstructions` managed blocks in shared host files (e.g. `AGENTS.md`)
  use a provider-agnostic block identifier (`integration-project-instructions`)
  so multiple agent hosts sharing the same host file do not produce duplicate
  content. Each capability's block is keyed by `integration-{capability}` for
  shared capabilities, and `integration-{provider}-{capability}` for
  provider-specific ones.
- Trellis version is detected via `.trellis/.version` (semver string).
  `detect_trellis_version()` returns `TrellisVersionInfo { version, is_v05_or_later }`.
  v0.4.x (pre-skill-architecture) and v0.5.x+ (skill-first) share the same
  `FinalizeBoundary` capability but differ in when the host workflow triggers
  it.
- The OMV `finalize-boundary` helper should be called during Phase 3.4 commit
  confirmation (when the user confirms a commit), not deferred to
  `/trellis:finish-work`. This ensures each completed unit of work produces a
  distinct version bump; the AI instructions in `.omv/ai/adapters/trellis/guide.md`
  and `.omv/ai/adapters/project-instructions.md` describe this convention for
  v0.5+ projects. v0.4 `/finish-work` may auto-trigger the helper as part of
  its own workflow.
- i18n is mandatory for CLI and init TUI from the first implementation.
- machine-readable output uses a shared JSON envelope across supported commands.
- NTP time is advisory for `omv` logic only and must never mutate the system
  clock.

## Bootstrap Sources

These specs were aligned from:

- [OMV CLI Foundation PRD](/Users/magicdian/Documents/personal_project/oh-my-versioning/.trellis/tasks/04-13-omv-cli-foundation/prd.md)
- [Menuconfig Style Matrix](/Users/magicdian/Documents/personal_project/oh-my-versioning/docs/matrix/MENUCONFIG_STYLE_MATRIX.md)
- the referenced `bridgingio` i18n pattern the user asked us to follow

---

**Language**: Keep backend specs in English.
