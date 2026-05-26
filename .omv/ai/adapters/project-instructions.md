<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/project-instructions.md contract=1 -->
# OMV Agent Instructions

Read `./.omv/ai/instructions.md` before touching project versions.

- Use `omv current --json` to inspect the managed version.
- Use `omv plan --json` before editing version-sensitive surfaces.
- Use `omv sync --check --json` to verify target drift without writing.
- Use `omv integrate status --json` and `omv integrate apply --json` for host integration provider/capability status where available.
- At completion boundaries (when `/trellis:finish-work` or equivalent wrap-up is invoked), the OMV finalize-boundary helper from `.omv/ai/contract.json` will be called with an explicit `change_type`. If the project uses Trellis, this happens automatically via the finish-work skill's OMV block. Otherwise, call it manually after tests pass.
- Use `omv bump --json` to advance the managed version.
- Do not edit native manifest versions directly.
- Treat this host file as a derived projection; `.omv/*` and `.omv/ai/*` remain authoritative.