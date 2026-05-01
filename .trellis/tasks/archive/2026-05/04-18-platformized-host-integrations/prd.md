# brainstorm: platformized host integrations and finalize boundary

## Goal

Design the missing product surface that turns OMV host integration from
documentation-only adapter projection into a platformized workflow system.
The design must support multiple future host combinations such as
`codex + trellis`, `codex + openspec`, and `claude + trellis` without
hard-binding the product to any single pair, while still making adapter
installation and completion-boundary automation concrete enough for an MVP.

## What I already know

* The user wants to fill a product gap, not just patch current docs.
* The user explicitly does not want the design to be hard-wired to
  `codex + trellis`.
* The user wants two capabilities to become first-class product behavior:
  * adapter installation into host files
  * automatic completion-boundary invocation of `finalize-task`
* The repo already has adapter inventories for:
  * agent hosts: `codex`, `claude`
  * spec hosts: `trellis`, `openspec`
* The repo already has a shared execution endpoint:
  * `omv event finalize-task`
* `finalize-task` already owns the semantic decision and idempotent
  `bump + sync` flow.
* Current `init` only detects version targets by language manifests and does
  not detect workflow hosts.
* Current init draft/state only models:
  * target languages
  * pre-project strategy
  * timezone
  * build policy
  * locale
* Current adapter installation is independent from `init`.
* Current adapters mainly project version-read/write instructions into host
  files, but they do not model completion boundaries as their own installable
  capability.
* Current Trellis adapter installs:
  * `.trellis/spec/guides/omv-versioning-guide.md`
  * an index snippet into `.trellis/spec/guides/index.md`
* Current Codex adapter installs:
  * `AGENTS.md`
  * `.codex/skills/omv-versioning/SKILL.md`
* Current generated adapter content still instructs hosts to use
  `omv current --json` and `omv bump --json`; it does not yet elevate
  `finalize-task` into the host workflow contract.

## Assumptions (temporary)

* The desired output is a product/MVP design that can lead into implementation,
  not implementation in this brainstorm step.
* The platform abstraction should be user-facing, not only internal Rust traits.
* `codex`, `claude`, `trellis`, and `openspec` should be treated as host
  providers with different capability profiles, not just flat enum values.
* Completion-boundary automation should remain optional and explicit at init
  time, because some users may want host instructions without automatic bumping.

## Open Questions

* None currently. The last blocking MVP behavior decision (`change_type`
  capture fallback) is now resolved.

## Requirements (evolving)

* `omv init` should gain a host-integration step in addition to target setup.
* Host integrations must be modeled in a platformized way that supports new
  combinations without multiplying bespoke pair-specific product flows.
* The user-facing product model should remain layered composition rather than
  pair-specific bundles or raw plugin vocabulary.
* The internal architecture should move toward a provider/plugin kernel.
* The first MVP should use an internal provider registry plus a persisted
  capability-oriented integration state.
* The new integration desired-state and capability-status model should live in a
  separate `.omv/integrations.toml` file rather than extending
  `.omv/adapters.toml`.
* `.omv/integrations.toml` should persist selected state plus the last known
  detection snapshot, rather than only desired state or a full supported-host
  inventory dump.
* The detection snapshot in `.omv/integrations.toml` should stay provider-level
  in the first MVP.
* `omv integrate apply` should always re-detect the workspace before executing
  installs.
* Best-effort apply should still return a non-zero exit when any selected
  capability fails, even if some capabilities succeeded.
* The first MVP provider/capability support matrix should be limited to:
  * `codex` as the supported instruction host
  * `trellis` as the supported spec host and first completion-boundary host
* `claude` and `openspec` should remain outside the first MVP support matrix.
* Providers outside the first MVP support matrix should stay hidden from init UI
  rather than appearing as disabled/upcoming choices.
* The user-facing command surface for post-init workflow operations should move
  to `omv integrate ...` rather than continuing to grow the `adapter` family.
* The existing `omv adapter ...` commands should remain temporarily as
  compatibility aliases during the MVP transition.
* The first MVP `omv integrate ...` command set should stay minimal:
  * `omv integrate status`
  * `omv integrate apply`
* `omv integrate apply` should default to applying all pending or selected
  integrations rather than requiring explicit targeting.
* For non-detected but supported providers, apply behavior should follow a
  provider-specific bootstrap policy rather than one global rule.
