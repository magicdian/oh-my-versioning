# Smooth OMV Version Source and AI Spec Flow

## Goal

Make `omv` feel like the natural, frictionless version authority in both code
and AI-assisted workflows, so humans and models stop treating native manifests,
spec artifacts, or chat memory as version truth and instead route version
changes through one canonical OMV contract.

## What I already know

* The user wants two things:
  * smoother code references to the `omv` library / OMV version truth
  * smoother AI/spec workflow behavior so models update versions the "right"
    way automatically
* The repo already defines `.omv/` as the single source of truth in
  `README.md` and `.trellis/spec/backend/database-guidelines.md`
* Current mutable version truth lives in `.omv/state.toml`:
  * `logical_date`
  * `build_number`
  * `last_issued_version`
* Version math is centralized in `src/core/versioning/engine.rs`
* `omv bump` updates `.omv/state.toml`, then syncs manifests/runtime exports via
  target adapters in `src/sync/*`
* Current AI guidance exists, but only as lightweight generated files:
  * `.omv/skills/README.md`
  * `.omv/skills/bump-guidance.md`
* The generated guidance currently says "use `omv bump`" but does not yet
  establish a stronger cross-tool contract
* Trellis-managed AI surfaces such as `AGENTS.md`, `.agents/skills/*`, and
  `.opencode/*` belong to the host development framework, not to OMV's product
  surface
* Those Trellis surfaces are still useful as reference material for how agent
  hook/instruction injection can work, but they should not be treated as OMV's
  own artifact model
* The user's concrete current OMV-facing artifact shape is much narrower:
  * `.omv/config.toml`
  * `.omv/state.toml`
  * `.omv/targets.toml`
  * native manifest files such as `Cargo.toml` and `CMakeLists.txt`
  * runtime export files such as `include/omv_version.h` and
    `src/generated/version.rs`
* There is currently no `openspec/` or `.specify/` directory in this repo, so
  OpenSpec / Spec Kit integration is still future-facing rather than already
  installed locally
* Prior product notes already anticipated this direction:
  * `.trellis/tasks/archive/2026-04/04-13-omv-cli-foundation/prd.md`
    explicitly calls out future OpenSpec / Spec Kit integration
  * `.trellis/tasks/archive/2026-04/04-13-omv-target-sync-skills/prd.md`
    already required `.omv/skills` to guide AI toward `omv bump`
* External workflow conventions from official docs:
  * OpenSpec initializes an `openspec/` tree, writes a managed root
    `AGENTS.md`, uses `openspec/project.md` for project-level conventions, and
    provides `/openspec-*` workflows
  * Spec Kit uses `.specify/memory/constitution.md` as project steering for
    `/speckit.constitution`, then drives `/speckit.specify`, `/speckit.plan`,
    `/speckit.tasks`, and `/speckit.implement`
  * The broader `AGENTS.md` convention is designed as a predictable,
    automatically-read place for agent instructions

## Assumptions (temporary)

* We want a brownfield-friendly design that works in existing repos, not only
  greenfield scaffolding
* The desired end state is not "AI remembers better", but "AI is guided by
  concrete repository contracts and OMV-owned entrypoints"
* Native manifests such as `Cargo.toml`, `pyproject.toml`, and `go.mod` should
  remain synchronized outputs, not decision-making inputs
* OpenSpec / Spec Kit integration should preferably reuse one OMV-owned contract
  instead of maintaining separate duplicated instructions per framework
* Trellis/OpenCode integration should be treated as an example integration, not
  the center of the OMV architecture

## Open Questions

* None at the current framework-definition stage

## Requirements (evolving)

* Define the canonical OMV version authority clearly enough that code, specs,
  and AI tools all point at the same truth
* Preserve `.omv` as the only truth source for managed version data
* Avoid any workflow where AI derives the "next version" by reading native
  manifests directly
* Define how code should reference the current managed version without creating
  a second truth source
* Define how OMV-owned version rules should be exposed to:
  * code inside managed projects
  * AI/spec workflows such as OpenSpec and Spec Kit
  * optional host-framework integrations such as Trellis/OpenCode
