# brainstorm: adapt OMV Trellis integration for Trellis 0.5

## Goal

Adapt OMV's Trellis host integration to Trellis 0.5.x skill-first naming while
preserving compatibility with existing Trellis 0.4.x projects. The immediate
risk is the Trellis finish-work surface moving from
`.agents/skills/finish-work/SKILL.md` to
`.agents/skills/trellis-finish-work/SKILL.md`, which can make OMV's
`finalize-boundary` capability look uninstalled or fail to refresh after a
Trellis update.

## What I Already Know

* The user is preparing to update a project from Trellis 0.4.0 to Trellis 0.5.7.
* Trellis 0.5.x renames core skills and agents with a `trellis-` prefix.
* The dry-run migration includes
  `.agents/skills/finish-work/SKILL.md -> .agents/skills/trellis-finish-work/SKILL.md`.
* OMV currently treats installed host files as projections; `.omv/` remains the
  source of truth.
* OMV's Trellis spec-guide integration targets `.trellis/spec/guides/*`, which
  Trellis update preserves as user data.
* The risky capability is `finalize-boundary`, currently installed into
  `.agents/skills/finish-work/SKILL.md`.

## Assumptions

* OMV should support both Trellis 0.4.x and Trellis 0.5.x in the same released
  binary.
* A project may be partially migrated: both old and new finish-work skill files
  may exist, or only one may contain the OMV managed block.
* OMV must not blindly recreate deprecated Trellis 0.4 paths in a fully migrated
  Trellis 0.5 project.
* OMV should continue to use the same managed block id,
  `spec-trellis-finalize-boundary-finish-work`, so old blocks can be found and
  refreshed.

## Requirements

* Detect Trellis finalize-boundary installation across both known finish-work
  surfaces:
  * `.agents/skills/trellis-finish-work/SKILL.md` for Trellis 0.5.x and newer.
  * `.agents/skills/finish-work/SKILL.md` for Trellis 0.4.x compatibility.
* Prefer the Trellis 0.5.x path when both files exist and neither has a stronger
  installed-block signal.
* Preserve old-path compatibility: existing Trellis 0.4 projects must continue
  to receive OMV finalize-boundary installation and refreshes.
* Preserve existing managed blocks during migration: if the old path contains
  the OMV block and the new path exists, OMV should report a potentially
  abnormal mixed-migration state instead of silently treating the capability as
  healthy.
* Detect Trellis update backup-only states where the OMV block was moved into a
  `.backup` file while the active Trellis 0.5 skill file was overwritten.
* Do not automatically migrate the OMV managed block from the old path to the
  new path during status checks. Status should be read-only and should prompt
  the user to run `omv integrate apply` manually when a repair is needed.
* Update provider descriptors/status output so target paths communicate both the
  preferred path and compatibility path, or otherwise clearly expose which path
  is selected.
* Update tests and specs so the path-resolution contract is executable and
  future Trellis template changes are easy to adapt.

## Acceptance Criteria

* [ ] `omv integrate status --json` reports Trellis `finalize-boundary` as
      installed when the OMV managed block exists in the Trellis 0.5 path.
* [ ] `omv integrate status --json` reports Trellis `finalize-boundary` as
      installed when the OMV managed block exists only in the Trellis 0.4 path.
* [ ] `omv integrate apply --json` refreshes the Trellis 0.5 path when that
      path exists.
* [ ] `omv integrate apply --json` refreshes the Trellis 0.4 path when only the
      old path exists.
* [ ] When both paths exist and only the old path contains the OMV block,
      status reports a potentially abnormal state because Trellis may now run
      the new path, and the recovery hint tells the user to run
      `omv integrate apply`.
* [ ] When the OMV block exists only in a Trellis-created `.backup` file,
      status reports a potentially abnormal state and the recovery hint tells
      the user to run `omv integrate apply`.
* [ ] OMV does not create `.agents/skills/finish-work/SKILL.md` in a project
      that only has the Trellis 0.5 `trellis-finish-work` skill.
* [ ] Existing tests for stale managed block replacement still pass.
* [ ] Backend specs/docs mention both Trellis finish-work surfaces and the
      compatibility behavior.

## Definition of Done

* Tests added/updated for Trellis 0.4, Trellis 0.5, and mixed migration states.
* `cargo fmt --check` passes.
* `cargo test --all-targets --all-features` passes.
* Relevant `.trellis/spec/` docs are updated if the contract changes.
* Manual validation notes include a Trellis 0.4-style fixture and a Trellis
  0.5-style fixture.

## Research Notes

### Current Code Paths

* `src/core/integration.rs` declares Trellis `FinalizeBoundary` target paths as
  `.agents/skills/finish-work/SKILL.md`.
* `src/app/mod.rs` maps `(Trellis, FinalizeBoundary)` to one hard-coded
  `IntegrationTarget` with `host_rel = ".agents/skills/finish-work/SKILL.md"`.
* `src/app/mod.rs` status detection reads only that one path.
* `src/app/mod.rs` apply reads the existing finish-work file before inserting
  the OMV managed block, so a missing old path can fail installation.
* `src/adapter.rs` owns the managed block id and upsert behavior.
* `tests/integration/target_sync.rs` only covers the old path.
* `.trellis/spec/backend/database-guidelines.md` documents the old target path.

### Constraints From OMV

* `.omv/ai/*` remains canonical guidance; installed host files are derived
  projections.
* Dedicated managed blocks must only replace OMV-managed content and must not
  overwrite user-authored Trellis skill content.
* Trellis provider bootstrap still requires an existing `.trellis` directory.
* Compatibility must be path-based and content-based, not a network/version
  lookup.

## Feasible Approaches

### Approach A: Multi-path resolver per capability (recommended)