* In the first MVP, OMV should bootstrap only lightweight instruction hosts;
  framework-style providers such as Trellis and OpenSpec should require an
  existing host installation before integration apply can mutate them.
* `omv integrate status` should prioritize a provider + capability status
  matrix in MVP output.
* `omv init` should include a mandatory integration review/confirm step before
  any automatic installation attempt.
* The init integration review screen should show:
  * selected providers
  * selected capabilities
  * target files per capability
* Init should display all supported providers, not just detected ones, and mark
  them with detection/recommendation state.
* The MVP integration model should use a medium-grained capability set rather
  than provider-only or file-level capability modeling.
* The product model must distinguish at least two kinds of host capability:
  * host-file adapter projection
  * completion-boundary automation
* The medium-grained capability examples for MVP should look like:
  * `project-instructions`
  * `host-skill`
  * `spec-guide`
  * `spec-index-snippet`
  * `finalize-boundary`
* The MVP capability status model should remain minimal:
  * `selected`
  * `pending`
  * `installed`
  * `failed`
* Failed capability reasons should use a stable reason code plus a
  human-readable display message.
* In init UX, `finalize-boundary` should use a recommended-preselected model
  with explicit user override rather than forced auto-bundling.
* Detection should be automatic where possible, but final enablement should
  remain explicit user choice.
* The system should support hosts with partial capability:
  * read/write guidance only
  * adapter projection only
  * completion-boundary automation capable
* Trellis completion-boundary automation should be the first implemented
  boundary, but not the only boundary assumed by the model.
* In the first MVP, the Trellis completion boundary should hook into the
  existing `/trellis:finish-work` path rather than introducing a new dedicated
  completion command.
* In the first MVP, `task.py finish -> after_finish` lifecycle hooks should not
  be the primary finalize trigger, because that boundary is later than the
  desired "tests completed, ready to finalize version" moment.
* In the first MVP, finalize payload generation should use a hybrid ownership
  model:
  * deterministic fields and invocation wiring come from an OMV-managed helper
  * the agent supplies semantic fields such as `change_type`
* In the first MVP, that helper should be an OMV-native command exposed through
  `.omv/ai/contract.json`, not a generated repo-local script or host-inline
  logic.
* In the first MVP, the helper command should stay generic across providers
  rather than using provider-specific command names.
* In the first MVP, the helper should auto-resolve task identity from active
  Trellis task context by default, while still allowing explicit override.
* In the first MVP, finalize idempotency fingerprint should derive from:
  * task identity
  * boundary identity
  * workspace snapshot hash
* In the first MVP, the workspace snapshot hash should include:
  * `HEAD` commit identity
  * staged and unstaged content deltas
  * untracked file content deltas
* In the first MVP, the helper should normalize OMV-managed version outputs out
  of the snapshot hash so rerunning finalize on the same work does not create a
  false new fingerprint.
* In the first MVP, snapshot-normalization rules should derive from OMV-managed
  metadata plus a small fixed core file set, rather than a hardcoded broad path
  table or user-authored config.
* In the first MVP, that fixed core file set should stay narrow and
  version-bearing only:
  * `.omv/state.toml`
  * `.omv/finalizations.toml`
  * `.omv/skills/README.md`
* In the first MVP, metadata-driven target normalization logic should live
  alongside target sync adapters, with per-language normalization helpers for
  mixed manifest files.
* In the first MVP, generic helper callers should express boundary identity via
  structured fields such as provider + boundary name, rather than via one
  opaque pre-flattened source token.
* In the first MVP, structured helper boundary identity should flatten
  internally to the legacy `source` string when calling `finalize-task` and
  writing audit records, rather than forcing an immediate finalize/audit schema
  redesign.
* In the first MVP, the OMV finalize section should capture `change_type`
  through an explicit choice from the existing enum values:
  * `bugfix`
  * `feature`
  * `refactor`
  * `docs`
  * `chore`
* In the first MVP, if `change_type` is missing at the finalize boundary:
  * the host skill should first try an interactive follow-up prompt so the
    user can choose one enum value
  * OMV should not infer or silently default the value
  * if interactive prompting is unavailable or unresolved, the helper should
    not call `finalize-task`, and the boundary should surface an explicit
    pending/manual-action message instead of faking completion
* In the first MVP, the helper should invoke `finalize-task` only after the
  `/trellis:finish-work` path completes successfully, with helper-derived
  `status=done` and `tests=passed`.
