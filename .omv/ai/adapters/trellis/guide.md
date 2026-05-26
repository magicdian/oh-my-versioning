<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/trellis/guide.md contract=1 -->
# OMV Versioning Guide

- `.omv/state.toml` is the version source of truth.
- Use `omv current --json` for reads.
- Use `omv plan --json` to preview target changes.
- Use `omv sync --check --json` to verify drift without mutation.
- Use `omv integrate status --json` and `omv integrate apply --json` for host integration provider/capability state where available.
- **Trellis v0.5+:** If the Trellis finalize-boundary capability is installed, call the OMV helper advertised in `.omv/ai/contract.json` during Phase 3.4 commit confirmation, before `/trellis:finish-work`. When the user confirms a commit during Phase 3.4, run `omv event finalize-boundary --provider trellis --boundary finish-work --change-type <type> --json`, then commit the resulting bump files. Supply an explicit `change_type`; do not infer it.
- **Trellis v0.4:** When the finalize-boundary block is present in the finish-work skill, the `/trellis:finish-work` flow may trigger `finalize-boundary` automatically. If it does not, invoke it explicitly after finish-work succeeds.
- Use `omv bump --json` for writes.
- Do not trust manifest versions as authority.
- Do not treat this guide or other host files as OMV authority.

Canonical reference: `./.omv/ai/instructions.md`