# OMV CLI Foundation

## Goal

Design and implement the MVP of `omv`, a Rust-based CLI that becomes the single source of truth for project versions using the date-based format `YYMM.dd.BuildNumber`, while keeping room for future language integration, AI workflow integration, and tool distribution.

## What I already know

* Project name: `oh-my-versioning`
* CLI name: `omv`
* Primary language: Rust
* Current project itself should also use the `YYMM.dd.BuildNumber` versioning scheme
* For the Rust CLI project itself, this fits Cargo's `x.y.z` numeric version shape directly, for example `2604.13.1`
* Version format example:
  * April 13, 2026 first build → `2604.13.1`
  * April 9, 2026 first build → `2604.9.1`
  * Day component is not zero-padded
* Version bump rule:
  * If stored version date matches "today" in configured timezone, increment `BuildNumber`
  * If stored version date differs from "today", replace date with today and reset `BuildNumber` to `1`
* `omv init` should:
  * ask user to select timezone, defaulting to `UTC+0`
  * suggest `UTC+0` for large OSS projects, and local timezone for personal projects
  * store config under `.omv/`
  * place `.omv/` in current directory if not in Git, otherwise in Git repo root
  * allow selecting a development language
  * support cases where the project has not been created yet
  * prefer automatic language/project discovery first
  * pre-enable automatically discovered language support entries
  * allow users to press `Space` to disable auto-discovered entries
  * allow users to press `Space` to manually enable additional language support entries
  * use a `ratatui` menuconfig-style interface
* `omv` should be the only source of truth for version data
* Language selection during init may require generating language-specific libraries/helpers so the main app can import version information from `omv`
* Exported language libraries are mainly for letting the application read version information at runtime
* Time correctness matters:
  * `omv` may need built-in NTP querying
  * if stored version date is greater than current date, the CLI should stop and ask the user to manually confirm the correct date
  * if the user-confirmed date matches NTP date, previous version data is likely abnormal and should be corrected
* `.omv/skills` should contain AI/model guidance so tools can update version numbers through the `omv` toolchain
* CLI help, command output, and init TUI all need i18n support from V1
* User-facing text must not be hardcoded in code; locale catalogs should be used
* Supported locales for V1 are:
  * English (`en-US`)
  * Chinese (`zh-CN`)
* User locale preference should be stored in config and reused by later commands
* Future integration targets may include frameworks/tooling such as OpenSpec and SpecKit
* Future distribution may include installing via `npm`, even if the core tool is implemented in Rust
* Internal source of truth is fundamentally the date/time basis rather than only a stored version string
* The first version should support `x.y.z` output capability
* The date-derived version `YYMM.DD.BuildNumber` is also a valid `x.y.z` numeric triplet for Cargo usage
* `BuildNumber` is a secondary core dimension relative to date, and should support multiple counting policies:
  * reset daily
  * continue accumulating
* `omv` itself should use `x.y.z`-compatible output, with the daily-reset build policy
* `omv init` should behave like a menuconfig flow rather than a plain prompt wizard
* Reference UX contract comes from `/Users/magicdian/Documents/personal_project/oh-my-versioning/docs/matrix/MENUCONFIG_STYLE_MATRIX.md`
* i18n implementation can follow the broad pattern used in bridgingio:
  * embedded locale catalogs
  * locale normalization
  * English fallback
  * catalog key parity validation
* Inference from that matrix:
  * main flow should stay single-column
  * semantic row templates matter
  * `multi-select-row` with `[ ]` / `[*]` is the right interaction for language support toggles
  * `field-entry-row` is a good fit for timezone / policy selections
  * `Enter` should follow `--->` detail flows, while `Space` should control toggle semantics
  * `Esc` should close popup, back out one level, then exit from root
* When no project manifests exist yet and the user selects language support, `omv init` should present a popup so the user can choose one of three strategies:
  * record support intent only
  * initialize runtime export templates without touching native manifests
  * create minimal language project scaffolding

## Assumptions (temporary)

* The first milestone is a local CLI workflow, not a hosted service
* NPM distribution is important but can be deferred until after the core product shape is stable
* NTP validation can be designed in a way that degrades gracefully when offline

