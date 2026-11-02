# Pipeline Release Mode

Status: implemented in OMV `release_mode` support.

This document describes the "single-writer" release model for OMV projects with
more than one contributor. It is opt-in: the default remains the existing local
workflow.

## 1. Why

OMV's version truth is a single mutable file, `.omv/state.toml`. `omv bump` is a
read-modify-write operation that also rewrites native manifests and runtime
exports. If several branches run `bump` independently, three kinds of failure
appear:

1. **Text conflicts** in `.omv/state.toml`, `Cargo.toml`, lock files, and
   generated version exports.
2. **Semantic regressions**: a later merge can move `logical_date` backwards or
   reuse a build number.
3. **Order dependence**: the version depends on merge order rather than on the
   content of the change.

The fix is not a smarter merge. It is to make the version write a single-writer
operation: only the release pipeline assigns versions. Contributors read version
state and never write it.

## 2. Configuration

Add `release_mode` to `.omv/config.toml` and commit it with the project:

```toml
release_mode = "pipeline"
```

Accepted values:

| Value | Behavior |
|---|---|
| `local` | Default. `omv bump` and `omv sync` write for everyone. Existing behavior is unchanged. |
| `pipeline` | Version writes are reserved for the release job. Contributors use read-only commands. |

A missing key means `local`, so upgrading OMV does not change an existing
project. An invalid value fails with `invalid_release_mode`.

`release_mode` is independent of `project_profile`:

- `project_profile` describes the project (personal, oss, ...).
- `release_mode` describes who may write version truth.

## 3. Command matrix

| Command | `local` | `pipeline` without authorization | `pipeline` with `--as-release` |
|---|---|---|---|
| `omv current` | allowed | allowed | allowed |
| `omv plan` | allowed | allowed | allowed |
| `omv sync --check` | allowed | allowed | allowed |
| `omv sync` (write) | allowed | **denied** | allowed |
| `omv bump` | allowed | **denied** | allowed |
| `omv bump --dry-run` | allowed | allowed | allowed |
| `omv event finalize-task` | bumps | **pending, no write** | pending, no write |
| `omv event finalize-boundary` | bumps | **pending, no write** | pending, no write |
| `omv init` | allowed | allowed | allowed |

`--as-release` is accepted in `local` mode as a no-op, so CI scripts can be
portable across projects.

### New bump flags

```text
omv bump [--expect <version>] [--now <YYYY-MM-DD>] [--dry-run] [--as-release]
```

- `--expect <version>`: compare-and-swap against
  `state.last_issued_version`. A mismatch returns `state_moved` with the
  expected and actual versions. Use it in the release job to avoid double
  issuance when two jobs race.
- `--now <YYYY-MM-DD>`: use a fixed logical date instead of NTP/system time.
  `last_time_source` is recorded as `manual-confirmed`. Use it to make CI
  deterministic across time zones and midnight boundaries.
- `--dry-run`: compute the next version and target plan without writing any
  file. Returns the planned version and plan in JSON.
- `--as-release`: authorize a write in `pipeline` mode. It does not bypass
  `--expect` or any other validation.

## 4. Roles and file ownership

| Role | Reads version | Writes version surfaces | Commits code | Creates tags |
|---|---|---|---|---|
| Developer / AI agent | yes | no | yes (own branch) | no |
| Reviewer | yes | no | no | no |
| Release job | yes | yes | version commit only | yes |
| Tag pipeline | yes | no | no | no |

Version surfaces that the release job owns:

- `.omv/state.toml`
- `.omv/finalizations.toml` (when finalize records are used)
- native manifests: `Cargo.toml`, `CMakeLists.txt`, `pyproject.toml`, `go.mod`,
  `pom.xml`, and configured v2 targets
- lock files refreshed by the bump
- runtime exports such as `src/generated/version.rs` and `include/omv_version.h`
- generated guidance under `.omv/skills/` and `.omv/ai/`

Business code is never part of the release commit.

## 5. Release job flow

The release job is a transaction. Keep every step before the push in a
throwaway checkout; only push once the whole transaction is valid.

```text
1. Acquire the release lock (resource_group / concurrency).
2. Check out the main tip and verify a clean worktree.
3. Read `omv current --json` -> CURRENT.
4. omv bump --expect CURRENT --now <date> --as-release --json
5. omv sync --check --json
6. Refresh lock files (cargo metadata / project build) if needed.
7. Verify the diff contains only version-whitelisted paths.
8. Commit `chore(release): <version>`.
9. Push to main (CI-skip) and handle a rejected push by retrying from the new tip.
10. Create an annotated tag on the release commit with a receipt.
11. Hand off to the tag pipeline for build/publish.
```

### Failure handling

