# brainstorm: product gaps and automation hooks

## Goal

Assess `oh-my-versioning` from a product and automation perspective: identify
missing user-facing interfaces, find weak points in AI-driven silent automation,
and evaluate whether agent hook mechanisms can drive version updates
automatically for bugfixes and requirement changes.

## What I already know

* The user wants a product-level gap analysis rather than a narrow code fix.
* The user explicitly cares about three areas:
  * missing interfaces for end users
  * deficiencies in AI model automation / silent invocation
  * hook-driven automatic version updates for bugfix and requirement changes
* The user prefers invisible automation over manual CLI bumping.
* The desired trigger is not "every code edit", but "after functional work is
  complete and tested".
* The current architecture question is whether OMV should integrate lightly into
  frameworks such as OpenSpec/Trellis, rely on agent hooks, or combine both.
* The repo is a single Rust CLI/TUI project with backend and frontend spec
  layers.
* Existing code/modules indicate current scope around version calculation,
  `.omv` persistence, target sync, adapter state, i18n, and a TUI init flow.
* Current runtime commands are `init`, `current`, `bump`, `sync`, `adapter
  install|refresh|list|status`, `help`, and `version`.
* `omv bump` already performs state mutation plus manifest/runtime-export sync.
* The product already generates `.omv/ai/contract.json`,
  `.omv/ai/instructions.md`, `.omv/skills/*`, and host adapter projections for
  `codex`, `claude`, `openspec`, and `trellis`.
* Current agent/spec adapters inject guidance only; they do not install
  executable lifecycle hooks or workflow wrappers.
* Config schema already contains `project_profile` and `version_output`, but the
  current init UI/CLI does not expose knobs to change them.
* Target schema supports `id`, `root`, `manifest_path`, `runtime_export_path`,
  `strategy`, and `enabled`, but current init flow only creates one default
  target per language with fixed paths.
* `PreProjectStrategy` is collected in init state but is not consumed by target
  sync adapters yet.
* Manual confirmation for future-date conflicts is mentioned in specs, but the
  current runtime only returns a blocking error.
* No current task was active when this brainstorm started.

## Assumptions (temporary)

* The desired outcome is a concrete product/technical roadmap, not immediate
  implementation.
* "Agent hook" likely refers to external coding-agent lifecycle hooks or
  command wrappers that can trigger `omv` actions automatically during
  development workflows.
* The right first deliverable may be a scoped MVP recommendation instead of a
  full automation design.
* "功能修复/需求增加" should be treated as semantic change categories, but the
  actual version bump should happen at a later milestone than the first code
  edit.

## Open Questions

* How strongly should OMV enforce automatic `finalize-task` invocation?
  * A. best-effort hook only
  * B. wrapper-command primary + hook fallback
  * C. framework-owned completion flow only

## Requirements (evolving)

* Inventory the currently exposed product surfaces in the repo.
* Identify likely missing interfaces from an end-user perspective.
* Evaluate current automation suitability for agent-first usage.
* Propose feasible approaches for hook-based automatic version updates.
* Recommend an MVP sequence that avoids over-automation before contracts become
  stable.
* Distinguish "semantic change detection" from "final version mutation timing".
* Prefer automatic bump execution only after development has reached a stable
  completion checkpoint.
* First MVP completion checkpoint is an explicit `finalize-task` action after
  tests pass.
* The system should minimize the chance that `finalize-task` is skipped by
  making it part of the normal completion path rather than optional guidance.
* The preferred primary completion boundary is the existing Trellis
  `$finish-work` flow.
* Optional host hooks should call the same OMV event endpoint only as a safety
  net, not as sole authority.
* `finalize-task` must be idempotent for duplicate calls from wrapper flows or
  hooks.
* Automatic bumping should only happen for semantic changes such as bugfixes
  and requirement additions, not for refactor-only or doc-only work.
* The workflow should include a blocking verification step so semantic work
  cannot be considered complete if `finalize-task` was skipped.

## Acceptance Criteria (evolving)

* [ ] Current product surfaces are summarized from actual repo evidence.
* [ ] Missing interfaces are grouped by user-facing product value.
* [ ] AI automation risks and gaps are identified concretely.
* [ ] At least 2 feasible hook-based versioning approaches are compared.
* [ ] A recommended MVP direction is proposed for follow-up implementation.
* [ ] Recommended trigger point aligns with "complete and tested" rather than
  "any code write".
* [ ] Recommended invocation strategy explains how `finalize-task` becomes a
  reliable workflow boundary.
* [ ] MVP design defines the primary caller, fallback callers, and idempotency
  contract for `finalize-task`.
* [ ] MVP design explains how change type is supplied to OMV.

## Definition of Done (team quality bar)

* Product and technical findings are grounded in repo inspection.
* Scope boundaries and trade-offs are explicit.
* Recommended next step is concrete enough to convert into implementation work.

## Out of Scope (explicit)

