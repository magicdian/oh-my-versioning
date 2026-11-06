---
name: phronia-finish-work
description: "Wrap up the session: integrate the task (local fast-forward or remote PR/MR), stop on failure, sync spec deltas, archive completed tasks, then regenerate docs and record the session journal. Use when done coding and ready to end the session."
---

# Finish Work

Wrap up the current session in this order: **integrate → spec sync → archive → docs/journal**. Code commits are NOT done here — those happen in workflow Phase 3.4 before you invoke this command.

`integrate`, `spec sync`, and `archive` are different steps. Integrate lands the work and records the outcome; spec sync applies the task's `spec-delta.md` in the main checkout; archive moves the task directory and cleans up a landed worktree. Never archive a task whose integrate failed, is dirty, has not landed, or has an unsynced spec delta.

## Preconditions

Before integrating, confirm all of these:

1. **The task worktree is clean or the work is committed.** Check the worktree path returned by `phronia task workspace ensure <ref> --json` (or the shared registry via `phronia task worktree list --json`):
   ```bash
   git -C <worktree-path> status --porcelain
   ```
   Non-empty means uncommitted work. Return to Phase 3.4 or run `phronia task commit <ref>` inside the task worktree. Do not archive to "clean it up".
2. **Task artifacts are committed.** `prd.md` / `design.md` / `implement.md`, the task's `spec-delta.md`, and any code changes belong in the work commits, not in the archive commit. A code task must not commit `.phronia/spec/**`; if it did, `integrate` fails with `SPEC_IN_CODE_TASK`.
3. **Verification is configured, or `--no-verify` is explicit.** If the project has no verification command, integrate refuses; pass `--no-verify` only when the user accepts an unverified landing. The outcome is then reported as unverified.

If the task has no worktree, integrate operates on the current checkout; the same preconditions apply to the main checkout.

<!-- OMV-MANAGED-BEGIN:spec-phronia-finalize-boundary-finish-work -->
## OMV Finalize Boundary (Verify — run BEFORE archive)

For Phronia, the OMV version bump already happened at the Phase 3.4 commit-confirmation boundary (`omv event finalize-boundary --provider <agent> --boundary commit ...`). Do NOT bump again here. These steps only verify the commit-boundary bump landed.

- [ ] Do NOT run `omv event finalize-boundary` in finish-work; the unit of work was already finalized at the commit boundary.
- [ ] Run `omv sync --check --json`. Treat any required target drift as blocking: drift here means the commit-boundary bump was not synced/committed — surface it and resolve before archiving.
- [ ] If `Cargo.lock` / other lock files or generated version files are uncommitted (e.g. the build step was skipped after the commit-boundary bump), run the project build command (e.g. `cargo build` for Rust, `npm install` for Node) and commit them before archiving.
<!-- OMV-MANAGED-END:spec-phronia-finalize-boundary-finish-work -->

## Step 1: Survey current state

```bash
python3 ./.phronia/scripts/get_context.py --mode record
```

This prints:

- **My active tasks** — review whether any besides the current one are actually done (code merged, AC met) and should be archived this round.
- **Git status** — quick visual on what's dirty.
- **Recent commits** — you'll need their hashes in Step 6 for `--commit`.

If `--mode record` surfaces other completed tasks not tied to the current session, surface them to the user with a one-shot confirmation: "These N tasks look done — integrate and archive them too in this round? [y/N]". Default is no; the current active task is always handled regardless.

## Step 2: Sanity check — classify dirty paths

Run:

```bash
git status --porcelain
```

Filter out paths under `.phronia/workspace/` and `.phronia/tasks/` — those are managed by `add_session.py` and `task.py archive` auto-commits and will appear dirty as part of this skill's own work.

For each remaining dirty path, decide whether it belongs to **the current task** or to **other parallel work** (e.g., another terminal window editing the same repo). Heuristics:

- Paths referenced in the current task's `prd.md` / `implement.jsonl` / `check.jsonl` → current task
- Paths in code areas matching the task's stated scope, or that you remember editing this session → current task
- Paths in unrelated areas you have no recollection of touching this session → other parallel work

Then route:

- **Any remaining path looks like current-task work** — bail out with:
  > "Working tree has uncommitted code changes from this task: `<list>`. Return to workflow Phase 3.4 to commit them before running ``finish-work` (Phronia command)`."

  Do NOT run `git commit` here. Do NOT prompt the user to commit. The user goes back to Phase 3.4 and the AI drives the batched commit there.
- **All remaining paths look unrelated** (other parallel-window work) — report them once and continue to Step 3:
  > "FYI, dirty files outside this task's scope — leaving them for the other window: `<list>`."
- **Genuinely unsure** — ask the user once: "Are `<list>` this task's work I forgot to commit, or another window's? (commit / ignore)" — then route per their answer.

## Step 3: Integrate every task being archived