* In the first MVP, the Trellis `finalize-boundary` capability should mutate
  only the active platform-resolved completion surface rather than every
  sibling finish-work representation in the repo.
* In the first MVP, the active completion surface should be mutated via a
  single OMV-managed block rather than whole-file takeover or freeform patching.
* In the first MVP, that OMV-managed block should live as a dedicated final
  checklist section before `Quick Check Flow`, not as a side note or appendix.
* The model should allow future combinations such as:
  * `codex + trellis`
  * `codex + openspec`
  * `claude + trellis`
* The design should avoid pair explosion like `codex+trellis`,
  `codex+openspec`, `claude+trellis` as separate hard-coded first-class types.
* The product should define what init installs immediately versus what remains
  optional post-init operations.
* `omv init` should check whether the working tree is safe for automatic
  integration installation.
* The worktree-safety gate should use a targeted check over integration-affected
  files rather than requiring a fully clean worktree.
* If the working tree contains unrelated pre-existing modifications, init
  should save integration state but prompt the user to run an explicit apply
  step instead of mutating host files immediately.
* If the working tree is safe, init should attempt automatic installation of
  selected integrations.
* After automatic installation, init should instruct the user to review the
  resulting host-file changes.
* If automatic installation fails, init must surface concrete reasons rather
  than a generic failure.
* If some capabilities install successfully and others fail, OMV should keep
  successful installs, record failed capabilities with reasons, and let the
  user retry/apply later.

## Acceptance Criteria (evolving)

* [ ] Current host-integration limitations are summarized from actual repo
      evidence.
* [ ] At least 2 feasible product models for platformized host integration are
      compared.
* [ ] The recommended model preserves combinability across agent/spec hosts.
* [ ] The recommended model explicitly separates adapter projection from
      completion-boundary automation.
* [ ] The MVP recommendation identifies what is first-wave supported and what is
      intentionally deferred.
* [ ] The init product flow is described clearly enough to implement later.
* [ ] The ADR clearly separates the user-facing product model from the internal
      architecture model.
* [ ] The MVP depth for the provider/plugin kernel is explicit and stable.
* [ ] The init/apply behavior for clean versus dirty worktrees is explicit.
* [ ] Partial install behavior is explicit and capability-granular.
* [ ] The worktree-safety rule is scoped to integration-affected files.
* [ ] The chosen capability granularity is explicit and implementable.
* [ ] The boundary-selection UX is explicit and non-binding.
* [ ] The integration-state file boundary is explicit.
* [ ] The post-init command surface is explicit.
* [ ] The transition plan for legacy `adapter` commands is explicit.
* [ ] The MVP `integrate` command set is explicit and minimal.
* [ ] Init provider selection supports non-detected but supported providers.
* [ ] The default scope of `integrate apply` is explicit.
* [ ] The bootstrap policy for non-detected providers is explicit.
* [ ] MVP bootstrap rules for framework providers are explicit.
* [ ] The `integrate status` output priority is explicit.
* [ ] The init review/confirm step is explicit.
* [ ] The init review screen detail level is explicit.
* [ ] The MVP capability status model is explicit and minimal.
* [ ] Failed capability reasons use a stable code model.
* [ ] The primary Trellis finalize boundary for MVP is explicit.
* [ ] Finalize payload ownership between helper and agent is explicit.
* [ ] The native finalize-helper command surface is explicit.
* [ ] The generic-versus-provider-specific helper decision is explicit.
* [ ] Task identity resolution for the helper is explicit.
* [ ] Fingerprint composition for helper-driven finalize is explicit.
* [ ] Snapshot-hash normalization or exclusion rules are explicit.
* [ ] The source of snapshot-normalization rules is explicit.
* [ ] The MVP fixed-core normalization set is explicit.
* [ ] The placement of target normalization logic is explicit.
* [ ] Helper boundary-identity input shape is explicit.
* [ ] Helper-to-finalize/audit source mapping is explicit.
* [ ] Helper trigger timing relative to finish-work success/failure is explicit.
* [ ] The MVP finalize-boundary file target set is explicit.
* [ ] The active-surface mutation mode is explicit.
* [ ] The finalize block placement inside finish-work is explicit.
* [ ] Change-type capture inside the finalize block is explicit.
* [ ] Missing change-type behavior is explicit.
* [ ] The integration-state persistence boundary is explicit.
* [ ] The detection snapshot granularity is explicit.
* [ ] The re-detect rule for `integrate apply` is explicit.
* [ ] Command exit semantics for mixed apply results are explicit.
* [ ] The first MVP support matrix is explicit.
* [ ] Init visibility rules for non-MVP providers are explicit.