* Implementing the chosen automation flow in this brainstorm step.
* Finalizing external agent platform integrations without confirming target
  agents/hooks.

## Technical Notes

* Task directory: `.trellis/tasks/04-18-product-gaps-automation-hooks`
* Relevant files discovered in initial scan include:
  * `src/main.rs`
  * `src/cli/mod.rs`
  * `src/core/versioning/engine.rs`
  * `src/storage/*.rs`
  * `src/sync/*.rs`
  * `src/ui/**/*`
  * `README.md`
  * `docs/matrix/MENUCONFIG_STYLE_MATRIX.md`

## Research Notes

### Current product surface confirmed from repo

* User-facing command surface is intentionally small:
  * `omv init`
  * `omv current`
  * `omv bump`
  * `omv sync`
  * `omv adapter install|refresh|list|status`
* Agent-facing machine contract is currently command-oriented rather than
  event-oriented:
  * read current truth
  * write next truth
  * refresh adapter projections
* Current init UX exposes locale, timezone, build policy, language selection,
  and pre-project strategy popup.

### Likely missing interfaces from a product perspective

* Config management interface:
  * no `omv config get|set`
  * no way to change `project_profile` or `version_output` through supported UX
* Target management interface:
  * no `omv target add|edit|remove|list`
  * no way to customize root, manifest path, or runtime export path
  * no practical monorepo / multi-module management flow yet
* Preview / audit interface:
  * no `omv bump --dry-run`
  * no `omv diff` / `omv explain`
  * no history / rollback / reason log
* Automation operations interface:
  * no `omv hook install`
  * no `omv event bump`
  * no idempotency token or task-scoped mutation API
* Recovery / trust interface:
  * future-date conflict has no confirmation or repair command
  * no `doctor` / `repair` command for drifted manifests or stale adapters

### AI silent automation gaps

* Current `.omv/ai/contract.json` only tells tools which commands to call; it
  does not define:
  * when a version bump is required
  * how to prevent duplicate bumps
  * how to encode task / issue / requirement context
  * how to distinguish preview from mutation
* Adapter outputs are documentation-first:
  * good for instruction injection
  * weak for "silent" automation because no executable hook or wrapper is
    projected
* Version mutation is not auditable enough for automated agents:
  * `state.toml` stores the last version and time source
  * it does not store actor, reason, task id, diff hash, or requirement id
* Hook-driven automation would currently over-bump easily because repeated agent
  callbacks have no idempotency contract.
* Multi-step work is not modeled:
  * a bugfix or requirement change may involve many edits
  * current model only exposes `bump now`, not `stage / decide / finalize`
* Pure spec/framework integration is not enough for invisible automation:
  OpenSpec/Trellis can describe intent and state, but they are not the best
  execution boundary for "now perform the bump" unless paired with a runtime
  trigger.
* Pure hook integration is not enough for semantic accuracy:
  hooks can see lifecycle moments such as "after tests" or "task complete", but
  without structured context they may not know whether the change is a bugfix,
  requirement expansion, refactor-only work, or no-version-change work.

### Feasible approaches here

**Approach A: Framework-state driven** 

* How it works:
  * OMV integrates into OpenSpec/Trellis task/spec lifecycle
  * spec/task state carries change classification and completion status
  * framework completion events or commands call OMV
* Pros:
  * best semantic clarity
  * easiest place to encode bugfix vs requirement change
* Cons:
  * weak portability outside those frameworks
  * still not truly invisible unless the framework exposes/owns the final
    execution point

**Approach B: Hook-driven automation**

* How it works:
  * agent/CLI hooks call an OMV event endpoint after tests, task completion, or
    session-finalization
  * OMV decides whether to bump and syncs outputs
* Pros:
  * best "无感" experience
  * works across multiple agent/spec ecosystems
* Cons:
  * semantic accuracy is weaker unless extra metadata is passed in
  * highest risk of duplicate/false-positive bumps if the hook contract is too
    thin

**Approach C: Hybrid state + hook** (Recommended)

* How it works:
  * frameworks such as OpenSpec/Trellis provide structured change context
    (`bugfix`, `new requirement`, task id, requirement id, status)
  * a hook provides the timing boundary (`tests passed`, `task finalized`,
    `ready to merge`)
  * OMV owns one event API that consumes both and performs the bump
* Pros:
  * best balance of semantic accuracy and invisible execution
  * keeps OMV portable while still benefiting from richer framework context
  * supports future non-framework hook callers too
* Cons:
  * needs a small event contract and audit model before implementation

### Finalize-task invocation strategies

**Strategy 1: Documentation-only**

* Put `finalize-task` in adapter instructions and workflow docs.
* Reliability: low.
* Problem: this is still advisory and will be skipped by agents or humans.

**Strategy 2: Host-native hook only**

* Install a hook in supported hosts so "tests passed" or "task completed"
  directly calls `omv event finalize-task`.
