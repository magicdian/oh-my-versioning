# Code Reuse Thinking Guide

> Stop before creating duplicate logic in `omv`.

---

## The Problem

This project is small enough that duplication will drift quickly if we are not
careful. The highest-risk duplication points are:

- version formatting
- date validation
- `.omv` path resolution
- target sync behavior
- i18n key lookup and formatting
- integration provider/capability descriptors and status/failure mapping

## Before Writing New Code

### Step 1: Search First

```bash
rg "build_policy|version_output|locale|target language" src tests
```

### Step 2: Ask These Questions

| Question | If Yes... |
| --- | --- |
| Does a formatter or parser already exist? | Reuse it |
| Is this another language-target adapter? | Extend the target adapter pattern |
| Is this another host integration provider/capability? | Extend the internal integration provider registry |
| Is this another user-facing string? | Add a catalog key, do not hardcode |
| Is this another `.omv` write path? | Reuse atomic storage helpers |

## Common Duplication Patterns

### Pattern 1: Multiple version calculators

**Bad**: separate "init version", "bump version", and "sync version" math

**Good**: one version engine used by every command

### Pattern 2: Per-command locale lookup wrappers

**Bad**: each command builds its own ad hoc translation helper

**Good**: one shared `Catalog` API

### Pattern 3: Per-language custom root resolution

**Bad**: each adapter re-discovers repo root and `.omv` location

**Good**: root is resolved once and passed down

### Pattern 4: Divergent adapter and integrate projection logic

**Bad**: `omv adapter refresh` and `omv integrate apply` render different
Codex/Trellis files from separate templates

**Good**: keep canonical `.omv/ai/*` generation and projection helpers shared;
let `.omv/integrations.toml` decide selected provider/capability state while
projection remains a reusable implementation detail

### Pattern 5: Repeated provider capability tables

**Bad**: copying `codex`, `trellis`, `project-instructions`,
`finalize-boundary`, and target-file lists into CLI, TUI, docs generation, and
tests independently

**Good**: define provider descriptors once in the backend integration registry
and derive CLI status, init review rows, generated contracts, and tests from
that typed model where practical

## When to Abstract

Abstract when:

- a path/format rule is used by more than one command
- two language adapters share file-write sequencing
- legacy adapter compatibility and integrate apply share host projection
  behavior
- multiple screens need the same row-derivation or popup-selection logic

Do not abstract when:

- the code is truly one-off
- the shared shape is not stable yet

## Checklist Before Commit

- [ ] Searched for an existing formatter or helper
- [ ] Did not copy a catalog lookup helper into a second module
- [ ] Did not duplicate target sync orchestration per command
- [ ] Reused one `.omv` path-resolution strategy
- [ ] Reused provider/capability descriptors instead of copying host support
      matrices
- [ ] Kept public plugin runtime claims out of MVP docs and code
