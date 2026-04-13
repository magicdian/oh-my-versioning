# Implement Target Sync Adapters and AI Skills Templates

## Goal

Implement flat target registration, target synchronization, runtime export
generation, and `.omv/skills` guidance that drives version updates through
`omv bump`.

## Requirements

- Support flat targets in `.omv/targets.toml`
- Implement V1 language families: C/C++, Java, Rust, Python, Go
- Sync native manifests from `.omv`
- Generate runtime-readable version exports where appropriate
- Generate AI guidance under `.omv/skills`

## Acceptance Criteria

- [ ] Each V1 language family has a sync/export path defined in code
- [ ] `omv bump` can sync registered targets
- [ ] `.omv/skills` instructs AI tooling to use `omv bump`
- [ ] Target sync behavior is covered by integration tests
