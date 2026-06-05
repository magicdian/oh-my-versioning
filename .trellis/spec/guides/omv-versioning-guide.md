<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/trellis/guide.md contract=1 -->
<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/trellis/guide.md contract=1 -->
# OMV Versioning Guide

- `.omv/state.toml` is the version source of truth.
- Use `omv current --json` for reads.
- Use `omv plan --json` to preview target changes.
- Use `omv sync --check --json` to verify drift without mutation.
- Use `omv integrate status --json` and `omv integrate apply --json` for host integration provider/capability state where available.
- If the Trellis finalize-boundary capability is installed: for Trellis v0.5+ the finalize-boundary bump happens at the Phase 3.4 commit-confirmation boundary (`omv event finalize-boundary --provider <agent> --boundary commit ...`) with an explicit `change_type`, NOT at `/trellis:finish-work` (whose OMV block is verification-only). For Trellis v0.4.x, `/trellis:finish-work` remains the bump trigger (legacy behavior). Supply an explicit `change_type`; do not infer it.
- Use `omv bump --json` for writes.
- Do not trust manifest versions as authority.
- Do not treat this guide or other host files as OMV authority.

Canonical reference: `./.omv/ai/instructions.md`