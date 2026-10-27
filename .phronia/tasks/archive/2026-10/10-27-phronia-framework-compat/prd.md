# Phronia framework compatibility and coexistence guard

## Goal

Make Phronia a first-class spec/workflow provider in OMV, and make multi-framework projects fail safely instead of letting OMV guess which framework owns versioning projections.

## Requirements

- Add `phronia` as an OMV integration provider and legacy spec adapter alongside `trellis`.
- Project OMV guidance into:
  - `.phronia/spec/guides/omv-versioning-guide.md`
  - `.phronia/spec/guides/index.md` (managed block)
- Inject a verification-only `finalize-boundary` block into every existing Phronia finish-work surface:
  - `.agents/skills/phronia-finish-work/SKILL.md`
  - `.claude/commands/phronia/finish-work.md`
  - `.opencode/commands/phronia/finish-work.md`
  - `.zcode/commands/phronia/finish-work.md`
- Resolve Phronia task ids without requiring platform session knowledge:
  - explicit `--task-id`
  - `.phronia/.current-task`
  - a single agreeing `.phronia/.runtime/sessions/*.json` pointer
  - the only task with status `in_progress` or `review`
- Do not auto-delete or auto-replace Trellis metadata when `.phronia` is detected.
  When `.trellis` and `.phronia` coexist, report an actionable conflict and let
  the user decide which framework to keep.
- Never recreate a removed framework host from a stale adapter record.
- Normalize CRLF/LF for sync drift comparison and write generated files with LF.
- Keep generated contract, instructions, docs, i18n, and tests in sync.

## Acceptance Criteria

- [x] `omv integrate status/apply` detects Phronia and installs all three capabilities.
- [x] `omv adapter install --spec phronia` works when `.phronia` exists.
- [x] Both `.trellis` and `.phronia` present:
  - `status` reports `spec-framework-conflict`
  - `apply` fails with `spec-framework-conflict`
  - no framework files are written or deleted
- [x] A stale Trellis adapter record in a Phronia-only project cannot recreate `.trellis`.
- [x] CRLF-only differences are not drift; generated writes contain no CR.
- [x] `cargo fmt --check`, `clippy -D warnings`, and full tests pass.
- [x] Released through dev → main → tag with npm/GitHub artifacts.

## Notes

- This task was backfilled after implementation because the transition-period
  session skipped the normal planning gate.
