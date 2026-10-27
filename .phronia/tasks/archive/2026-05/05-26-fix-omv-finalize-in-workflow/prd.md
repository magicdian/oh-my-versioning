# Fix OMV finalize-boundary not triggered in workflow Phase 3.4

## Goal

Ensure AI agents reliably call `omv event finalize-boundary` after committing code, by embedding the step directly into workflow.md Phase 3.4 and fixing contradictory timing instructions across OMV adapter files.

## Problem Statement

Three locations give conflicting or disconnected guidance on when to call `finalize-boundary`:

1. **`.omv/ai/adapters/project-instructions.md`** — "During commit confirmation...call finalize-boundary as soon as the commit is confirmed"
2. **`.omv/ai/adapters/trellis/guide.md`** — "after `/trellis:finish-work` succeeds"
3. **`workflow.md` Phase 3.4** — no mention of OMV at all

The agent follows workflow.md's Phase 3.4 step list (inspect → draft → confirm → commit → done) and never encounters the OMV instruction. The project-instructions block is in a separate injection context that doesn't cross-reference workflow steps.

## Requirements

1. Add an OMV finalize-boundary step to workflow.md Phase 3.4, **after step 6 (commit executed)** and before 3.5 wrap-up
2. Also add a mention in the `[workflow-state:in_progress]` and `[workflow-state:in_progress-inline]` breadcrumb blocks (per the INVARIANT rule: required steps must appear in the breadcrumb)
3. Fix `.omv/ai/adapters/trellis/guide.md` timing: change "after `/trellis:finish-work` succeeds" → "after Phase 3.4 commit succeeds (before `/trellis:finish-work`)"
4. The step must be **conditional** — only fire when `.omv/` exists in the project (not all Trellis projects use OMV)
5. Make finish-work's existing OMV checklist a **fallback/verification** rather than the primary trigger point

## Acceptance Criteria

- [ ] workflow.md Phase 3.4 contains explicit OMV finalize-boundary step (conditional on `.omv/` presence)
- [ ] `[workflow-state:in_progress]` breadcrumb mentions OMV finalize-boundary after commit
- [ ] `[workflow-state:in_progress-inline]` breadcrumb mentions OMV finalize-boundary after commit
- [ ] `.omv/ai/adapters/trellis/guide.md` timing is consistent with workflow.md
- [ ] No regression: projects without `.omv/` are unaffected

## Definition of Done

- Lint/format pass
- Workflow breadcrumb INVARIANT still holds (finalize-boundary mentioned in breadcrumb for its phase)
- Manual review confirms no contradictory timing language remains

## Out of Scope

- Implementing a git hook approach (separate feature if desired)
- Changing the `finalize-boundary` CLI interface
- Modifying the finish-work skill's OMV block (it stays as verification/fallback)

## Technical Approach

**Key constraint**: finalize-boundary must run **before** `git commit`, so that version-bumped files (state.toml, Cargo.toml, generated version.rs, Cargo.lock, etc.) are included in the same commit as the code change — or in a dedicated version-bump commit immediately after. If the project has a lock file (Cargo.lock, package-lock.json, etc.), a build/check step must run after finalize-boundary to update it before committing.

**Timing sequence** (within Phase 3.4):
```
draft commit plan → user confirms → finalize-boundary → build (update lock) → git add all → git commit
```

### Changes:

1. In `workflow.md` Phase 3.4, insert between step 5 (confirm) and step 6 (commit):
   - New step 6: "If `.omv/` exists: determine `change_type` from commit content, run `omv event finalize-boundary`, then run project build command to update lock files. Add OMV-generated files to the commit file list."
   - Renumber old step 6 → 7, old step 7 → 8

2. In `[workflow-state:in_progress]` breadcrumb block: append after "drives the commit" sentence: "Phase 3.4 OMV (required when `.omv/` present, once): before committing, call `omv event finalize-boundary` + rebuild to update lock files, then include version files in the commit."

3. Same for `[workflow-state:in_progress-inline]` breadcrumb block.

4. Update `.omv/ai/adapters/trellis/guide.md` timing: change "after `/trellis:finish-work` succeeds" → "during Phase 3.4 before committing (after user confirms the commit plan)"

5. Update finish-work skill's OMV block to clarify it's a **verification fallback** (check that finalize-boundary was already called; if not, call it before archive).
