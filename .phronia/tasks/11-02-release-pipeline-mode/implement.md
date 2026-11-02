# Implementation Plan: Pipeline release mode

## Order

1. Data model and config
2. Error taxonomy and i18n
3. CLI parsing
4. App authorization / date / CAS / dry-run
5. Event pending behavior
6. Current JSON
7. Tests
8. Documentation
9. Full verification

## Step 1 — Data model and config

- `src/core/versioning/mod.rs`: add `ReleaseMode { Local, Pipeline }` with
  `Default = Local`, `as_str`, `parse`.
- `src/core/schema.rs`: add `release_mode: ReleaseMode` to `OmvConfig` and the
  default impl.
- `src/storage/config.rs`:
  - parse `release_mode`;
  - missing => default Local;
  - invalid => `ConfigError::InvalidReleaseMode`;
  - write `release_mode = "..."` in `save_config`.
- Update config tests for default, explicit, invalid, and round trip.

## Step 2 — Errors and i18n

- `src/errors.rs`:
  - `CliError::MissingOptionValue(String)`
  - `CliError::ReleaseModeDenied { command: String }`
  - `ConfigError::InvalidReleaseMode(String)`
  - `StateError::ExpectationMismatch { expected, actual }`
  - add `Display`, `code()`, and `details()` mappings.
- `resources/i18n/en-US.toml` and `zh-CN.toml`: add the new keys listed in the
  design; keep catalogs in parity.
- `src/app/mod.rs` `render_help`: add the new option lines.

## Step 3 — CLI parsing

- `src/cli/mod.rs`:
  - `Command::Bump(BumpCommand)`; add `BumpCommand`.
  - `SyncCommand.as_release`.
  - `parse_bump_command`; extend `parse_sync_command`.
  - Update matches/tests that assume `Command::Bump` is a unit variant.
- Add parser tests for every flag and missing-value errors.

## Step 4 — App execution

- Load config once at the handler boundary where needed.
- Add `authorize_release_write(omv_root, as_release, command)`.
- `run_bump(..., command: BumpCommand)`:
  - dry-run => `execute_bump_preview`;
  - otherwise authorize, then `execute_bump` with expect/now options.
- Refactor `execute_bump` to accept `expect` and `now`.
- Add `execute_bump_preview`.
- `run_sync(..., command: SyncCommand)`:
  - authorize only when `!check`.
- Add `release_mode` to `CurrentExecution` and `execute_current`.
- Keep `BumpExecution` shape unchanged; add a separate preview struct.

## Step 5 — Event pending

- In `run_event`, load config + state.
- If `release_mode == Pipeline`, build `PipelinePendingExecution` from the
  command fields and return it without calling `execute_finalize_*`.
- Text rendering uses `cli.event.pipeline_pending`.
- Local path unchanged.

## Step 6 — Tests

Add tests for:

- config default/explicit/invalid/round trip;
- CLI parsing of new flags;
- local mode bump unchanged;
- pipeline bump denied/allowed;
- pipeline sync denied/allowed/check allowed;
- `--expect` match/mismatch;
- `--now` date and `manual-confirmed`;
- `--dry-run` no writes + planned version;
- event pending no writes for both actions;
- current JSON includes `release_mode`;
- i18n parity.

Prefer extending existing test helpers (`temp_omv_root`, `seed_project`, etc.)
rather than introducing new harnesses.

## Step 7 — Documentation

- Add `docs/RELEASE_PIPELINE.md` with the content described in the design.
- Update `README.md` command/global-option sections and link the doc.
- Keep `.omv/ai/*` generated files untouched unless a test/refresh requires
  regeneration; the design doc is the operator-facing reference.

## Step 8 — Verification

Run:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Also run a manual smoke test in a temporary project:

```bash
# local mode (no config change): bump still works
# pipeline mode: bump denied; bump --as-release works; sync denied; sync --check works
# bump --dry-run: no files changed
# event finalize-boundary: pending, no state change
```

## Rollback

- Revert the commit; no persisted data format change exists beyond the optional
  config key.
- Projects that opted into `pipeline` can revert to `release_mode = "local"` or
  remove the key.
- No migration of `state.toml`/`finalizations.toml` is required.
