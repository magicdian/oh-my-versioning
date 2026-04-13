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

## Before Writing New Code

### Step 1: Search First

```bash
rg "build_policy|version_output|locale|target language" src tests
```

### Step 2: Ask These Questions

| Question | If Yes... |
| --- | --- |
| Does a formatter or parser already exist? | Reuse it |
| Is this another language-target adapter? | Extend the adapter pattern |
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

## When to Abstract

Abstract when:

- a path/format rule is used by more than one command
- two language adapters share file-write sequencing
- multiple screens need the same row-derivation or popup-selection logic

Do not abstract when:

- the code is truly one-off
- the shared shape is not stable yet

## Checklist Before Commit

- [ ] Searched for an existing formatter or helper
- [ ] Did not copy a catalog lookup helper into a second module
- [ ] Did not duplicate target sync orchestration per command
- [ ] Reused one `.omv` path-resolution strategy