| Failure | Recovery |
|---|---|
| `state_moved` (CAS mismatch) | Fetch the new main tip, recompute `CURRENT`, retry the whole transaction. |
| Push rejected | Discard the local bump and restart from the new tip. Never rebase or merge a version commit. |
| Tag already exists | Treat as already released. If it points to the same commit, success; otherwise abort and investigate. |
| Job dies after push, before tag | A recovery run should detect the release commit at HEAD and create the missing tag without bumping again. |
| `sync --check` fails | Abort before commit; investigate target drift. |
| Unexpected file in the diff | Abort. Do not `git add -A`. |

Never revert `.omv/state.toml`. If a release is wrong, roll forward with a new
version.

## 6. GitLab CI sketch

The release job needs two capabilities:

1. A token that can push to the protected main branch (project access token or
   bot user). `CI_JOB_TOKEN` cannot push to protected branches by default.
2. Permission to create the version tag/release (`JOB-TOKEN` is sufficient for
   the Releases API in many GitLab versions; keep the tag guard described
   below).

```yaml
release:
  stage: ship
  resource_group: omv-release
  interruptible: false
  rules:
    - if: '$CI_COMMIT_BRANCH == "main" && $CI_PIPELINE_SOURCE == "push" && $SHIP == "1"'
      when: on_success
    - if: '$CI_COMMIT_BRANCH == "main" && $CI_PIPELINE_SOURCE == "push"'
      when: manual
  script:
    - |
      set -eu
      git fetch origin main
      git checkout -B omv-release "$CI_COMMIT_SHA"

      CURRENT="$(omv current --json | python3 -c 'import sys,json;print(json.load(sys.stdin)["data"]["version"])')"
      omv bump --expect "$CURRENT" --now "$(date +%F)" --as-release --json
      omv sync --check --json

      # v1 Rust targets do not update Cargo.lock; refresh it before committing.
      cargo metadata --format-version 1 > /dev/null

      ALLOWED='^(\.omv/(state|finalizations)\.toml|Cargo\.toml|Cargo\.lock|src/generated/version\.rs)$'
      if git diff --name-only | grep -vE "$ALLOWED"; then
        echo "unexpected file in release diff" >&2
        exit 1
      fi

      VERSION="$(omv current --json | python3 -c 'import sys,json;print(json.load(sys.stdin)["data"]["version"])')"
      git add .omv/state.toml .omv/finalizations.toml Cargo.toml Cargo.lock src/generated/version.rs
      git commit -m "chore(release): $VERSION"
      git push "https://oauth2:${OMV_RELEASE_TOKEN}@${CI_SERVER_HOST}/${CI_PROJECT_PATH}.git" HEAD:main -o ci.skip

      # Create an annotated tag/release on the version commit. Put provenance
      # in the tag message so the tag pipeline can verify it.
      curl -fsS --request POST --header "JOB-TOKEN: $CI_JOB_TOKEN" \
        --data "name=$VERSION" \
        --data "tag_name=$VERSION" \
        --data-urlencode "tag_message=omv-release receipt: source=$CI_COMMIT_SHA job=$CI_JOB_ID" \
        --data "ref=$(git rev-parse HEAD)" \
        "$CI_API_V4_URL/projects/$CI_PROJECT_ID/releases"
```

Notes:

- Mask `OMV_RELEASE_TOKEN` and expose it only to the protected release job.
- Keep the tag guard in the tag pipeline (`gate:release`): reject lightweight
  tags and tags without the release receipt.
- If you prefer creating the tag with `git tag -a` and `git push --tags`, do
  that with the same bot token and keep the receipt in the tag message.
- Adapt the `ALLOWED` pattern to every target/list file that OMV syncs in your
  project. A missing path in the whitelist is a correctness bug in the job, not
  a reason to weaken the check.

## 7. GitHub Actions sketch

```yaml
name: release

on:
  workflow_dispatch:
  push:
    branches: [main]

concurrency:
  group: omv-release
  cancel-in-progress: false

permissions:
  contents: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
          token: ${{ secrets.OMV_RELEASE_TOKEN }}
      - run: npm install -g @magicdian/omv@<pinned-version>
      - run: |
          set -euo pipefail
          CURRENT="$(omv current --json | python3 -c 'import sys,json;print(json.load(sys.stdin)["data"]["version"])')"
          omv bump --expect "$CURRENT" --now "$(date +%F)" --as-release --json
          omv sync --check --json
          cargo metadata --format-version 1 > /dev/null
          # same diff whitelist check as the GitLab example
          VERSION="$(omv current --json | python3 -c 'import sys,json;print(json.load(sys.stdin)["data"]["version"])')"
          git config user.name "omv-release-bot"
          git config user.email "omv-release-bot@users.noreply.github.com"
          git add .omv/state.toml .omv/finalizations.toml Cargo.toml Cargo.lock src/generated/version.rs
          git commit -m "chore(release): $VERSION"
          git push origin HEAD:main
          git tag -a "$VERSION" -m "omv-release receipt: source=$GITHUB_SHA run=$GITHUB_RUN_ID"
          git push origin "$VERSION"
```

Notes:

- The default `GITHUB_TOKEN` cannot bypass branch protection. Use a GitHub App
  or a fine-grained PAT, and add the app/actor to the branch-protection bypass
  list. Never use a personal PAT for shared automation.