Run integrate **before** archive, with an explicit task ref:

```bash
phronia task integrate <task-ref>
```

- Integrate resolves the mode (`local` / `remote`, or `auto`) and always prints the resolved mode plus the reason. If the choice is genuinely ambiguous, it asks once and records the answer in `meta.integration.mode`; subsequent retries do not ask.
- `local` — rebase onto the integration branch, verify the rebased tree, then fast-forward. The result is a linear history and `meta.integration.outcome=landed`.
- `remote` — push the task branch and create a PR/MR. Phronia never merges remotely; the outcome stays `pushed` until the forge merges.
- The integration lock is shared by every worktree of the same repository; only one integration runs at a time. `--queue` / `--wait` / `--drain` are available for batches.

For each extra task confirmed in Step 1, run its own `phronia task integrate <ref>` first. `phronia task integrate --drain` fits a batch of already-queued tasks.

### On integration failure: stop

If `integrate` exits non-zero, **stop this skill immediately**. Do not run `task.py archive`, `phronia docs generate`, or `add_session.py`.

- Read the printed error (`code`, `message`, `next`) and the log path.
- For conflicts or semantic failures, follow the handoff (`--handoff` writes `.phronia/.runtime/integration-handoff.md`). Resolve in the task worktree, commit, then retry integrate.
- The task keeps its worktree and its `in_progress` status; nothing was archived.

Never archive a task whose integrate failed, whose worktree is dirty, or whose `meta.integration.outcome` is not `landed`.

## Step 4: Sync spec deltas

A code task records spec intent in `.phronia/tasks/<ref>/spec-delta.md`; the task branch must not contain `.phronia/spec/**` changes. After the code has landed, the main checkout applies the delta.

```bash
phronia task spec plan <task-ref>
# Apply the delta to .phronia/spec/** in the main checkout, using the landed diff
# and the current spec text. Keep the delta's target list authoritative.
phronia task spec commit <task-ref>
```

- `spec plan` is read-only and requires `meta.integration.outcome=landed`.
- `spec commit` holds the same integration lock as `integrate`, refuses a dirty non-spec main checkout, commits only `.phronia/spec/**` as `chore(spec): sync <refs>`, then records `meta.spec_sync={status:"synced",commit,synced_at,refs}` in a metadata-only commit.
- A task without `spec-delta.md` reports `none`; skip it.
- If the plan or commit fails (not landed, target missing, lock busy, no spec change), **stop**. Do not archive.
- `--skip-spec-sync` is only for an explicit user decision. It records the reason in the task's `notes`, and archive then proceeds.

## Step 5: Archive task(s)

```bash
python3 ./.phronia/scripts/task.py archive <task-name>
```

At minimum: the current active task (if any). Plus any extra tasks the user confirmed in Step 1, each of which must have passed Step 3. Each archive produces a `chore(task): archive ...` commit via the script's auto-commit.

Archive enforces the lifecycle guard:

- **pending spec delta** (`SPEC_SYNC_PENDING`) → refused; run Step 4 first, or use `--skip-spec-sync` only with explicit user consent.
- **dirty worktree** → refused; commit in the worktree or explicitly remove it.
- **clean worktree + outcome `landed`** → the worktree is removed automatically (the branch is kept), then the task is archived.
- **clean worktree + not landed** (including `pushed`, `blocked`, `none`) → refused; integrate first, or explicitly `phronia task worktree remove <ref>`.
- **no worktree** → the pre-existing archive behavior is unchanged.
- **branch metadata** is resolved from `task.json.branch` → the shared worktree registry → `landed` + an integration commit reachable from the base. The normal path no longer needs `--skip-branch-validation`.

`phronia task worktree prune` is a separate maintenance command, not part of archive. Use it to clean up already-archived / landed / clean worktrees; it never deletes dirty or unlanded ones.

If there is no active task and the user did not confirm any cleanup archives, skip this step.

After archiving, refresh the mechanical knowledge layer:

```bash
phronia docs generate
```

If the change introduced a new module, data flow, or architectural decision,
also update `docs/architecture.md`; if it changed capabilities or user-facing
behavior, update `docs/product.md`. Both are narrative and reviewed like code.

## Step 6: Record session journal

```bash
python3 ./.phronia/scripts/add_session.py \
  --title "Session Title" \
  --commit "hash1,hash2" \
  --summary "Brief summary"
```

Use the work-commit hashes produced in Phase 3.4 (visible in Step 1's `Recent commits` list, or via `git log --oneline`) for `--commit`. Do not include the spec-sync or archive commit hashes from Steps 4–5. This produces a `chore: record journal` commit.

Final git log order: `<work commits from 3.4>` → `<integrate metadata-only commit, if any>` → `chore(spec): sync ...` → `chore(task): record spec sync ...` → `chore(task): archive ...` (one or more) → `chore: record journal`.