## Open Questions

* None at the current product-definition stage

## Requirements (evolving)

* Provide a Rust CLI named `omv`
* Use a versioning core centered on date/time as the canonical truth
* Derive project-facing version values from the date core plus `BuildNumber`
* Keep the architecture extensible so additional version schemes/formats can be supported later
* Implement version bump logic based on configured timezone
* Initialize `.omv/` in Git root when inside a Git repo, otherwise in current directory
* Capture timezone configuration during `omv init`
* Capture target development language during `omv init`
* Implement `omv init` as a `ratatui` menuconfig-style TUI in V1
* `omv init` should auto-discover likely project languages/targets when possible
* Auto-discovered language support entries should start enabled by default
* Users should be able to toggle discovered and non-discovered language support entries with `Space`
* When no project manifests exist yet, `omv init` should show an explicit popup so the user can choose the pre-project handling strategy for selected languages
* Support i18n for both CLI and init TUI in V1
* Persist the user's locale preference in `.omv/config.toml`
* Avoid hardcoded user-facing copy in source code
* Support `en-US` and `zh-CN` catalogs in V1
* Treat `.omv` metadata as the single source of truth for project version
* Use a split `.omv` layout in MVP, with separate files for configuration, mutable state, and registered targets
* Represent registered targets in `.omv/targets.toml` as a flat target list in V1
* Support validation against trusted time data to reduce incorrect version bumps caused by bad system clocks
* Use NTP checking by default, while allowing users to skip it via CLI options/configuration
* Never modify the system clock; NTP time is only used internally by `omv` for validation and version decisions
* Support `x.y.z` output in the MVP
* Support date-derived triplet output such as `2604.13.1`, which is valid for Cargo and semver-like consumers expecting numeric `x.y.z`
* Support configurable `BuildNumber` policies, at minimum:
  * reset daily
  * continuous increment
* Support language integrations in the MVP for:
  * C/C++
  * Java
  * Rust
  * Python
  * Go
* Support exporting runtime-readable version artifacts/libraries for supported languages
* Support syncing project/package metadata files from `.omv` so language-native tooling sees the correct version
* Support complex repositories, including the possibility of multiple managed project targets under one `.omv` root
* Provide AI-facing instructions/assets under `.omv/skills`
* Provide a meaningful first version of AI integration rather than leaving it as a placeholder only
* Guide AI frameworks and skills to update versions through `omv bump`
* Keep architecture open to future packaging and update channels, including deferred NPM distribution

## Acceptance Criteria (evolving)

* [ ] A user can initialize a project with `omv init`
* [ ] The CLI stores configuration under the correct `.omv/` location
* [ ] `omv init` supports pre-project initialization, not only existing repositories with manifests
* [ ] `omv init` uses a menuconfig-style TUI with discover-and-toggle behavior for language support
* [ ] When no project manifests exist, `omv init` presents a popup that lets the user choose among the three pre-project strategies
* [ ] CLI output and init TUI support `en-US` and `zh-CN`
* [ ] Locale preference is persisted and reused across later commands
* [ ] User-facing copy is catalog-driven rather than hardcoded
* [ ] The CLI can calculate the next version according to the date-based bump rule
* [ ] The project's own version can be managed by `omv`
* [ ] The MVP supports `x.y.z`-style version output derived from the date/time core
* [ ] `omv` itself uses an `x.y.z`-compatible version representation suitable for Cargo
* [ ] Users can choose the `BuildNumber` counting policy where supported by the design
* [ ] NTP is used by default for validation but can be skipped explicitly
* [ ] The system clock is never changed by `omv`
* [ ] The MVP supports the first batch of language integrations: C/C++, Java, Rust, Python, and Go
* [ ] The design defines how `.omv` syncs version values into native project files for supported ecosystems
* [ ] The design defines how applications read version values at runtime from generated/exported artifacts
* [ ] The design includes extension points for future version formats without rewriting the core
* [ ] The design clearly defines the canonical `.omv` file layout for config, state, and targets
* [ ] The design defines a flat V1 target-registration model that can be evolved later for monorepo needs
* [ ] The MVP clearly defines how AI tooling under `.omv/skills` invokes `omv` for version updates
* [ ] `omv bump` updates `.omv` truth and synchronizes registered targets by default

