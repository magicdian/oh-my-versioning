# Thinking Guides

> Short checklists that help contributors think before coding.

---

## Available Guides

| Guide | Purpose | When to Use |
|-------|---------|-------------|
| [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md) | Prevent duplicated version, i18n, and sync logic | When adding helpers, formatters, or adapters |
| [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md) | Trace data across `.omv`, `.omv/ai`, CLI/TUI, automation JSON, and target sync | Features spanning 3+ layers or touching persisted contracts |

## Quick Reference

Read [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md) when a
change touches any of:

- date/time validation
- `.omv` persistence
- `.omv/ai` projection
- target synchronization
- structured JSON contracts
- i18n preference and user-facing output
- CLI/TUI plus backend behavior together

Read [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md) when a change
introduces:

- new formatters
- new target adapters
- new catalog lookup helpers
- new path-resolution helpers

## Pre-Modification Rule

Before changing constants, config fields, or catalog keys:

```bash
rg "value_to_change" .
```

This is especially important for:

- locale keys
- version format strings
- target language identifiers
- `.omv` schema fields

<!-- OMV-MANAGED-BEGIN:integration-trellis-spec-index-snippet -->
## OMV

- [OMV Versioning Guide](./omv-versioning-guide.md) | Managed version and integration source rules
<!-- OMV-MANAGED-END:integration-trellis-spec-index-snippet -->