## Definition of Done

* Product boundary is explicit.
* MVP versus future platform scope is explicit.
* Key abstractions are named clearly enough to map into code later.
* Remaining MVP behavior decisions are either explicitly locked or explicitly
  deferred, rather than left ambiguous.

## Research Notes

### Current repo evidence

* `src/ui/discovery.rs` only auto-detects language/manifests.
* `src/ui/state/draft.rs` only stores target/config setup and has no host
  integration state.
* `src/app/mod.rs::run_init` builds init from language discovery only.
* `src/core/adapter.rs` separates hosts into two flat enums:
  * `AgentAdapter`
  * `SpecAdapter`
* `src/adapter.rs` installs host file projections but does not yet model
  completion boundaries as an installable/install-status capability.
* `src/cli/mod.rs` already separates runtime workflow triggers under
  `event ...` from installation operations under `adapter ...`.
* The current `event` namespace contains only `finalize-task`, which expects a
  fully populated payload from the caller.
* The current finalize contract and persisted audit both model boundary origin
  as a single `source: String` field, with examples like
  `trellis-finish-work`.
* Trellis already persists active-task context in `.trellis/.current-task`, and
  task directories contain `task.json` with stable task metadata such as `id`.
* There is no existing structured store for "completion event id" or "tests
  passed record" that a finalize helper could reuse directly.
* Trellis already has a shared `run_git(...)` wrapper, and current status
  tooling already uses `git status --short` to inspect worktree state.
* `execute_bump` persists `.omv/state.toml` and then calls `execute_sync`,
  which mutates version-managed target files and OMV-managed skill artifacts.
* Existing OMV metadata already identifies several managed output classes:
  * `.omv/targets.toml` identifies enabled manifest/runtime-export targets
  * adapter registry records installed host-file targets when adapters are used
  * `.omv/state.toml` and `.omv/finalizations.toml` are fixed core OMV files
* Current repo evidence suggests only a narrow subset of `.omv/**` is
  version-bearing during normal bump/finalize flows:
  * `.omv/state.toml`
  * `.omv/finalizations.toml`
  * `.omv/skills/README.md`
  while `.omv/ai/*` and `.omv/skills/bump-guidance.md` are effectively static
  in the current design.
* Target outputs are not uniform:
  * runtime export files are pure generated version views
  * manifest files are mixed user-owned files with language-specific OMV-managed
    version fragments
* Current target sync code already encodes those language-specific write rules
  in per-language modules under `src/sync/*`.
* Trellis completion surfaces are platform-resolved today:
  * task context uses `adapter.get_trellis_command_path("finish-work")`
  * for Codex, that resolves to `.agents/skills/finish-work/SKILL.md`
  * sibling docs such as `.opencode/commands/trellis/finish-work.md` exist, but
    they are not the active Trellis completion surface for Codex.
* Existing adapter installation already supports managed-block projection into
  user-owned host files, which is a plausible mutation primitive for the active
  finish-work surface.
* The active Codex Trellis finish-work file has a stable section structure:
  * checklist sections
  * `Quick Check Flow`
  * `Common Oversights`
  * `Relationship to Other Commands`
  * `Core Principle`
* Existing finalization semantics accept a fixed `change_type` enum only:
  * `bugfix`
  * `feature`
  * `refactor`
  * `docs`
  * `chore`
* Current finalization decision rules only bump on:
  * `bugfix`
  * `feature`
  and treat:
  * `refactor`
  * `docs`
  * `chore`
  as no-op audit outcomes.
* Current Trellis and Codex adapter artifacts are guidance-first and still
  instruct direct `current`/`bump` usage.
* `src/app/mod.rs::execute_finalize_task` already provides the shared, reusable
  event endpoint that future boundaries should call.
* `.omv/ai/contract.json` is already the machine-readable automation contract,
  but it currently exposes only `current`, `bump`, and adapter-management
  commands, not finalize-boundary helper entrypoints.
* `task.json.relatedFiles` exists in schema, but current task creation/storage
  paths default it to empty and do not maintain it as a reliable canonical file
  scope for fingerprinting.
* Because finalize currently performs bump + sync inline, a naive workspace hash
  would change on rerun even when the underlying task work did not change.