* Assume arbitrary `.omv/*.md` files are **not** auto-loaded by AI tools unless
  a supported entrypoint explicitly references or injects them
* Support an adapter architecture with at least two projection categories:
  * agent adapter: inject OMV awareness into agent/IDE skill or instruction
    entrypoints
  * spec adapter: inject OMV version-management rules into spec framework
    governance/spec surfaces
* First implementation wave should target:
  * agent adapters: Claude Code, Codex
  * spec adapters: OpenSpec, Trellis
* Make the integration durable across tool updates so custom version guidance is
  less likely to be lost or silently drift
* Keep the design compatible with current Trellis/OpenCode setup without making
  that setup a hard dependency
* Leave room for stronger automation later, such as structured JSON output,
  version drift detection, or release hooks
* Deliver both read and write automation surfaces in the first structured
  contract iteration:
  * `omv current`
  * `omv bump`
* Keep `omv bump` as the write path that also performs sync by default
* Define stdout/stderr behavior for automation so scripts and AI tools can
  consume output deterministically
* Support both:
  * `--json` as the ergonomic shortcut
  * `--output json` as the extensible general form
* In structured output mode, emit machine-readable JSON for both success and
  failure cases while still using exit codes to indicate failure
* Decide whether OMV should generate a first-class AI/spec integration surface
  under `.omv/` in addition to the CLI contract
* Use one shared JSON envelope for structured commands rather than letting each
  command invent its own top-level contract
* Ship adapters as installable, user-invoked integrations rather than as
  always-on direct file ownership in V1
* Use `managed mirrored summaries` as the preferred spec-adapter shape
* On Unix-like systems, consider symlink-backed installations where the host
  framework and filesystem support them cleanly
* Provide a Windows-safe fallback for any symlink-dependent install path
* Default install backend strategy should be `auto`:
  * prefer `link` when the environment and target support it
  * automatically fall back to `materialize` otherwise
* Expose adapters through one unified command family:
  * `omv adapter install`
  * `omv adapter refresh`
  * `omv adapter list`
  * `omv adapter status`
* Distinguish adapter categories through explicit flags instead of positional
  ambiguity, for example:
  * `--agent codex`
  * `--agent claude`
  * `--spec openspec`
  * `--spec trellis`
* Allow a single install invocation to include both agent and spec adapters when
  useful, for example:
  * `omv adapter install --agent codex --spec openspec`
* Persist adapter installation state through a hybrid model:
  * dedicated OMV registry metadata
  * plus host-file markers/source metadata for visibility and recovery

## Acceptance Criteria (evolving)

* [ ] The design names one canonical OMV contract that all version-aware tools
      should reference
* [ ] The design explains how code reads or imports version information without
      making native manifests authoritative
* [ ] The design explains how AI agents are steered to use `omv bump` instead of
      editing manifest versions directly
* [ ] The design defines how `omv current` and `omv bump` expose structured,
      automation-safe results
* [ ] The design distinguishes OMV product surfaces from host-framework
      integration surfaces
* [ ] The design covers future OpenSpec / Spec Kit usage and optionally explains
      how Trellis-like hosts can hook into the same contract
* [ ] The design identifies at least one drift/failure mode and how the workflow
      should guard against it
* [ ] The MVP boundary is explicit: what we will do now vs later

## Definition of Done (team quality bar)

* Requirements and MVP boundary are explicitly confirmed
* Recommended approach and trade-offs are recorded
* Relevant code-spec targets for implementation are identified before any code
  change starts
* If implementation follows, tests/docs are updated and lint/typecheck pass

## Out of Scope (explicit)

* Fully implementing every OpenSpec / Spec Kit file template before the design is
  settled
* Solving release automation, changelog generation, and package publishing in
  the same task
* Supporting every AI framework equally in the first iteration
* Replacing existing Trellis workflow with OpenSpec or Spec Kit
* Treating Trellis-managed files as part of the OMV artifact schema

## Research Notes

### What similar tools do