- `concurrency` serializes releases. The `--expect` CAS is the second line of
  defense.
- If you do not want a push-triggered release, remove the `push` trigger and
  keep `workflow_dispatch` only.

## 8. PR/MR guard (read-only)

Every pull/merge request should prove it did not touch version surfaces:

```bash
omv sync --check --json

BASE=origin/main   # or the MR target branch
git fetch origin main
CHANGED="$(git diff --name-only "$BASE...HEAD" -- \
  .omv/state.toml \
  .omv/finalizations.toml \
  .omv/config.toml \
  Cargo.toml Cargo.lock \
  src/generated/version.rs)"
if [ -n "$CHANGED" ]; then
  printf '%s\n' "$CHANGED" >&2
  echo "PR must not change version surfaces" >&2
  exit 1
fi
```

Include `.omv/config.toml` if you want policy changes (including
`release_mode`) to require release-owner review.

## 9. Structured output examples

### Dry run

```json
{
  "ok": true,
  "contract_version": "1",
  "command": "bump",
  "data": {
    "previous_version": "2611.2.1",
    "version": "2611.2.2",
    "logical_date": "2026-11-02",
    "build_number": 2,
    "time_source": "manual-confirmed",
    "plan": {
      "version": "2611.2.2",
      "project_status": "drift",
      "totals": { "ok": 0, "drift": 1, "missing": 0, "unsupported": 0, "error": 0, "skipped": 0 }
    }
  },
  "error": null
}
```

### CAS mismatch

```json
{
  "ok": false,
  "error": {
    "code": "state_moved",
    "message": "state moved: expected last_issued_version=2611.2.1, found 2611.2.2",
    "details": { "expected": "2611.2.1", "actual": "2611.2.2" }
  }
}
```

### Pipeline write without authorization

```json
{
  "ok": false,
  "error": {
    "code": "release_mode_denied",
    "message": "release_mode=pipeline: `bump` may only write in the release pipeline; pass --as-release in an authorized release job",
    "details": {
      "command": "bump",
      "release_mode": "pipeline",
      "hint": "release jobs must pass --as-release; other callers should use current/plan/sync --check"
    }
  }
}
```

### Finalize event in pipeline mode

```json
{
  "ok": true,
  "command": "event.finalize-boundary",
  "data": {
    "release_mode": "pipeline",
    "outcome": "pending",
    "reason": "release-pipeline-owns-version",
    "manual_action_required": true,
    "version": "2611.2.1",
    "provider": "phronia",
    "boundary": "finish-work",
    "task_id": "11-02-example",
    "change_type": "feature"
  },
  "error": null
}
```

## 10. Migration

1. **Stay on `local` until ready.** No action is required; existing configs and
   scripts keep working.
2. **Add the policy.** Set `release_mode = "pipeline"` in `.omv/config.toml`
   and commit it. Run `omv current --json` once to refresh `.omv/ai/*` with
   pipeline-aware guidance; run `omv sync --as-release` if you also want the
   generated `.omv/skills/*` guidance refreshed.
3. **Add the MR guard.** Run `omv sync --check --json` and reject changes to
   version surfaces.
4. **Add the release job.** Start in a test branch or with the job's push step
   disabled; verify `bump`, `sync --check`, lock refresh, and the diff
   whitelist.
5. **Grant the bot only what it needs.** Protected-branch push plus tag/release
   creation. Keep the token scoped to the project.
6. **Update agent/spec instructions.** Pipeline-aware guidance is generated by
   OMV; commit the refreshed `.omv/ai/*` and host projections together.
7. **Verify parallel PRs.** Merge two branches in either order; confirm a
   monotonic `state.toml`, a unique tag, and matching manifest versions.

### Rollback

Set `release_mode = "local"` or remove the key. No data migration is needed.
The local workflow resumes immediately.

## 11. Boundaries and FAQ

**Does this change local users?**
No. `local` is the default and the code paths are unchanged when the key is
absent.

**Does OMV push or tag?**
No. OMV only computes and writes the working tree. Commit, push, and tag remain
CI responsibilities, which keeps credentials and policy outside the CLI.

**Why not detect CI (`CI=true`) and allow writes there?**
Environment variables are heuristic and spoofable, and they cannot distinguish
"running read-only checks in CI" from "running an authorized release job". Use
the committed policy plus an explicit `--as-release` in the release job. The
real boundary is branch/tag protection and the bot token.

**Does `--as-release` bypass the CAS?**
No. It only satisfies the release-mode guard. `--expect` still applies.

**What about hotfixes or multiple packages?**
Those need separate version domains (release branches or per-package versions).
`pipeline` mode still applies per domain, but OMV currently models one global
version. This is documented as a future extension, not part of this mode.

**What about change fragments / changesets?**
Not included. `pipeline` mode only moves the write to one place. A fragment
layer can be added later without changing the single-writer invariant.
