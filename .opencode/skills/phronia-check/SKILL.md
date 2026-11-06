---
name: phronia-check
description: "Comprehensive quality verification: spec compliance, lint, type-check, tests, cross-layer data flow, code reuse, and consistency checks. Use when code is written and needs quality verification, before committing changes, or to catch context drift during long sessions."
---

# Code Quality Check

Comprehensive quality verification for recently written code. Combines spec compliance, cross-layer safety, and pre-commit checks.

---

## Step 1: Identify What Changed

```bash
git diff --name-only HEAD
git status
```

## Step 2: Read Task Artifacts and Applicable Specs

Read the current task artifacts in order:

- `prd.md`
- `design.md` if present
- `implement.md` if present

```bash
python3 ./.phronia/scripts/get_context.py --mode packages
```

For each changed package/layer, read the spec index and follow its **Quality Check** section:

```bash
cat .phronia/spec/<package>/<layer>/index.md
```

Read the specific guideline files referenced — the index is a pointer, not the goal.

## Step 3: Run Project Checks

Run the project's lint, type-check, and test commands. Fix any failures before proceeding.

## Step 3.5: Record Machine-Readable Verdicts

Refresh the rule ledger and record one verdict per verified rule you evaluated:

```bash
phronia rules compile
phronia rules verdict --task .phronia/tasks/<task> --input verdicts.json --check
```

`verdicts.json` is an array of `{ "id": "CLI-BE-001", "status": "pass" }`
entries (`fail` requires `evidence`). The command writes
`check-results.json` inside the task and reports any verified rules you did not
evaluate as `missing`. Draft rules are advisory and need no verdict.

## Step 4: Review Against Checklist

### Code Quality

- [ ] Linter passes?
- [ ] Type checker passes (if applicable)?
- [ ] Tests pass?
- [ ] No debug logging left in?
- [ ] No suppressed warnings or type-safety bypasses?

### Test Coverage

- [ ] New function → unit test added?
- [ ] Bug fix → regression test added?
- [ ] Changed behavior → existing tests updated?

### Spec Delta

Spec is control-plane state; the task branch must not carry spec writes.

- [ ] Does the task produce spec knowledge? If yes, is it recorded in `.phronia/tasks/<ref>/spec-delta.md` rather than `.phronia/spec/**`?
- [ ] Does every `## Target files` entry resolve to a repo-relative path under `.phronia/spec/` and match the intended change?
- [ ] Is the delta complete enough for the main session to apply after integration (intent, rationale, verification)?
- [ ] Does the diff contain any `.phronia/spec/**` change? That is `SPEC_IN_CODE_TASK`; revert it and move the intent into the delta. Do not "fix" it by editing spec in the task branch.

> "If I fixed a bug or discovered something non-obvious, should I document it so future me won't hit the same issue?" → If YES, write it into `spec-delta.md`; the main session applies it after the code lands.

### Scope Discipline

- [ ] Any tidying of code the task did not require?
- [ ] Any abstraction, config or extension point added for a case that does not exist yet?
- [ ] Any speculative fallback for a state that cannot occur?
- [ ] Any file changed that the acceptance criteria do not mention?
- [ ] Any workaround added at the caller instead of a fix where the behavior actually lives?

## Step 5: Cross-Layer Dimensions (if applicable)

Skip this step if your change is confined to a single layer.

### A. Data Flow (changes touch 3+ layers)

- [ ] Read flow traces correctly: Storage → Service → API → UI
- [ ] Write flow traces correctly: UI → API → Service → Storage
- [ ] Types/schemas correctly passed between layers?
- [ ] Errors properly propagated to caller?

### B. Code Reuse (modifying constants, creating utilities)

- [ ] Searched for existing similar code before creating new?
  ```bash
  grep -r "pattern" src/
  ```
- [ ] If the same value repeats, does it represent one stable concept whose callers must change together? Extract only then — two literals that merely happen to match today should stay separate.
- [ ] After batch modification, all occurrences updated?

### C. Import/Dependency (creating new files)

- [ ] Correct import paths (relative vs absolute)?
- [ ] No circular dependencies?

### D. Same-Layer Consistency

- [ ] Other places using the same concept are consistent?

---

## Step 6: Report and Fix

Report every violation you find. Then:

- Mechanical and local (lint nit, missing type, wrong import, dead branch, failing assertion) → fix in place, then re-run project checks.
- Design or judgment (naming a shared concept, moving a module boundary, changing a public interface, reassigning where behavior lives) → record the evidence and your recommendation, and stop. Do not rewrite it silently.

If a fix would touch files outside the current task's scope, say so and stop instead of widening the change.
