# Type Safety

> Type-safety patterns for `omv` operator-facing code.

---

## Overview

UI code must use the same typed domain model as backend code wherever possible.
Avoid raw strings for locale, language, build policy, output mode, and popup
strategy.

## Type Organization

Use shared enums/structs for:

- `OperatorLocale`
- `BuildPolicy`
- `VersionOutput`
- `TargetLanguage`
- `PreProjectStrategy`
- `IntegrationProviderId`
- `IntegrationCapabilityId`
- `IntegrationCapabilityStatus`
- `RowTemplate`
- `PopupKind`

Keep these in shared modules rather than redefining them in each screen.

## Validation

- deserialize persisted TOML into typed enums/structs
- validate auto-discovery results before converting them into draft rows
- convert unsupported/raw input into typed validation errors before rendering
- convert provider/capability records from backend integration descriptors into
  typed UI rows before rendering

## Common Patterns

- `enum` for exclusive choices
- `struct` for draft state
- `match` over action enums for event handling
- centralized catalog key constants when the same key is reused across modules

## Forbidden Patterns

- raw `"zh-CN"` or `"en-US"` literals scattered across multiple UI files
- raw provider/capability strings scattered through screens
- stringly typed row template names
- direct indexing into ad hoc maps for popup control flow when an enum would do