* OpenSpec and Spec Kit are useful comparison points because they both have a
  project-governance layer plus downstream task/implementation flows. That makes
  them good targets for projecting OMV rules into later, without making them the
  source of truth themselves.
* Trellis/OpenCode demonstrates that host frameworks can inject project context
  through hooks. The reusable lesson is the injection pattern, not the exact
  file layout.

### Constraints from our repo/project

* Current OMV backend contracts already insist on one version engine and one
  `.omv` truth source
* Current AI guidance is too light to prevent drift on its own
* The actual observable OMV product surface today is centered on `.omv`,
  manifest sync, and runtime export files
* Runtime export generation already gives code a natural read path for the
  current version without requiring direct `.omv` parsing in app code
* The repo already has custom session-start injection and Trellis command
  surfaces, so any optional host integration should compose with these rather
  than fight them
* OpenSpec and Spec Kit are not initialized in this repo yet, so we can design a
  clean integration seam before carrying legacy baggage

### Feasible approaches here

**Approach A: One OMV contract, many projections** (Recommended)

* How it works:
  * introduce one OMV-owned, machine-readable or strongly-structured contract
    for version workflow
  * define clear separation between:
    * version truth in `.omv/state.toml`
    * code consumption through generated runtime exports
    * automation/spec consumption through a stable CLI/API surface such as
      `omv current --json`
  * project optional AI/spec integrations from that contract later
* Pros:
  * one place to update the rule
  * best fit for future OpenSpec / Spec Kit adoption
  * lowers instruction drift across agent ecosystems
* Cons:
  * requires designing a small contract format and generator/update path

**Approach B: Repo instruction first**

* How it works:
  * keep `.omv` as truth
  * document rules in repo instructions first
  * later hand-write similar guidance into OpenSpec / Spec Kit
* Pros:
  * fastest to roll out
  * minimal backend changes
* Cons:
  * high duplication risk
  * weaker portability across tools
  * likely to drift as tool configs evolve

**Approach C: CLI/library first**

* How it works:
  * expose a stable read/write automation surface from OMV itself, such as
    `omv current --json`, `omv bump --json`, or a reusable crate API
  * treat generated runtime export files as the code-facing read surface for
    managed projects
  * tell AI/spec frameworks to call the structured OMV surface directly
* Pros:
  * strong automation story
  * easiest for future scripts and agents to consume
  * aligns well with current artifact shape
* Cons:
  * still needs project-level instructions, otherwise AI may not choose to use
    the API
  * does not by itself solve instruction duplication across OpenSpec / Spec Kit

## MVP Framework Proposal

### Canonical Layers

**1. Truth layer**

* `.omv/state.toml` remains the only mutable source of truth for managed version
  state
* version math continues to live in `src/core/versioning/engine.rs`

**2. Code-read layer**

* application code should read generated runtime exports, not parse `.omv`
  directly
* examples in current codebase:
  * Rust: `src/generated/version.rs`
  * C/C++: `include/omv_version.h`
* this avoids introducing a second mutable truth source while keeping app code
  ergonomics simple

**3. Automation layer**

* `omv current` becomes the structured read interface for tools, AI, and scripts
* `omv bump` becomes the structured write interface for tools, AI, and scripts
* structured mode should expose machine-usable fields rather than localized text
* structured output should be accessible through both:
  * `--json`
  * `--output json`
* structured output mode should return JSON on both success and failure

**4. Adapter layer**

* OMV should support thin projections into external ecosystems rather than
  making those ecosystems part of the truth model
* Adapters split into two types:
  * `agent adapters`
    * purpose: help an IDE/agent session discover that OMV exists and know which
      commands/docs to call
    * examples: Codex skills, AGENTS entrypoints, IDE hook snippets
  * `spec adapters`
    * purpose: encode version-management rules into project governance and change
      workflows
    * examples: OpenSpec project/spec files, Spec Kit constitution/rule files

### Initial Contract Direction

**Recommended first contract shape**

