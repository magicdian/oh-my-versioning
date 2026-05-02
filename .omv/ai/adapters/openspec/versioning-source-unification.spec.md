<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/openspec/versioning-source-unification.spec.md contract=1 -->
# Spec: Versioning Source Unification

## Requirements

- The project MUST treat `.omv/state.toml` as version truth.
- Workflows MUST read current version via `omv current --json`.
- Workflows MUST update managed version via `omv bump --json`.
- Native manifests and runtime export files MUST be treated as derived outputs.