### Product gap summary

* There is no host-integration step in `omv init`.
* There is no automatic host detection for Codex / Claude / Trellis / OpenSpec.
* There is no persisted host-integration selection model separate from
  `.omv/targets.toml`.
* There is no first-class product notion of "completion boundary".
* There is no abstraction for "this host can project docs but not install an
  executable boundary".
* Current adapter registry schema stores installation metadata for file targets,
  but not provider capabilities or boundary-install status.
* The current adapter surface risks pair explosion if future work keeps adding
  ad hoc host combinations at the feature level.
* There is no machine-readable helper contract for a host to obtain or invoke a
  finalize-boundary entrypoint.

### Feasible approaches here

**Approach A: Layered composition model** (Recommended)

* How it works:
  * Treat hosts as providers in separate layers:
    * agent host
    * spec/workflow host
    * completion-boundary host
  * `omv init` detects available providers and lets the user select from each
    layer.
  * OMV composes the selected providers into one integration plan.
  * `codex + trellis` is a composition result, not a bespoke type.
* Pros:
  * avoids pair explosion
  * keeps user-visible combinations flexible
  * naturally separates projection capability from boundary capability
  * clean MVP path: first implement Trellis boundary, keep others projection-only
* Cons:
  * requires a new persisted integration model
  * slightly more complex init UX than today's language-only flow

**Approach B: Provider plugin model**

* How it works:
  * Introduce a generic provider/plugin abstraction where every host declares:
    * detection rules
    * file projection targets
    * supported capabilities
    * optional completion-boundary installer
  * Init renders providers dynamically from registry metadata.
* Pros:
  * strongest long-term platform story
  * easiest future extensibility for third-party host packs
* Cons:
  * likely too much infrastructure for the next MVP
  * higher implementation and testing surface before user value shows up

**Approach C: Bundle-first model with advanced override**

* How it works:
  * Expose first-wave bundles such as `codex + trellis` and
    `claude + trellis`, but internally map them onto shared capabilities.
  * Advanced mode lets users pick providers individually.
* Pros:
  * simplest UX for near-term adoption
  * hides complexity from users initially
* Cons:
  * product language still nudges toward pair-centric thinking
  * easy to regress into hard-coded bundle proliferation

## Expansion Sweep

### Future evolution

* Later hosts may provide only detection + projection, without hook/boundary
  support.
* Some future hosts may own the completion boundary better than Trellis, so the
  model cannot assume "spec host == boundary host".

### Related scenarios

* `omv adapter install` and `omv init` should not diverge into different
  integration concepts; init should likely produce the same install plan that a
  later manual install can re-run.
* `omv adapter refresh` will need to understand the same platform model so
  integrations remain reproducible.
* Trellis already exposes two completion-adjacent boundaries:
  * `/trellis:finish-work`
  * `task.py finish -> after_finish`
  They should remain separate concepts because they happen at different points
  in the workflow.

### Failure and edge cases

* A host may be detected but already contain unmanaged files; init must preview
  conflicts before installing.
* A selected host combination may include only one boundary-capable provider;
  the product should explain which component will actually call
  `finalize-task`.
* A user may want adapter projection without automatic finalize invocation.

## Decision (ADR-lite)

**Context**: OMV needs to evolve from static host instructions into a
platformized integration system. The immediate pressure is to install adapters
into host files and let Trellis automatically call `finalize-task`, but the
product must stay extensible across future host combinations.

**Decision**:

* Lock the user-facing product model to layered composition.
* Lock the long-term internal architecture direction to provider/plugin.
* For MVP depth, implement:
  * internal provider registry
  * persisted capability-oriented integration state redesign
* Model completion boundary as a separate capability, not as an accidental
  side-effect of a specific host pair.
* Use Trellis as the first implemented completion-boundary provider in MVP.
* Hook the first Trellis completion boundary into the existing
  `/trellis:finish-work` path instead of introducing a separate completion
  command.
* Use a hybrid finalize-payload model in MVP:
  * OMV-managed helper logic derives deterministic fields and owns invocation
  * the agent supplies semantic classification such as `change_type`
* Expose the helper as an OMV-native CLI contract rather than a generated
  script so host integrations keep consuming one canonical automation surface.
* Keep the helper command generic across providers so new boundary-capable
  hosts can reuse the same contract shape.
* Auto-resolve task identity from active Trellis context by default, with
  explicit overrides for nonstandard or scripted cases.