* Reliability: medium.
* Problem: hook support differs by host, and some flows will bypass it.

**Strategy 3: Wrapper-command primary + hook fallback** (Recommended)

* Make the normal completion path go through an OMV-owned command such as:
  * `omv finish`
  * `omv task finalize`
  * or a Trellis/OpenSpec-integrated completion wrapper
* That wrapper:
  * runs or verifies tests
  * checks task/spec completion metadata
  * calls `omv event finalize-task`
  * records an idempotency fingerprint
* Optional host hooks call the same endpoint as a safety net for supported
  environments.
* Reliability: highest practical MVP.

### Why wrapper-command primary is stronger

* You cannot truly "ensure" an external hook fires unless OMV owns the workflow
  boundary.
* A wrapper command makes finalize part of the only supported completion path.
* Hooks should be treated as acceleration / safety net, not sole authority.
* The event endpoint should be idempotent so duplicate calls are harmless.

## Technical Approach

### MVP boundary

* Normal completion path:
  * implement work
  * run tests / verification
  * run `$finish-work`
  * Trellis/OpenSpec completion wrapper calls `omv event finalize-task`
  * OMV decides whether to bump and then syncs outputs
  * completion check verifies that semantic changes were finalized
* Fallback path:
  * supported agent hooks may call the same OMV endpoint after a matching
    completion event
  * duplicate calls are ignored by idempotency checks

### Proposed CLI / event surface

* New command family:
  * `omv event finalize-task`
* Example shape:

```bash
omv event finalize-task \
  --task-id 04-18-product-gaps-automation-hooks \
  --change-type bugfix \
  --status done \
  --tests passed \
  --fingerprint <stable-value> \
  --source trellis-finish-work \
  --json
```

### OMV decision rules in MVP

* Return `no-op` when:
  * tests are not marked passed
  * task status is not complete
  * change type is `refactor`, `docs`, or `chore`
  * the same fingerprint was already finalized
* Execute bump when:
  * task is complete
  * tests passed
  * change type is `bugfix` or `feature`
  * fingerprint is new

### Minimal persistence additions

* Add a new OMV audit file, likely:
  * `.omv/events.toml`
  * or `.omv/finalizations.toml`
* Minimal record fields:
  * `task_id`
  * `fingerprint`
  * `change_type`
  * `source`
  * `tests`
  * `result` (`bumped` / `noop`)
  * `version_before`
  * `version_after`
  * `timestamp`

### Fingerprint direction for idempotency

* First MVP should avoid git-deep inference.
* Use a stable completion fingerprint derived from workflow metadata, for
  example:
  * `task_id + change_type + completion_state + current_version`
  * optionally plus a spec/task revision marker when available
* Goal:
  * same completion event repeated -> no second bump
  * a later completed change on the same task can still bump again

### Adapter / framework integration

* Trellis adapter:
  * add OMV finalize guidance to the finish-work / task completion path
  * ideally provide a wrapper command rather than doc-only instruction
  * add a blocking verification step in finish-work or pre-record path when
    semantic work is complete but no matching finalize record exists
* OpenSpec adapter:
  * expose change classification and completion metadata
  * use same OMV event endpoint
* Codex / Claude adapters:
  * keep instructions thin
  * if host supports hooks, call the same endpoint as fallback only

### Enforcement model

* Positive path:
  * the supported completion command calls finalize automatically
* Negative path:
  * if semantic work is marked complete but no matching finalize record exists,
    completion fails fast
* Reliability comes from:
  * owned completion boundary
  * plus blocking verification
  * with host hooks only as backup

## Decision (ADR-lite)

**Context**: The user wants version updates to be invisible during normal
development, but only after functional work is complete and tested. Pure manual
CLI bumping is too weak; pure hook automation is too unreliable; pure framework
integration is too narrow.

**Decision**:

* Use the hybrid model:
  * framework/state provides semantic context
  * completion wrapper provides the primary workflow boundary
  * host hooks provide fallback invocation only
* First-class primary boundary is Trellis `$finish-work`
* Add `omv event finalize-task` as the single execution endpoint
* Require idempotency and audit recording from the first MVP
* Limit automatic bumping to semantic changes (`bugfix`, `feature`) in MVP

**Consequences**:

* OMV must grow from command-oriented automation to event-oriented automation
* A small new persistence contract is needed for finalization audit / dedupe
* Adapter integration should focus on one shared endpoint instead of
  host-specific bump logic
* "Ensure finalize runs" is reframed as "the supported completion path owns
  finalize", with hooks as backup rather than authority
* Reliable automation requires both:
  * automatic invocation on the happy path
  * and blocking verification when the happy path is bypassed

## Out of Scope (explicit)

* Full git diff understanding to infer semantic change type automatically
* Auto-bumping on every successful test run
* Host-specific deep hook implementations for every agent framework in MVP
* Automatic semantic classification without workflow metadata