## Definition of Done

* Tests added or updated for core version calculation logic
* CLI behavior documented in project docs
* Lint / format / test commands pass
* The MVP scope and explicit non-goals are documented before coding begins

## Out of Scope (explicit)

* Broad language ecosystem support beyond C/C++, Java, Rust, Python, and Go
* Production-grade auto-update system across all package managers
* Immediate NPM distribution implementation
* Deep integrations with every AI/spec framework in the first iteration

## Decision (ADR-lite)

**Context**: The product needs to be useful immediately, but the user also wants it to grow into a broadly usable version-management foundation across multiple languages and workflows.

**Decision**:
* MVP scope follows the "full first version" direction, except NPM distribution is deferred
* The architecture should preserve room for future version-format expansion
* The versioning model is centered on date/time as the deepest source of truth
* Project-facing versions are derived from date/time plus `BuildNumber`
* NTP validation is enabled by default, but users can explicitly skip it
* NTP data is advisory for `omv` only and must never modify system time
* First-release language support targets C/C++, Java, Rust, Python, and Go
* MVP supports `x.y.z` output, and the date-derived version form `2604.13.1` is treated as a first-class triplet representation
* `BuildNumber` policy should be configurable, including daily reset and continuous increment
* `omv` itself follows the daily-reset policy for its own version progression
* `.omv` is the only source of truth; native project files and generated runtime libraries are derived artifacts
* Generated/exported libraries are primarily for runtime version access inside applications
* Version synchronization should happen automatically as part of normal `omv` commands
* Hook-based integrations are a future extension point, but not a V1 requirement
* AI framework integration in V1 should explicitly instruct models/skills to use `omv bump` rather than editing version files directly
* V1 should use a split storage model inside `.omv/`, separating configuration, mutable version state, and target registration
* Monorepo-specific directory sharding can be deferred until there is a real need
* `omv init` should be designed as a menuconfig-like TUI flow built with `ratatui`
* The menu flow should prefer automatic discovery, but keep operator control through explicit toggles
* The init UX should follow the referenced menuconfig interaction contract closely enough that row semantics and key behavior feel predictable
* `.omv/targets.toml` should use a flat list of targets in V1 rather than language-grouped or monorepo-first nesting
* i18n is a first-class V1 capability for both CLI and TUI
* The project should use catalog-driven localization modeled after the referenced bridgingio implementation pattern

**Consequences**:
* Native project files such as `Cargo.toml`, `go.mod`, `pyproject.toml`, and similar files should be treated as synchronized outputs rather than canonical storage
* Some ecosystems may allow indirect references through build tooling, but portable cross-language support likely still requires `omv`-managed sync/materialization
* The MVP should define target registration and sync flows clearly for single-project and multi-project repositories
* `omv bump` should likely perform both state update and sync, while `omv sync` remains available for regeneration/repair workflows
* Hook interfaces should be designed so they can be added later without changing the core storage contract
* A likely V1 shape is:
  * `.omv/config.toml`
  * `.omv/state.toml`
  * `.omv/targets.toml`
  * `.omv/skills/`
* A flat target list keeps the initial implementation simpler and is easier to migrate later than a prematurely nested monorepo structure

## Technical Notes

* Current repo is effectively empty from an application-code perspective:
  * `README.md` currently contains only the project name and a one-line description
  * No Rust source tree or Cargo manifest is present yet
* Existing Trellis task `00-bootstrap-guidelines` is still open, but this PRD is for a new product-definition task focused on `omv`
* Inference from ecosystem norms: native package manifests usually cannot all share a single universal "reference external version file" mechanism across C/C++, Java, Rust, Python, and Go
* Design implication: the most portable approach is likely `.omv` as truth plus per-language sync/export adapters
* UI/UX reference reviewed and localized for this project: `/Users/magicdian/Documents/personal_project/oh-my-versioning/docs/matrix/MENUCONFIG_STYLE_MATRIX.md`
