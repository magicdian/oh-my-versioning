# Implementation Plan

This task was backfilled after the code landed. The checklist records the work
in the order it was performed and the validation gate used before release.

- [x] Add `IntegrationProvider::Phronia` and `SpecAdapter::Phronia`.
- [x] Add canonical Phronia guide / index snippet sources.
- [x] Add Phronia finish-work verification block and multi-surface injection.
- [x] Add Phronia task-id resolution.
- [x] Add coexistence conflict detection and wire it into
      `integrate status`, `integrate apply`, `adapter install/refresh`, and `init`.
- [x] Require an existing host tree for Trellis/Phronia spec adapter installs.
- [x] Normalize CRLF/LF in sync comparison and force LF on generated writes.
- [x] Update contract, instructions, docs, i18n, and tests.
- [x] Refresh project projections with the npm-installed OMV.
- [x] Run validation:
      `cargo fmt --check`
      `cargo clippy --all-targets --all-features -- -D warnings`
      `cargo test --all-targets --all-features`
      `omv sync --check --json`
- [ ] Bump to the next daily build, commit, push `dev`, merge to `main`,
      tag, and publish the GitHub/npm release.
- [ ] Archive this task and record the session journal.

## Rollback

Revert the feature commit and the version bump commit together. Generated
projections can be restored with `omv integrate apply` after checking that only
one spec/workflow host tree remains.
