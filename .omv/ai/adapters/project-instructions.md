<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/project-instructions.md contract=1 -->
# OMV Agent Instructions

Read `./.omv/ai/instructions.md` before touching project versions.

- Use `omv current --json` to inspect the managed version.
- Use `omv plan --json` before editing version-sensitive surfaces.
- Use `omv sync --check --json` to verify target drift without writing.
- Use `omv integrate status --json` and `omv integrate apply --json` for host integration provider/capability status where available.
- At completion boundaries (during commit confirmation when a unit of work is complete), call the OMV finalize-boundary helper from `.omv/ai/contract.json` with an explicit `change_type`. For Trellis v0.5+ this runs at the commit-confirmation boundary, not at `/trellis:finish-work` (which only verifies the commit-boundary bump). Otherwise, call it manually after tests pass.
- After a semantic bump, run the project build command (e.g. `cargo build` for Rust, `npm install` for Node) to refresh lock files, then commit the OMV-generated files (`.omv/state.toml`, `.omv/finalizations.toml`, lock files like `Cargo.lock`, generated version files) together.
- Use `omv bump --json` to advance the managed version.
- Do not edit native manifest versions directly.
- Treat this host file as a derived projection; `.omv/*` and `.omv/ai/*` remain authoritative.