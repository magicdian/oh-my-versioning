# Frontend Development Guidelines

> Operator-facing UI guidelines for `omv`.

---

## Overview

`omv` does not have a browser frontend. In this project, "frontend" means the
operator-facing CLI/TUI experience:

- `ratatui` screens
- menuconfig row and popup behavior
- keyboard interactions
- localized rendering and status copy
- UI draft state before `.omv` persistence

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | TUI module layout and screen/widget separation | Bootstrapped |
| [Component Guidelines](./component-guidelines.md) | Screen, widget, row-template, and popup rules | Bootstrapped |
| [Hook Guidelines](./hook-guidelines.md) | Reusable interaction helpers and event controllers for Rust TUI | Bootstrapped |
| [State Management](./state-management.md) | UI draft state vs persisted `.omv` state | Bootstrapped |
| [Quality Guidelines](./quality-guidelines.md) | UX correctness and interaction testing | Bootstrapped |
| [Type Safety](./type-safety.md) | Typed UI state, menu actions, and locale-safe rendering | Bootstrapped |

## Pre-Development Checklist

Before changing `omv init` or any future TUI screen, read:

1. [Component Guidelines](./component-guidelines.md)
2. [State Management](./state-management.md)
3. [Type Safety](./type-safety.md)
4. [Quality Guidelines](./quality-guidelines.md)

Also read:

- [Menuconfig Style Matrix](/Users/magicdian/Documents/personal_project/oh-my-versioning/docs/matrix/MENUCONFIG_STYLE_MATRIX.md)
- [Backend Localization Guidelines](../backend/localization-guidelines.md)

## Current Frontend Design Decisions

- V1 operator UI is a `ratatui` menuconfig-style flow for `omv init`
- `Space` toggles semantics; `Enter` follows `--->`; `Esc` closes popup, backs
  out, then exits from root
- all UI copy is localized and catalog-driven
- render code must not own business logic such as version calculation or target
  synchronization

---

**Language**: Keep frontend specs in English.