* Build finalize idempotency around task identity + boundary identity +
  workspace snapshot hash rather than task id alone.
* Include `HEAD` plus staged/unstaged/untracked content deltas in that
  snapshot hash rather than path/status listing only.
* Normalize OMV-managed version outputs out of that snapshot hash so finalize's
  own bump/sync side effects do not create false new fingerprints.
* Derive normalization rules from OMV-managed metadata plus a small fixed core
  file set rather than from a broad hardcoded path table.
* Keep the fixed core normalization set narrow and version-bearing only.
* Co-locate target normalization logic with per-language sync adapters so
  manifest normalization follows the same language-specific ownership boundary
  as target writes.
* Accept structured boundary identity at the generic helper layer so provider
  and boundary naming stay platformizable.
* Flatten structured boundary identity back into the legacy `source` string
  internally so MVP can reuse the existing finalize and audit schema.
* Trigger finalize helper only after the completion path succeeds, so MVP keeps
  the bump boundary aligned with "done + tests passed" rather than broadening
  finalization into failure/no-op bookkeeping.
* Mutate only the active platform-resolved completion surface in MVP, not every
  sibling Trellis representation checked into the repo.
* Mutate that active surface through one OMV-managed block so refresh/reapply
  stays idempotent.
* Place that OMV-managed block as a dedicated final checklist section so it
  reads as part of completion, not as an afterthought.
* Require `change_type` as an explicit enum choice at the human/agent boundary
  rather than relying on freeform coercion or inference in MVP.
* When `change_type` is missing, prefer interactive recovery inside the host
  skill over hard failure, but never substitute an inferred/default value.
* Keep Codex/Claude/OpenSpec/Trellis as providers that can advertise different
  capabilities rather than embedding pair-specific business logic.
* For init behavior:
  * save integration state during init
  * check worktree cleanliness after setup
  * use a targeted safety check on files the selected integration plan would
    mutate
  * if the tree is safe, attempt automatic installation immediately
  * if unrelated modifications already exist, skip mutation and instruct the
    user to run apply explicitly
  * after auto-install, prompt the user to inspect resulting changes
  * on failure, surface explicit install reasons
* For install failure semantics:
  * use best-effort installation
  * record per-capability install status
  * preserve successful installs
  * surface explicit reasons for failed capabilities

**Consequences**:

* OMV needs a new persisted integration state separate from target state.
* `init` becomes a two-domain setup flow:
  * version targets
  * workflow integrations
* Adapter content generation and boundary automation installation should become
  related but separable operations.
* Trellis lifecycle hooks remain available for future enhancement or backstop
  logic, but they are not the primary finalize boundary in MVP.
* OMV now needs one standard helper entrypoint that host adapters can reuse
  instead of duplicating finalize-call assembly in host-specific text.
* Missing `change_type` is now an interaction-state concern for the host skill
  layer, not a place where OMV should invent semantic meaning on behalf of the
  user.

## Latest Discussion

* The layered-composition product model and provider/plugin architecture
  direction are now agreed.
* The user chose MVP depth option 2:
  * internal provider registry
  * persisted capability-oriented integration state redesign
* The user asked whether choosing layered composition for MVP and moving to a
  provider-platform later would be difficult.
* Current answer direction:
  * migration difficulty is highly sensitive to how MVP is implemented
  * if MVP keeps pair-specific enums and install branches, later migration is
    expensive
  * if MVP keeps layered UX but introduces internal provider descriptors and
    capability metadata now, later migration is moderate and controlled
* The user refined init behavior to a guarded auto-install model:
  * save integration state first
  * attempt auto-install only when the worktree is safe
  * otherwise instruct the user to apply explicitly
  * always show post-install review guidance
  * always show concrete failure reasons
* The user chose partial failure semantics:
  * best-effort installation
  * capability-level success/failure recording
  * later retry/apply for failed capabilities
* The user chose worktree gating option 2:
  * targeted safety check on integration-affected files
* The user chose capability granularity option 2:
  * medium-grained capability model
* The user chose boundary-selection option 3:
  * recommended/preselected boundary capability
  * explicit user override
* The user chose integration-state persistence option 2:
  * separate `.omv/integrations.toml`
* The user chose command-surface option 2:
  * introduce `omv integrate ...`
* The user chose transition option 2:
  * keep `omv adapter ...` temporarily as compatibility aliases
