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
- V1 also stores adapter installation state in `.omv/adapters.toml`.
- `.omv/ai/*` is the canonical integration surface projected into agent/spec
  hosts.
- `.omv/targets.toml` uses a flat target list in V1.
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