Add a small resolver for Trellis `finalize-boundary` that returns candidate
finish-work surfaces in priority order and chooses a concrete target based on
filesystem state.

Selection rules:

1. If any candidate contains the OMV managed block, treat the capability as
   installed.
2. For apply, refresh the installed candidate if exactly one contains the block.
3. If both paths exist and only the old path contains the block, status should
   report a potentially abnormal mixed-migration state and tell the user to run
   `omv integrate apply`.
4. If both contain the block, refresh the preferred Trellis 0.5 path and report
   the capability as installed. Dedupe/removal of the old block is outside the
   status path.
5. If no candidate contains the block, write to the preferred existing surface:
   Trellis 0.5 path first, then Trellis 0.4 path.
6. If a known `.backup` finish-work file contains the OMV block but no active
   surface does, status should report a potentially abnormal backup-only state
   and tell the user to run `omv integrate apply`.
7. If no known finish-work surface exists, return a clear install failure that
   asks the user to run/update Trellis first.

Pros:

* Keeps old and new Trellis compatible in one binary.
* Avoids creating deprecated old paths in Trellis 0.5 projects.
* Scales if Trellis renames another surface later.

Cons:

* Requires changing the current single-target integration model for at least one
  capability.

### Approach B: Provider version detection plus branching

Detect Trellis version from known files/configs and pick one target path based
on inferred version.

Pros:

* Status output can show one clean path.
* The branching model is easy to explain.

Cons:

* Version detection may be unreliable in partially migrated projects.
* More brittle than content/path capability detection.
* Still needs fallback behavior when version evidence is missing.

### Approach C: Install into both old and new paths when present

Treat both finish-work skill files as active surfaces and keep the OMV managed
block synchronized in both.

Pros:

* Maximum visibility across mixed hosts.
* Simple installed detection.

Cons:

* Can duplicate completion instructions.
* May preserve deprecated Trellis 0.4 files longer than intended.
* Higher chance of confusing users after a Trellis 0.5 migration.

## Recommended Technical Approach

Use Approach A. Model the Trellis `finalize-boundary` install target as a
resolved target rather than a static single path:

* Keep `IntegrationTargetBehavior::TrellisFinalizeBoundary`.
* Add candidate path support for that behavior, either as:
  * a targeted helper specific to Trellis finalize-boundary, or
  * a generalized `candidate_host_rels: Vec<&'static str>` field on
    `IntegrationTarget`.
* Prefer minimal generalization unless another capability immediately needs it.
* Keep the managed block id unchanged.
* Update descriptor target paths to include both paths or a preferred path plus
  compatibility note.
* Add tests before or with the resolver to lock the compatibility matrix.

## Expansion Sweep

### Future Evolution

* Trellis may rename other skills or move platform-specific files again; a
  candidate-path resolver could become a reusable provider-version compatibility
  mechanism.
* OMV may later support additional Trellis completion boundaries beyond
  finish-work, so the resolver should not bake semantic behavior into generic
  status code.

### Related Scenarios

* `AGENTS.md` and `.codex/skills/omv-versioning/SKILL.md` remain separate Codex
  projections and should not be changed for this task.
* Trellis spec guide and index snippet should remain in `.trellis/spec/guides/*`
  and should not be moved as part of finish-work compatibility.

### Failure and Edge Cases

* Both old and new files exist, old contains OMV block, new does not.
* Both files exist and both contain OMV blocks.
* Active Trellis 0.5 file exists, but the OMV block exists only in
  `.agents/skills/trellis-finish-work/SKILL.md.backup`.
* New file exists but has no `## Quick Check Flow` marker.
* Neither finish-work surface exists although `.trellis` exists.
* User manually edited the OMV managed block contents.

## Open Questions

* None currently.

## Decision (ADR-lite)

**Context**: Trellis 0.5 renames the active finish-work skill path. A migrated
project may still contain an OMV managed block in the old Trellis 0.4 path, or
only in a Trellis-created `.backup` file, while the actual runtime surface has
moved to the new Trellis 0.5 path.

**Decision**: OMV status must not auto-migrate managed blocks. If both old and
new finish-work surfaces exist and the OMV block is only present in the old
path, or if the block is only present in a known `.backup` file, status should
report that the capability may be abnormal and include a recovery hint to run
`omv integrate apply`. Apply is the explicit repair action that may
install/refresh the preferred Trellis 0.5 surface.

**Consequences**: Status remains read-only and avoids surprising host-file
mutation. Users get a clear warning when Trellis may be executing a different
surface than the one containing OMV guidance. Implementation needs a status
state or failure/hint representation that can distinguish "installed in
compatibility path only" from fully healthy installation.

## Out of Scope

* Removing Trellis 0.4 compatibility.
* Changing `.omv/` schema solely to record Trellis version.
* Implementing network-based Trellis latest-version checks.
* Reworking all host integration capabilities into a plugin system.
* Supporting future Trellis paths beyond the known 0.4 and 0.5 finish-work
  surfaces in this task.

## Technical Notes

* Likely files to modify during implementation:
  * `src/app/mod.rs`
  * `src/core/integration.rs`
  * `tests/integration/target_sync.rs`
  * `.trellis/spec/backend/database-guidelines.md`
  * `.trellis/spec/backend/quality-guidelines.md`
  * `docs/OMV_CONTRACT_ARCHITECTURE.md`
* Relevant specs for implementation:
  * `.trellis/spec/backend/directory-structure.md`
  * `.trellis/spec/backend/database-guidelines.md`
  * `.trellis/spec/backend/error-handling.md`
  * `.trellis/spec/backend/quality-guidelines.md`
  * `.trellis/spec/guides/cross-layer-thinking-guide.md`
  * `.trellis/spec/guides/code-reuse-thinking-guide.md`