* The user chose MVP subcommand option 1:
  * `omv integrate status`
  * `omv integrate apply`
* The user chose init-provider option 2:
  * show all supported providers
  * mark detected/recommended state in UI
* The user chose apply-scope option 1:
  * `omv integrate apply` defaults to all pending/selected integrations
* The user chose non-detected-provider option 3:
  * provider-specific bootstrap policy
* The user chose framework-bootstrap option 2:
  * bootstrap only lightweight instruction hosts
  * require existing framework hosts for Trellis/OpenSpec-style providers
* The user chose `integrate status` option 2:
  * provider + capability status matrix
* The user chose init-review option 2:
  * mandatory integration review/confirm step before auto-install
* The user chose init-review-detail option 2:
  * providers + capabilities + target files
* The user chose capability-status option 1:
  * `selected / pending / installed / failed`
* The user chose failure-reason option 2:
  * stable reason code + display message
* The user chose integrations-persistence option 2:
  * selected state + last known detection snapshot
* The user chose detection-snapshot option 1:
  * provider-level only
* The user chose apply-redetect option 1:
  * always re-detect the workspace before executing installs
* The user chose apply-exit-semantics option 2:
  * non-zero failure if any selected capability fails
* The user chose MVP support-matrix option 1:
  * `codex` + `trellis` only
* The user chose non-MVP-provider visibility option 1:
  * hide them entirely from init UI
* The user is leaning to hook option 1 for MVP:
  * patch the existing `/trellis:finish-work` path
* Repo inspection shows Trellis also has `task.py finish -> after_finish`
  lifecycle hooks available, but that boundary is later than the desired
  completion moment for version finalization.
* The user chose finalize payload option 3:
  * hybrid ownership
  * helper derives deterministic fields and performs the call
  * agent supplies semantic fields such as `change_type`
* The user chose helper form option 1:
  * use an OMV-native helper command
  * expose it in `.omv/ai/contract.json`
* Repo inspection shows `.omv/ai/contract.json` is already the canonical
  machine-readable automation contract, but it does not yet expose any
  finalize-boundary helper command.
* The user chose helper command-shape option 1:
  * generic event helper
  * provider stays as data, not as command-name branching
* Repo inspection shows the CLI already distinguishes runtime workflow actions
  (`event ...`) from install-time operations (`adapter ...`), which makes
  `event` the natural namespace candidate for a finalize helper.
* Repo inspection also shows Trellis already has active-task context persisted
  in `.trellis/.current-task`, with stable task metadata available in
  `task.json`.
* The user chose task-identity option 1:
  * auto-resolve from active task context
  * keep explicit override support
* Repo inspection shows there is no existing structured "completion event id"
  or persisted test-result record for the helper to reuse.
* The user chose fingerprint option 1:
  * derive from task identity + boundary identity + workspace snapshot hash
* Repo inspection shows Trellis already has shared git execution utilities, but
  task-scoped file metadata is not maintained well enough to be the sole
  fingerprint scope.
* The user chose snapshot-input option 1:
  * `HEAD` commit plus staged/unstaged/untracked content deltas
* Repo inspection confirms `omv finalize-task` currently performs bump + sync
  inline, so OMV-managed files change during finalize and would perturb a naive
  rerun hash.
* The user chose normalization option 1:
  * normalize OMV-managed version outputs out of the snapshot hash
* Repo inspection shows OMV already has managed-output metadata in targets and
  adapter registries, plus fixed core OMV files, which creates a realistic
  metadata-driven alternative to hardcoded path tables.
* The user chose normalization-rule-source option 1:
  * derive from OMV-managed metadata plus a small fixed core file set
* Repo inspection suggests the current version-bearing fixed core set is narrow:
  `.omv/state.toml`, `.omv/finalizations.toml`, and `.omv/skills/README.md`
  are the main candidates, while `.omv/ai/*` is effectively static today.
* The user chose fixed-core-set option 1:
  * only version-bearing OMV core files
* Repo inspection shows target outputs are a mixed case:
  runtime exports are pure generated views, while manifests are mixed
  user-owned files with language-specific OMV-managed fragments.
* The user chose target-normalization-placement option 1:
  * keep per-language normalization helpers alongside target sync adapters
* Repo inspection shows helper identity is still missing one explicit contract:
  boundary identity is currently represented only implicitly through examples
  like `trellis-finish-work`.
