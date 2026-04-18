<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/openspec/project.md contract=1 -->
# OMV Version Governance

This project uses `omv` as the authoritative version source.

- Version truth: `.omv/state.toml`
- Read current version: `omv current --json`
- Update version truth: `omv bump --json`
- Native manifests are synchronized outputs, not authority

See `./.omv/ai/instructions.md` for the canonical workflow.