* Add a structured output mode for both `current` and `bump`
* Make the structured output explicitly versioned from day one
* Keep human-readable localized text as the default operator mode
* Keep machine-readable output unlocalized and stable
* Pair the CLI contract with OMV-owned integration artifacts under `.omv/`, but
  treat those artifacts as canonical content rather than auto-discovered magic
* Support two equivalent entry shapes for JSON:
  * `omv <command> --json`
  * `omv <command> --output json`
* In JSON mode, success and failure should both be machine-readable
* Use one shared envelope for structured commands, especially `current` and
  `bump`

### Candidate Structured Fields

**Shared envelope**

```json
{
  "ok": true,
  "contract_version": "1",
  "command": "current",
  "data": {},
  "error": null
}
```

Failure shape:

```json
{
  "ok": false,
  "contract_version": "1",
  "command": "bump",
  "data": null,
  "error": {
    "code": "future_stored_date",
    "message": "stored logical date is in the future",
    "details": {}
  }
}
```

**`omv current`**

* contract/schema version
* current issued version
* logical date
* build number
* build policy
* version output mode
* last time source
* resolved `.omv` root
* enabled targets summary
* runtime export/manifest target metadata if useful

**`omv bump`**

* contract/schema version
* previous issued version
* next issued version
* logical date
* build number
* time source used for this bump
* sync summary (`synced`, `skipped`)
* possibly changed target list or touched artifact metadata

### CLI Boundary Notes

* Current CLI parser only supports command selection plus `--locale` and
  `--no-ntp`
* Current success path returns localized strings through `AppOutput`
* Current error path prints localized stderr in `src/main.rs`
* Therefore structured automation likely requires introducing a new response
  model rather than only adding a formatting helper at the edge
* `--json` should likely behave as sugar for `--output json`
* JSON-mode failure behavior will require a structured error serialization path,
  not only localized `render_error(...)`

### Integration Projection Model

* Canonical OMV-owned artifacts live under `.omv/`
* Host tools should not duplicate OMV rules in full; they should project or
  reference the canonical OMV contract through their supported entrypoints
* Practical implication:
  * `.omv/contract.json` or similar is for machine consumption
  * `.omv/instructions.md` or similar is for human/model consumption
  * agent entrypoints such as `AGENTS.md`, Codex skills, or IDE hook snippets
    should contain a short adapter layer that points back to `.omv`
  * spec entrypoints such as OpenSpec governance/spec files or
    `.specify/memory/constitution.md` should contain a short adapter layer that
    points back to `.omv`
* This keeps one source of truth while acknowledging that most agents do not
  recursively auto-load arbitrary Markdown from custom directories

### Installation Model

**Chosen direction**

* first-wave adapters are `installable adapters`
* users explicitly run OMV commands to install them
* OMV should not silently take ownership of host/framework files in V1
* future evolution may move selected adapters toward direct management once the
  contract and write boundaries are proven stable
* adapter write strategy is `reference-first hybrid`
  * prefer thin references/imports back to `.omv/`
  * fall back to small managed blocks only when the host cannot consume a simple
    reference/import model
* spec adapter content strategy is `managed mirrored summaries`
* symlinks are a promising installation backend for Unix-like systems when they
  reduce drift without hurting host compatibility
* symlink backend default strategy is `auto`, not mandatory `link`

**Implications**

* OMV should own canonical content under `.omv/`
* adapter installation should be explicit, previewable, and repeatable
* adapter update behavior should be designed now so future upgrades do not
  require hand-merging full rule sets across multiple frameworks
* agent adapters are likely to be mostly pointers
* spec adapters may still need some host-native durable content because spec
  frameworks often reason over files inside their own directory structures
* installation backend and adapter content model are separate concerns:
  * content model: pointer vs managed summary
  * install backend: symlink vs copied/generated file
* OMV must detect host/platform capability before choosing `link`
* OMV should expose enough install metadata to explain whether an adapter is
  currently linked or materialized
* Unified commands need enough structure to avoid confusing agent adapters with
  spec adapters, hence explicit typed flags are preferred
* Adapter lifecycle operations should not rely only on filesystem guessing

### OpenSpec-Specific Inference From User Example

