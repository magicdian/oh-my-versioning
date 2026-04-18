<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/trellis/guide.md contract=1 -->
# OMV Versioning Guide

- `.omv/state.toml` is the version source of truth.
- Use `omv current --json` for reads.
- Use `omv bump --json` for writes.
- Do not trust manifest versions as authority.

Canonical reference: `./.omv/ai/instructions.md`