<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/project-instructions.md contract=1 -->
# OMV Agent Instructions

Read `./.omv/ai/instructions.md` before touching project versions.

- Use `omv current --json` to inspect the managed version.
- Use `omv plan --json` before editing version-sensitive surfaces.
- Use `omv sync --check --json` to verify target drift without writing.
- Use `omv integrate status --json` and `omv integrate apply --json` for host integration provider/capability status where available.
- During commit confirmation (when a unit of work is complete and ready to commit), call the OMV finalize-boundary helper from `.omv/ai/contract.json` only after tests pass and only with an explicit `change_type`. Do not wait until a separate finish-work step — call finalize-boundary as soon as the commit is confirmed.
- Use `omv bump --json` to advance the managed version.
- Do not edit native manifest versions directly.
- Treat this host file as a derived projection; `.omv/*` and `.omv/ai/*` remain authoritative.