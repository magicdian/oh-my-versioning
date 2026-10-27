# Scaffold Rust CLI Workspace and OMV Schemas

## Goal

Create the initial Rust CLI project structure for `omv`, including the CLI
entrypoint, workspace layout, and typed schema models for `.omv/config.toml`,
`.omv/state.toml`, and `.omv/targets.toml`.

## Requirements

- Create a Cargo-based Rust CLI project named `omv`
- Establish the module layout described in backend/frontend specs
- Add typed schema structs/enums for config, state, targets, locale, build
  policy, output mode, and target language
- Keep `.omv` as the only source of truth

## Acceptance Criteria

- [ ] Cargo project builds
- [ ] Core schema types exist and compile
- [ ] Module boundaries match the bootstrap specs
- [ ] No business logic is duplicated across CLI entry and core modules