* OpenSpec has two relevant long-lived surfaces:
  * governance/config surfaces such as `openspec/config.yaml` and top-level
    project context files
  * durable domain specs such as `openspec/specs/versioning-source-unification/spec.md`
* This suggests OMV should not only tell agents "use `omv bump`", but should
  also offer a spec adapter that expresses rules like:
  * OMV is the authoritative version source
  * native manifests are synchronized outputs
  * changes affecting version flow must preserve `.omv` as truth
* Archived OpenSpec change folders should remain derived project history, not
  OMV-managed truth

### Agent Adapter Inference From User Example

* Codex-style skills under `.codex/skills/*` are a natural example of an agent
  adapter surface
* A good OMV agent adapter likely does two things:
  * tells the model that OMV exists and where its canonical contract lives
  * gives explicit tool-use rules:
    * read current version via `omv current`
    * mutate version via `omv bump`
    * do not hand-edit manifest versions

### Proposed Adapter Families

**Agent adapters**

* `claude-project-memory-adapter`
* `codex-agents-adapter`
* `codex-skill-adapter`
* future IDE/hook snippets

**Spec adapters**

* `openspec-governance-snippet`
* `openspec-versioning-spec`
* `trellis-guideline-snippet`
* future `speckit-constitution-snippet`

### First-Wave Adapter Matrix

**Claude Code**

* likely entrypoint: `CLAUDE.md`
* important capability: Claude Code auto-loads project memory files and supports
  `@path/to/import` references
* OMV implication:
  * OMV should be able to generate a thin Claude adapter that points at OMV's
    canonical instructions rather than duplicating them inline
  * installation should likely be explicit, for example by writing or updating a
    `CLAUDE.md` entrypoint with a minimal OMV-managed section
  * this is a strong fit for the reference-first strategy because Claude Code
    supports path-based imports/references in memory files
  * command shape example:
    * `omv adapter install --agent claude`

**Codex**

* likely entrypoints:
  * `AGENTS.md` for broad project guidance
  * optional Codex skills for richer workflow/task injection
* OMV implication:
  * a minimal Codex adapter can likely be an AGENTS snippet
  * a richer Codex adapter can additionally ship a dedicated OMV skill
  * installation should likely avoid overwriting existing project guidance
  * this is also a good fit for reference-first guidance plus optional richer
    skill files
  * command shape example:
    * `omv adapter install --agent codex`

**OpenSpec**

* relevant surfaces from user example:
  * `openspec/config.yaml`
  * durable governance/project files
  * long-lived domain specs such as
    `openspec/specs/versioning-source-unification/spec.md`
* OMV implication:
  * OpenSpec needs both governance injection and a durable versioning-domain
    spec adapter
  * installation should likely target stable governance/spec surfaces rather
    than archived change directories
  * this may require more than a pure pointer, because OpenSpec tools reason
    over host-native governance/spec files
  * a symlink-backed host-native spec file could be ideal on Unix-like systems
    if OpenSpec tooling treats symlinked files transparently
  * command shape example:
    * `omv adapter install --spec openspec`

**Trellis**

* relevant surfaces from current repo:
  * root `AGENTS.md`
  * `.trellis/spec/**/*`
  * optional host command/hook surfaces
* OMV implication:
  * treat Trellis as a host/spec framework adapter, not as OMV truth
  * inject OMV rules into Trellis specs/guides in a way that remains distinct
    from OMV's own canonical artifacts
  * installation should likely write targeted snippets into Trellis-owned
    guidance surfaces rather than trying to manage the whole framework
  * this may also benefit from host-native mirrored guidance rather than a pure
    pointer-only model
  * symlink-backed installs may work for some Trellis-owned files, but OMV still
    needs a fallback for platforms and repos where linked files are undesirable
  * command shape example:
    * `omv adapter install --spec trellis`

### Symlink Strategy Notes

* Symlinks are attractive because they nearly eliminate adapter drift and make
  OMV updates visible immediately in installed host-native files
