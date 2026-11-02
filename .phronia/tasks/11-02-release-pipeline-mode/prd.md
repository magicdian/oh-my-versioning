# PRD: Pipeline release mode

## Goal

Add an opt-in `release_mode = "pipeline"` policy that makes the release job the
only writer of `.omv/state.toml` and all OMV-derived version surfaces, while
keeping `release_mode = "local"` (the default) byte-for-byte compatible with
today's behavior.

## Background

OMV currently treats `omv bump` as a local read-modify-write operation. In a
multi-person or PR-based workflow, two branches can both bump from the same
state, producing textual conflicts in `state.toml`/manifests and semantic
conflicts (regressed `logical_date`, duplicate build numbers, or a version that
depends on merge order).

The agreed solution is "Plan B": keep `.omv/state.toml` as the truth, but move
the write to a single serialized release job. Contributors only read version
state; the release job runs `bump + sync`, commits the version surfaces, pushes,
and tags. This PRD covers the OMV-side policy and command support needed for
that model. CI job wiring is documented, not implemented in this repository.

## Requirements

### R1 — Configurable release mode

- Add `release_mode` to `.omv/config.toml` with values `"local"` and `"pipeline"`.
- Default is `local`.
- A missing `release_mode` key parses as `local`, so upgrading OMV does not
  change existing projects.
- An invalid value fails with a typed config error.
- `omv current --json` exposes the effective `release_mode`.

### R2 — Pipeline write authorization

In `pipeline` mode:

- `omv bump` refuses to write unless `--as-release` is passed.
- `omv sync` (write mode) refuses to write unless `--as-release` is passed.
- `omv sync --check`, `omv plan`, `omv current`, and `omv bump --dry-run` remain
  available without authorization.
- `omv event finalize-task` and `omv event finalize-boundary` do not write
  state or finalizations; they return a structured `pending` result with
  `manual_action_required: true` and reason `release-pipeline-owns-version`.
- In `local` mode all existing commands behave exactly as today; `--as-release`
  is accepted but has no additional effect.

### R3 — Release-safe bump primitives

- `omv bump --expect <version>` performs compare-and-swap against
  `state.last_issued_version`; mismatch returns a typed `state_moved` error.
- `omv bump --now <YYYY-MM-DD>` uses the provided logical date instead of
  NTP/system time and records `last_time_source = "manual-confirmed"`.
- `omv bump --dry-run` computes the next version and target plan without
  writing any file; it returns the planned version and plan in JSON.
- Existing `omv bump` behavior (including `--no-ntp`) is unchanged when the new
  flags are absent.

### R4 — Operator documentation

- Add `docs/RELEASE_PIPELINE.md` describing the pipeline release model:
  roles, command matrix, CI wiring (GitLab and GitHub), permissions,
  idempotency/recovery, migration, and failure handling.
- Link the document from `README.md`.

### R5 — Compatibility

- Default local mode must remain behavior-compatible: no changed exit codes,
  no changed text output, and no required new config key.
- Existing `.omv/config.toml` files without `release_mode` must load and save
  without error.
- All existing tests must continue to pass.

## Acceptance Criteria

- [ ] A project without `release_mode` behaves identically to today.
- [ ] `release_mode = "pipeline"` blocks `bump` and `sync` writes without
      `--as-release`, with a clear structured error.
- [ ] `release_mode = "pipeline"` returns pending for both finalize event
      actions without mutating `.omv/state.toml`, `.omv/finalizations.toml`, or
      any target file.
- [ ] `bump --dry-run` produces identical next-version calculations to a real
      bump but writes nothing.
- [ ] `bump --expect` fails when the stored version differs and succeeds when it
      matches.
- [ ] `bump --now` uses the supplied date and records `manual-confirmed`.
- [ ] `current --json` includes `release_mode`.
- [ ] `docs/RELEASE_PIPELINE.md` exists and is linked from `README.md`.
- [ ] `cargo test` and `cargo clippy --all-targets -- -D warnings` pass.

## Non-Goals

- No change fragments / changesets input layer.
- No merge queue or merge-train integration.
- No git commit/push/tag inside OMV.
- No per-target or per-package version domains.
- No version allocation service.
- No CI workflow files added to this repository.

## Risks

- `finalize-boundary` is part of existing AI/spec workflows; returning `pending`
  in pipeline mode must not write partial state or break local mode.
- Adding fields to JSON output must remain additive.
- Time override must not allow stale/future state regressions.