* The user chose helper-boundary-identity option 1:
  * structured fields such as provider + boundary name
* Repo inspection shows current finalize and audit contracts still accept only
  one flat `source` string, so structured helper identity still needs a mapping
  decision at the finalize boundary.
* The user chose helper-source-mapping option 1:
  * flatten structured helper identity into the legacy `source` string
    internally
* The user chose helper-trigger option 1:
  * invoke helper only after finish-work passes
  * helper supplies `status=done` and `tests=passed`
* Repo inspection shows Trellis completion surfaces are platform-resolved today:
  for Codex, the active surface is `.agents/skills/finish-work/SKILL.md`, while
  `.opencode/commands/trellis/finish-work.md` is a sibling representation.
* The user chose finalize-boundary-target-set option 1:
  * mutate only the active platform-resolved completion surface
* Repo inspection shows the installer already has a managed-block projection
  primitive for user-owned host files, which is the natural safe mutation
  candidate for the active finish-work surface.
* The user chose active-surface-mutation option 1:
  * inject/update one OMV-managed block inside the active finish-work file
* Repo inspection shows the active Codex Trellis finish-work file has a stable
  section structure, so block placement is now the highest-value remaining UX
  decision.
* The user chose finalize-block-placement option 1:
  * a dedicated final checklist section before `Quick Check Flow`
* Repo inspection shows helper payload construction still depends on one human
  semantic field, `change_type`, and current finalization only accepts a fixed
  enum.
* The user chose change-type-capture option 1:
  * require an explicit choice from the existing enum values
* The user chose missing-change-type behavior as a modified option 1:
  * do not silently default or infer
  * let the skill ask the user to choose from the enum when the value is
    missing
  * if the interaction cannot be completed, leave OMV finalization pending
    with an explicit actionable message instead of forcing a fake success or
    defaulting to `chore`

## Migration Analysis

### Is provider-platform more future-proof?

Yes, but only at the internal architecture layer.

* A unified provider model is better for:
  * adding new hosts without growing match branches
  * expressing capability differences per host
  * eventually supporting external or semi-external provider packs
* It is not automatically better as the first user-facing product model, because
  it exposes infrastructure complexity before OMV has enough host diversity to
  justify that complexity.

### Clarified conclusion

* For long-term evolution across more agents and frameworks, the
  provider/plugin route is the stronger **internal architecture**.
* For near-term product usability, layered composition is the stronger
  **user-facing init model**.
* The agreed combined strategy is:
  * user-facing product model: layered composition
  * internal implementation model: provider descriptors / capability matrix
  * later evolution path: full provider/plugin platform if host diversity and
    maintenance pressure justify it
* The agreed MVP depth is:
  * internal provider registry
  * capability-oriented persisted integration state

### How hard is "MVP with layered composition now, provider-platform later"?

**Low-to-moderate** if MVP does these things now:

* keep layered composition as the user-facing init model
* introduce internal provider descriptors now instead of growing hard-coded
  pair logic
* persist integration state in a capability-oriented shape rather than
  `agent/spec pair` assumptions
* treat completion boundary as a capability record, not as Trellis-specific
  one-off state

**High** if MVP does these things:

* keeps `AgentAdapter` and `SpecAdapter` as the dominant long-term runtime
  abstraction
* stores only host-kind/name install targets without capability metadata
* bakes `codex + trellis` rules directly into init flow and persistence
* couples finalize-boundary installation to one specific adapter path

### Practical migration rule

The safest path is:

* **product layer now**: layered composition
* **implementation layer now**: internal provider descriptors + capability
  matrix
* **external platform layer later**: true provider/plugin system if ecosystem
  growth justifies it

That keeps the future provider-platform migration incremental rather than a
full rewrite.

## Out of Scope

* Implementing the chosen design in this brainstorm step.
* Full third-party plugin marketplace or remote-loaded providers.
* Automatic semantic change classification beyond the existing
  `finalize-task` contract.
* Deep hook installation for every supported host in the first MVP.

## Technical Notes

* New task: `.trellis/tasks/04-18-platformized-host-integrations`
* Relevant files inspected:
  * `src/app/mod.rs`
  * `src/ui/discovery.rs`
  * `src/ui/state/draft.rs`
  * `src/ui/runtime.rs`
  * `src/core/adapter.rs`
  * `src/adapter.rs`
  * `.agents/skills/finish-work/SKILL.md`
  * `.omv/ai/adapters/*`
