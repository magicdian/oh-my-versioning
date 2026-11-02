# Design: Pipeline release mode

## 1. Model

`release_mode` is a project-level policy stored in `.omv/config.toml` and
committed with the repository. It is orthogonal to `project_profile`: profile
describes the project, release mode describes who may write version truth.

```rust
enum ReleaseMode { Local, Pipeline } // Default = Local
```

Rules:

- `local`: existing behavior. `bump` and `sync` write for everyone.
- `pipeline`: `bump` and `sync` (write mode) require explicit
  `--as-release` authorization. `current`, `plan`, `sync --check`, and
  `bump --dry-run` remain read-only and unguarded.
- `finalize-task` and `finalize-boundary` never write in `pipeline` mode; they
  return a pending result.

The guard is intentionally **not** based on environment detection (`CI=true`,
`GITLAB_CI`, `GITHUB_ACTIONS`). Those signals are heuristic and spoofable. The
policy is config + explicit capability; real enforcement is branch/tag
protection plus CI diff guards.

## 2. Config

Extend `OmvConfig`:

```rust
pub release_mode: ReleaseMode,
```

- Default `Local`.
- Parse `release_mode = "local" | "pipeline"` in `src/storage/config.rs`.
- Missing key => `Local`.
- Invalid value => `ConfigError::InvalidReleaseMode(value)`.
- Serialize the key in `save_config` so `omv init` writes it explicitly.
- Include `release_mode` in `omv current --json` via `CurrentExecution`.

No `schema_version` bump: the field is additive and absence is defined as
`local`.

## 3. CLI surface

```text
omv bump [--expect <version>] [--now <YYYY-MM-DD>] [--dry-run] [--as-release]
omv sync [--check] [--as-release]
```

`Command::Bump` becomes `Command::Bump(BumpCommand)`.

```rust
struct BumpCommand {
    expect: Option<String>,
    now: Option<String>,
    dry_run: bool,
    as_release: bool,
}
```

`SyncCommand` gains `as_release: bool`.

New CLI parse errors:

- `MissingOptionValue("--expect" | "--now")` -> code `missing_option_value`.
- `ReleaseModeDenied { command }` -> code `release_mode_denied`.

`--as-release` is accepted in `local` mode as a no-op so CI scripts can be
portable across projects.

## 4. Bump execution

### 4.1 Authorization

`run_bump` loads config. If `release_mode == Pipeline` and `!dry_run` and
`!as_release`, return `release_mode_denied` with a hint. `run_sync` applies the
same guard only when `!check`.

The guard lives at the command-handler boundary, not inside `execute_sync`,
because `execute_bump` calls `execute_sync` internally. Internal calls must not
re-check authorization.

### 4.2 Date resolution

Refactor date resolution into one helper:

```text
if --now <date>:
    validated = ValidatedDate { date: parse_iso(date), source: ManualConfirmed }
else:
    validated = validate_current_date(effective_config, state, ntp, system)
```

`--now` bypasses NTP even when `ntp_enabled = true`; the explicit date wins.
`compute_next_version` still rejects `stored > validated`, so future-date
protection is preserved. Invalid dates produce `invalid_date_format`.

### 4.3 Compare-and-swap

Before computing or writing:

```text
if let Some(expected) = command.expect:
    if state.last_issued_version != expected:
        return StateError::ExpectationMismatch { expected, actual }
```

Code: `state_moved`. Details: `expected`, `actual`. Applies to dry-run too, so
CI can assert preconditions before a real release.

### 4.4 Dry run

`execute_bump_preview`:

1. Load config/state/targets.
2. Resolve date and compute next version.
3. Run `plan_all_targets(project_root, targets, next.value)`.
4. Return `BumpPreviewExecution { previous_version, version, logical_date,
   build_number, time_source, plan }`.
5. Do not save state, do not sync, do not regenerate skills/adapters.

`run_bump` selects text key `cli.bump.preview` or JSON envelope `bump` with the
preview payload.

### 4.5 Normal bump

Unchanged order: save state, then sync, then return `BumpExecution`. Existing
callers/tests keep the same data shape.

## 5. Event pipeline pending

`run_event` loads config. In `pipeline` mode it returns a read-only payload for
both actions without calling `execute_finalize_*`:

```json
{
  "release_mode": "pipeline",
  "outcome": "pending",
  "reason": "release-pipeline-owns-version",
  "manual_action_required": true,
  "version": "<current version>",
  "provider": "...",      // finalize-boundary only
  "boundary": "...",      // finalize-boundary only
  "task_id": "...",       // if supplied
  "change_type": "..."    // if supplied
}
```

Text key: `cli.event.pipeline_pending`.

Local mode keeps the existing finalize execution path unchanged.

## 6. Errors and i18n

New typed errors:

| Error | Code | Details |
|---|---|---|
| `CliError::MissingOptionValue(option)` | `missing_option_value` | `option` |
| `CliError::ReleaseModeDenied { command }` | `release_mode_denied` | `command`, `release_mode`, `hint` |
| `ConfigError::InvalidReleaseMode(value)` | `invalid_release_mode` | `release_mode` |
| `StateError::ExpectationMismatch { expected, actual }` | `state_moved` | `expected`, `actual` |

New i18n keys in both catalogs:

- `cli.bump.preview`
- `cli.event.pipeline_pending`
- `cli.help.options.expect`
- `cli.help.options.now`
- `cli.help.options.dry_run`
- `cli.help.options.as_release`

## 7. Testing

Unit/integration tests in the existing test modules:

- Config: missing key => local; explicit pipeline; invalid value => typed error;
  save/load round trip.
- CLI: parse all new flags; missing values; `--as-release` on sync.
- Bump: local mode unchanged; pipeline denies without flag; allows with flag;
  `--expect` match/mismatch; `--now` date + manual-confirmed; `--dry-run`
  writes nothing and returns planned version.
- Sync: pipeline denies write without flag; `--check` works; `--as-release`
  allows write.
- Event: pipeline returns pending and leaves state/finalizations untouched;
  local path still bumps.
- Current: JSON includes `release_mode`.
- i18n catalog parity tests.

## 8. Compatibility and migration

- Default `Local`; no behavior change without an explicit config edit.
- Existing config files load unchanged; saving adds the key explicitly.
- No new required dependencies.
- Documentation explains how to opt in and how to revert.

## 9. Documentation

`docs/RELEASE_PIPELINE.md` covers:

- Why single-writer versioning
- Role/permission matrix
- Command matrix
- GitLab release job sketch (resource_group, bot token, push retry, tag receipt)
- GitHub Actions equivalent (concurrency, App/PAT, branch protection bypass)
- CAS / idempotency / recovery
- Migration steps and rollback
- What is intentionally out of scope (fragments, merge queue, per-package)