* However, symlink support is not uniform:
  * Windows often needs Developer Mode or elevated privileges
  * some tools, editors, or repo policies may prefer plain files
  * Git/repo workflows may differ in how linked files are reviewed and managed
* Therefore OMV should likely support at least two install backends:
  * `link`: create symlinked host-native adapter files when supported
  * `materialize`: write/update managed host-native files with source metadata
* Default backend selection should be:
  * `auto`: try `link` where safe and supported
  * fall back to `materialize` when not supported or not desirable
* A materialized fallback should preserve:
  * source pointer back to `.omv`
  * managed-block or generated-file marker
  * enough metadata to support later `omv adapter refresh`

### Adapter State Model

**Chosen direction**

* use a hybrid state model:
  * dedicated OMV registry file for authoritative install metadata
  * host-file markers for visibility, inspection, and partial recovery

**Registry responsibilities**

* record installed adapters and categories (`agent` / `spec`)
* record targeted host/framework and install targets
* record install backend (`link` / `materialize`)
* record source canonical artifact path and/or contract version/hash
* support stable `status`, `refresh`, and future `uninstall`

**Host-file marker responsibilities**

* make OMV ownership visible inside touched files
* point back to canonical `.omv` source material
* allow limited recovery when the registry is stale or partially missing

**Likely file shape**

* a dedicated file such as `.omv/adapters.toml`
* host files contain concise managed markers or generated-file headers

### Drift / Failure Modes To Guard Against

* AI edits `Cargo.toml` / `CMakeLists.txt` directly instead of calling `omv bump`
* AI parses localized text output and breaks when locale changes
* host frameworks duplicate OMV rules and drift from the actual CLI contract
* runtime export paths are treated as truth rather than regenerated views
* future JSON changes break automation because no explicit contract version
* OMV writes excellent `.md` docs under `.omv/`, but no tool actually reads them
  because no entrypoint links or injects them
* spec frameworks receive only agent-level guidance, but their actual domain
  specs still allow direct-manifest editing patterns
* one adapter shape is assumed to fit all tools even though Claude Code, Codex,
  OpenSpec, and Trellis have different loading/update models
* output mode grows later, but the project painted itself into a corner because
  it only exposed a single hardcoded `--json` path with no broader mode concept
* JSON mode only works on success, forcing AI/tools to parse localized text on
  failure
* each command grows a different top-level JSON shape, making simple OMV
  automation harder than necessary
* adapter installation overwrites or tangles with existing host files because
  OMV does not have a stable merge/update strategy
* a pointer-only strategy is applied to spec frameworks that actually need
  host-native semantic files to be effective
* symlink-only installs are assumed to be portable, then fail or behave oddly on
  Windows or in constrained repo/tooling environments
* adapter CLI becomes ambiguous because agent/spec targets are expressed as loose
  positional names rather than typed flags
* adapter lifecycle state is inferred only from files, making refresh/uninstall
  fragile after manual edits or partial failures

### Decision Pressure

The biggest remaining architectural choice is whether the OMV automation
contract should be:

* **CLI-only**: the contract lives in documented command behavior and structured
  stdout/stderr
* **CLI + OMV-generated integration artifacts**: OMV also writes AI/spec-facing
  files under `.omv/` that point tools like OpenSpec / Spec Kit to the right
  commands and fields

## Decision (ADR-lite)

**Context**: The repo already has a solid internal version truth model, but the
human/AI workflow surfaces are still loosely coupled. The risk is not inside the
version engine; the risk is that code, specs, and AI instructions each evolve
their own habits.

**Current leaning after user clarification**: Still prefer Approach A at the
architecture level, but the most practical MVP shape now looks like Approach C
implemented on top of that architecture:

* keep `.omv/state.toml` as truth
* keep generated runtime exports as the code-facing read interface
* add one structured OMV CLI/API interface for AI/spec automation
* later project the same contract into OpenSpec / Spec Kit and optional
  Trellis-like host hooks

**Consequences**:

* We should design the contract before scattering more version rules into agent
  prompts
* OpenSpec / Spec Kit integration should probably reference OMV-owned rules
  rather than re-defining them independently
