# Build Init Menuconfig TUI Flow

## Goal

Implement the `omv init` menuconfig-style TUI with `ratatui`.

## Requirements

- Follow the menuconfig style matrix
- Auto-discover likely project languages and preselect them
- Allow manual toggling with `Space`
- Support pre-project strategy popup when manifests do not yet exist
- Support localized status/help/confirm flows

## Acceptance Criteria

- [ ] Main flow matches single-column menuconfig behavior
- [ ] Keyboard semantics for `Space`, `Enter`, and `Esc` match the matrix
- [ ] Discovery and manual toggling both work
- [ ] Pre-project strategy popup is implemented and persisted into init state
