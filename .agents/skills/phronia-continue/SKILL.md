---
name: phronia-continue
description: "Resume work on a task. Finds or asks for the task to continue, resolves its workspace (current checkout or worktree), then loads the workflow Phase Index and step-level detail. Use when coming back to a task and you need to know what to do next."
---

# Continue a Task

Resume work on a task — choose the task, resolve its workspace, then pick up at the right phase/step in `.phronia/workflow.md`.

Workspaces are **task-level**, not session-level. One task has one branch and at most one worktree; a second session on the same task attaches to it instead of creating another.

---

## Step 1: Find the task

```bash
python3 ./.phronia/scripts/task.py current --source
```

### No active task

1. List the candidates:

   ```bash
   python3 ./.phronia/scripts/task.py list --status in_progress
   python3 ./.phronia/scripts/task.py list --status planning
   ```

2. Ask the user **once** which task to continue. Do not silently pick the first one.
3. Continue to Step 2 with that task ref.

### Active task

Use the reported task path as the ref. If `--source` says the pointer is stale, treat it as "no active task" and ask which task to continue.

## Step 2: Resolve the workspace

Run the workspace decision with the **explicit task ref**:

```bash
phronia task workspace ensure <task-ref> --json
```

Session identity is required for claim ownership. Pass `--session-key <key>` or set `PHRONIA_CONTEXT_ID`; never rely on a CLI process pid (each `phronia` command is a new process, so pid liveness is meaningless).

Interpret the result:

- `reuse-current` — the main checkout is now owned by this task. Work here.
- `attach` — the task already has a worktree (shared registry, or the platform-created one). Use the returned path and branch.
- `create` — another task owns the main checkout, or this task needs isolation. Phronia creates the worktree and prints it. **Tell the user to start a new agent session in that directory** (or use the platform's native worktree launch). Phronia cannot move a running session's cwd.
- `refuse` — follow the printed `next` command: usually commit/stash the main checkout, release another owner's claim, or fix a branch/path conflict.

If the task does not exist yet but the user wants to start isolated work, create a provisional workspace before creating the task:

```bash
phronia task workspace prepare --for-new-task "<title>" --json
python3 ./.phronia/scripts/task.py create "<title>" --description "<what it delivers>"
```

`task.py create` inside that provisional worktree binds the task to it (no worktree gets created in the main checkout).

If the result is `create` while an active task pointer exists for a **different** task, do not edit the main checkout for this one: attach/create the worktree and tell the user which directory to open.

## Step 3: Set the active pointer (when appropriate)

- Task is `in_progress` — set the pointer so the per-turn breadcrumb follows it:

  ```bash
  python3 ./.phronia/scripts/task.py start <task-ref>
  ```

  `start` is idempotent for an already in-progress task.

- Task is `planning` — **do not** run `task.py start` yet; it flips the status to `in_progress` before planning review. Stay in Phase 1 and call `task.py start` at step 1.4 after the artifacts are reviewed.

## Step 4: Load current context and the Phase Index

```bash
python3 ./.phronia/scripts/get_context.py
python3 ./.phronia/scripts/get_context.py --mode phase
```

`get_context.py` reports the active task, git state, and recent commits. `--mode phase` shows the Plan / Execute / Finish index with routing + skill mapping.

## Step 5: Decide where you are

Route by `status` + artifact presence. This skill explains where to resume; it does not itself approve implementation.

- `status=planning` + no `prd.md` → **1.1** (load `phronia-brainstorm`)
- `status=planning` + `prd.md` only → decide whether the task is lightweight or complex. Lightweight can move to **1.4** review; complex returns to **1.1** to add `design.md` + `implement.md`.
- `status=planning` + complex artifacts complete + sub-agent jsonl not curated (empty, or only a legacy `_example` placeholder row) → **1.3**
- `status=planning` + required artifacts complete + required jsonl curated or inline mode → **1.4** (ask for start review; only run `task.py start` after user confirms)
- `status=in_progress` + implementation not started → **2.1**
- `status=in_progress` + implementation done, not yet checked → **2.2**
- `status=in_progress` + check passed → **3.3** (spec update) → **3.4** (commit)

Parallel/wave questions (which tasks can run together, what to dispatch) route to `phronia-coordinator`. Its capability gate decides between coordinator, multi-session, and serial.

Phase rules (full detail in `.phronia/workflow.md`):

1. Run steps **in order** within a phase — `[required]` steps must not be skipped
2. `[once]` steps are already done if the required output exists. `prd.md` alone can be enough only for lightweight tasks; complex tasks also need `design.md` and `implement.md`.
3. You may go back to an earlier phase if discoveries require it

## Step 6: Load the specific step

```bash
python3 ./.phronia/scripts/get_context.py --mode phase --step <X.X> --platform codex
```

Follow the loaded instructions. After each `[required]` step completes, move to the next.

For worktree tasks, remember the finish line: commit inside the task worktree (`phronia task commit <ref>`), then `phronia task integrate <ref>`, then ``finish-work` (Phronia command)` (which archives).

---

## Reference

Full workflow and detailed phase steps live in `.phronia/workflow.md`. This command is only an entry point — the canonical guidance is there.
