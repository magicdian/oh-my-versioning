# Implement Shared I18n for CLI and Init TUI

## Goal

Add first-class English and Chinese localization for CLI and init TUI output,
using one shared catalog system.

## Requirements

- Support `en-US` and `zh-CN`
- Persist locale preference in `.omv/config.toml`
- Use catalog-driven output instead of hardcoded strings
- Implement locale normalization, English fallback, and key parity validation

## Acceptance Criteria

- [ ] CLI and TUI both render through the shared catalog
- [ ] Locale preference persists across commands
- [ ] Catalog parity tests exist
- [ ] Missing selected-locale keys fall back to English