* We may want a distinction between:
  * "version truth" (`.omv/state.toml`)
  * "code read interface" (generated runtime exports)
  * "automation/spec read-write interface" (CLI/library/json)
  * "agent steering interface" (OpenSpec / Spec Kit / optional host hooks)
* The current implementation shape suggests that a stable JSON contract can be
  added without changing the core truth model
* The remaining question is whether OMV should also materialize AI/spec-facing
  guidance files as first-class artifacts under `.omv/`
* Current answer is likely "yes, but only together with thin per-tool adapter
  entrypoints"
* After the user's clarification, the adapter model should explicitly cover both:
  * agent skill/instruction injection
  * spec framework governance/spec injection
* The first delivery slice should focus on 4 adapters:
  * Claude Code
  * Codex
  * OpenSpec
  * Trellis
* The first delivery slice should use installable adapters invoked manually by
  the user
* Directly managed adapters are a possible later evolution after the write model
  proves safe
* The chosen write/update strategy is `reference-first hybrid`
* The chosen spec-adapter content strategy is `managed mirrored summaries`
* The install backend may be link-backed on Unix-like systems, with a safe
  non-link fallback for Windows and other environments
* The chosen backend default strategy is `auto`
* The chosen command shape is a unified `omv adapter` family with explicit
  typed flags such as `--agent` and `--spec`
* The chosen state model is `registry + marker` hybrid
* The chosen structured-output shape supports both `--json` and `--output json`
* The chosen JSON failure behavior is machine-readable JSON plus normal exit
  codes
* The chosen JSON contract shape is a shared envelope with `ok`,
  `contract_version`, `command`, `data`, and `error`

## Technical Notes

* Files inspected:
  * `README.md`
  * `src/core/versioning/engine.rs`
  * `src/storage/state.rs`
  * `src/core/schema.rs`
  * `src/app/mod.rs`
  * `src/sync/mod.rs`
  * `src/sync/rust.rs`
  * `src/sync/c_family.rs`
  * `src/sync/skills.rs`
  * `tests/integration/target_sync.rs`
  * `.trellis/spec/backend/database-guidelines.md`
  * `.trellis/spec/backend/quality-guidelines.md`
  * `.trellis/spec/guides/cross-layer-thinking-guide.md`
  * `.trellis/tasks/archive/2026-04/04-13-omv-cli-foundation/prd.md`
  * `.trellis/tasks/archive/2026-04/04-13-omv-target-sync-skills/prd.md`
  * `.opencode/plugin/session-start.js`
  * `.opencode/lib/trellis-context.js`
* External references used for research-first comparison:
  * OpenSpec docs / repo:
    * https://openspec.pro/
    * https://github.com/Fission-AI/OpenSpec
  * Spec Kit docs / repo:
    * https://github.github.com/spec-kit/index.html
    * https://github.github.com/spec-kit/quickstart.html
    * https://github.com/github/spec-kit
  * AGENTS.md convention:
    * https://agents.md/
    * https://github.com/openai/agents.md
* Key inferred risk:
  * if version intent lives only in prose prompts, tool upgrades or context
    shifts will reintroduce direct-manifest editing behavior
* Key user clarification:
  * Trellis-managed agent/hook files are not OMV's product artifact model
  * the current concrete OMV-facing footprint is `.omv` + manifest sync +
    runtime export files
  * the target long-term architecture should support different IDEs and spec
    frameworks through adapters
  * OpenSpec projects have durable `openspec/specs/*` domain specs and archived
    change folders that OMV should integrate with thoughtfully
* Tool-loading behavior from official docs:
  * AGENTS.md convention says agents read a predictable `AGENTS.md` entrypoint,
    and the nearest file takes precedence
  * Claude Code auto-loads `CLAUDE.md` project memory files and supports
    `@path` imports
  * OpenSpec initializes its own `AGENTS.md` and slash-command workflow
  * Spec Kit installs agent-specific command/template files and uses
    `.specify/memory/constitution.md` as project steering, but its upgrade docs
    warn that `constitution.md` can be overwritten
