<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/phronia/guide.md contract=1 -->
# OMV Versioning Guide

- `.omv/state.toml` is the version source of truth.
- Use `omv current --json` for reads.
- Use `omv plan --json` to preview target changes.
- Use `omv sync --check --json` to verify drift without mutation.
- Use `omv integrate status --json` and `omv integrate apply --json` for host integration provider/capability state where available.
- The OMV finalize-boundary bump happens at the Phronia Phase 3.4 commit-confirmation boundary (`omv event finalize-boundary --provider <agent> --boundary commit ...`) with an explicit `change_type`, NOT at `/phronia:finish-work` (whose OMV block is verification-only). Supply an explicit `change_type`; do not infer it.
- Use `omv bump --json` for writes.
- Do not trust manifest versions as authority.
- Do not treat this guide or other host files as OMV authority.

Canonical reference: `./.omv/ai/instructions.md`