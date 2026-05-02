# brainstorm: omv end-user distribution

## Goal

Define how OMV should be packaged and distributed to end users so installation
is simple across Windows, macOS, and Linux while fitting OMV's Rust CLI
implementation and future release process.

## What I already know

* The user wants installation to feel as simple as existing AI/dev tools such
  as `npm install -g @openai/codex`, `npm install -g @mindfoldhq/trellis@latest`,
  and `npm install -g uipro-cli`.
* The user prefers one cross-platform install command if practical.
* If one command is not practical, platform-native package managers are
  acceptable, especially Homebrew on macOS.
* The user does not want to assume end users have Rust installed, so
  `cargo install` should not be treated as a primary end-user installation
  route.
* The user is considering an MVP release flow where GitHub Releases build
  macOS, Linux, and Windows binaries first, and npm provides a simple global
  installer that resolves to those release binaries.
* The user chose GitHub Releases plus npm as the desired MVP distribution
  route.
* The formal public GitHub repository is
  `https://github.com/magicdian/oh-my-versioning`, and the release branch is
  `main`.
* The npm package `@magicdian/omv` does not exist yet and must be bootstrapped
  before Trusted Publishing can be configured from package settings.
* The one-time local npm bootstrap package must be created outside the repo,
  under `/tmp`, so it does not pollute the current project source tree.
* OMV is currently a Rust CLI crate named `omv` in `Cargo.toml`.
* The repository has no `package.json`, `dist.toml`, release workflow, Homebrew
  formula, Scoop manifest, or winget manifest yet.
* `README.md` documents OMV commands but does not yet document installation.

## Assumptions (temporary)

* OMV should ship prebuilt binaries for common platforms rather than expecting
  end users to compile from source.
* npm can be used as a distribution channel for a Rust binary wrapper, but it
  should not be the only long-term distribution channel.
* GitHub Releases can be the first artifact host unless the project later needs
  a dedicated CDN or static mirror.

## Open Questions

* None.

## Requirements (evolving)

* Provide a simple, documented install path for Windows, macOS, and Linux.
* Preserve the `omv` command name after installation.
* Prefer prebuilt binaries for fast install and no Rust toolchain requirement.
* Do not require a Rust toolchain for the primary end-user install path.
* Keep source installation via Cargo only as a developer/fallback path, not as
  a recommended user path.
* Publish npm versions in lockstep with OMV release versions; do not make a
  fixed npm package dynamically install whatever GitHub currently marks as
  latest.
* On release/tag publication, build and attach prebuilt OMV binaries for macOS,
  Linux, and Windows.
* Include architecture-aware release artifacts so the npm installer can select
  the correct binary for the user's platform.
* First release target matrix must include both `x86_64` and `aarch64` for
  macOS, Linux, and Windows.
* Automatically publish a new npm package version only after required release
  binaries are built and available.
* Keep npm package version, OMV crate version, and GitHub release tag aligned.
* Publish the npm package as `@magicdian/omv`; `magicdian` is the project brand
  and npm scope.
* The documented primary install command is `npm install -g @magicdian/omv`.
* Use `cargo-dist`/`dist` for release automation instead of hand-maintaining a
  custom matrix build and npm wrapper workflow.
* Use only npm Trusted Publishing/OIDC for CI publishing so the public GitHub
  repository does not store a long-lived npm token.
* Do not implement or document an `NPM_TOKEN` fallback path for publishing.
* Bootstrap package creation may use a one-time local interactive
  `npm publish --access public` with the user's npm account and 2FA, but must
  not use a long-lived token or GitHub secret.
* Document the bootstrap process as a `/tmp`-based local operation. No bootstrap
  placeholder package files should be committed to this repository.

## Acceptance Criteria (evolving)

* [ ] A recommended distribution strategy is selected.
* [ ] MVP install commands are defined for Windows, macOS, and Linux.
* [ ] Required release artifacts and package manager channels are identified.
* [ ] Out-of-scope package channels are explicitly documented.
* [ ] GitHub Release contains platform/architecture-specific archives for the
  supported target matrix.
* [ ] npm publish runs only after required release artifacts are available.
* [ ] The npm package installs or exposes the binary matching the user's OS and
  CPU architecture.
* [ ] A failed required build or missing required artifact prevents npm
  publication.
* [ ] `npm install -g @magicdian/omv` provides the `omv` command on supported
  platforms.
* [ ] Bootstrap instructions create temporary npm package files only under
  `/tmp`, not inside the repository.

## Definition of Done (team quality bar)

* Tests added/updated if release automation code is added.
* Lint / typecheck / CI green if implementation follows this brainstorm.
* Docs/notes updated if install behavior changes.
* Rollout/rollback considered for package publishing mistakes.

## Research Notes

### What similar tools and current packaging docs indicate

* npm supports executable packages through `package.json` `bin`; global
  installs link commands onto `PATH`, including Windows shims.
* Cargo supports `cargo install <crate>` for executable Rust crates, but this
  builds from source and requires a Rust toolchain, so it is not a good primary
  route for OMV end users.
* `cargo-dist` / `dist` is purpose-built for shipping prebuilt CLI binaries and
  can generate shell installers, PowerShell installers, npm installers,
  Homebrew formulae, and MSI installers.
* Homebrew distribution normally uses a tap repository for third-party formulae;
  `cargo-dist` can publish formulae to a custom tap backed by GitHub releases.
* Windows-native package distribution can be layered later through winget
  manifests and/or Scoop buckets.
* Precedents for non-Node/native tools distributed through npm include Biome
  (`@biomejs/biome`, Rust-based toolchain with npm install plus standalone
  executable), Oxc/oxlint (`oxlint`, Rust-based JS/TS linter installable with
  npm/npx), SWC (`@swc/core`, Rust compiler distributed through npm for JS
  users), and esbuild (Go native executable distributed through npm with
  platform-specific binaries).

### Constraints from this repo/project

* Current `Cargo.toml` has only package name/version/dependencies; it lacks
  release metadata such as description, homepage, repository, license, and
  dist configuration.
* OMV is a single binary CLI, which is a good fit for prebuilt archive
  distribution.
* The version format is currently date-based (`2604.18.1`) and must remain valid
  SemVer-compatible for Cargo/npm/dist package publication.

### Feasible approaches here

**Approach A: Prebuilt binary release pipeline with npm as the universal
developer-facing command** (Recommended)

* How it works: use `dist`/`cargo-dist` to build GitHub Release binaries and
  publish an npm package that installs/runs the correct platform binary; also
  publish shell and PowerShell installers.
* Primary command: `npm install -g <scope>/omv` or `npm install -g omv` if the
  package name is available.
* Pros: matches the user's desired cross-platform command; no Rust toolchain
  required; fits developers who already use Node/npm for Codex/Trellis-style
  tools.
* Cons: requires Node/npm even though OMV is not a Node tool; npm package name
  availability and ownership must be checked; release automation is more
  involved than Cargo-only.

**Approach B: Native package managers first, npm optional**

* How it works: publish GitHub Release binaries, Homebrew tap for macOS/Linux,
  winget/Scoop for Windows, and shell/PowerShell installers.
* Pros: feels native on each platform; package manager upgrades are natural;
  avoids making Node a soft dependency for a Rust CLI.
* Cons: no single command across all platforms; winget/Scoop publication adds
  review and maintenance overhead.

**Approach C: Cargo-first MVP**

* How it works: publish to crates.io and document `cargo install omv --locked`,
  with GitHub Release binaries as manual downloads.
* Pros: simplest to implement for a Rust crate; good for early developer users.
* Cons: requires Rust toolchain; slower installs; poor fit for non-Rust users
  and the user's "simple install" goal.

## Decision (ADR-lite)

**Context**: OMV is a Rust-authored CLI, but end users should not need a Rust
toolchain. The desired installation experience is close to modern developer
tools distributed via npm, while preserving native prebuilt binaries as the
actual release artifact.

**Decision**: Use GitHub Releases plus npm for the MVP distribution flow.
GitHub Releases are the authoritative binary artifact host. npm publishes a new
package version for each OMV release and installs or exposes the matching
prebuilt binary. The npm package must not float to GitHub's mutable `latest`
release independently of npm package versions. The npm package name is
`@magicdian/omv`. Use `cargo-dist`/`dist` to generate and maintain the release
workflow and npm installer plumbing.

**Consequences**: Release automation must publish artifacts in dependency
order: build binaries first, create/complete the GitHub Release, then publish
npm. npm versioning, GitHub tags, and OMV crate versions must remain aligned.
Native package managers such as Homebrew, winget, and Scoop can be added later
without changing the release artifact model. Generated release workflow files
should be treated as derived from dist configuration, so future changes should
be made in dist config and regenerated rather than hand-patching the workflow
where possible.

## Expansion Sweep

### Future evolution

* OMV may later need signed binaries, checksums, provenance/attestations, and
  mirror fallback if it becomes a widely used automation tool.
* Release automation should leave room for package manager publishing without
  changing the user-facing command name.

### Related scenarios

* Installation docs should stay consistent with future `omv self update` or
  generated updater support if adopted.
* Uninstall/upgrade commands should be documented per channel.

### Failure and edge cases

* Publishing the same version to multiple registries can partially fail and
  leave channels inconsistent.
* npm, Homebrew, winget, and Scoop each have different package name ownership,
  review, update, and rollback behavior.
* Corporate networks may block npm or install scripts; native package managers
  and direct binary archives are useful fallback paths.
* npm update behavior is registry-version based. `npm update -g <pkg>` updates
  when npm sees a newer package version or dist-tag target; it does not monitor
  GitHub Releases directly.
* A package that always downloads GitHub `latest` during install is less
  reproducible and does not give users normal npm update semantics unless the
  npm package version is also republished.
* Initial release target candidates:
  * `x86_64-apple-darwin`
  * `aarch64-apple-darwin`
  * `x86_64-unknown-linux-gnu`
  * `aarch64-unknown-linux-gnu`
  * `x86_64-pc-windows-msvc`
  * `aarch64-pc-windows-msvc`
* The npm publication job should depend on all required binary jobs. Optional
  targets should not silently block core release publication unless marked as
  required in the release matrix.
* User selected the full target matrix for the first release:
  macOS/Linux/Windows x `x86_64`/`aarch64`. All six platform artifacts are
  required before npm publication.
* User selected npm package scope/name `@magicdian/omv` because `magicdian` is
  the current project brand.
* User selected `cargo-dist`/`dist` as the release implementation path.
* Expected implementation shape:
  * add required release metadata to `Cargo.toml`
  * add dist configuration with GitHub CI, npm installer, npm publish job, and
    the six required targets
  * generate or add the GitHub Actions release workflow
  * document npm Trusted Publishing setup only; no explicit npm token fallback
  * update install docs with `npm install -g @magicdian/omv`
* Security notes:
  * repository secrets are encrypted and not readable from source code
  * secrets must not be printed, echoed, committed, or embedded in generated
    package files
  * workflows should avoid exposing publish credentials on pull request events
    from forks
  * public repository publishing must use trusted publishing because it uses
    short-lived OIDC credentials instead of a reusable token
  * no `NPM_TOKEN` or equivalent long-lived npm publish credential should be
    configured, referenced, or required by the release workflow
  * because Trusted Publishing is configured from npm package settings, the
    package must first exist; create it with a one-time manual publish, then
    configure Trusted Publishing and token-disallow publishing access
  * local package bootstrap should happen in `/tmp/npm-bootstrap-omv` or another
    temporary directory outside this repository
* User selected npm Trusted Publishing/OIDC only and rejected an explicit token
  fallback path.

## Out of Scope (explicit)

* Implementing release automation before the distribution strategy is approved.
* Building a custom installer service or CDN in the MVP.
* Supporting every Linux distro package manager in the first release.

## Technical Notes

* Inspected `Cargo.toml`: Rust crate name is `omv`, version `2604.18.1`,
  edition `2024`.
* Inspected `README.md`: command documentation exists, installation docs do not.
* Inspected `docs/OMV_CONTRACT_ARCHITECTURE.md`: future release integrations are
  explicitly not part of earlier architecture stages, so packaging should stay
  separate from OMV project contract behavior.
* Research references:
  * npm package executables: https://docs.npmjs.com/cli/v7/configuring-npm/package-json/
  * npm install behavior and global bin links: https://docs.npmjs.com/cli/v11/commands/npm-install/
  * Cargo install: https://doc.rust-lang.org/stable/cargo/commands/cargo-install.html
  * dist/cargo-dist overview: https://axodotdev.github.io/cargo-dist/
  * dist installer configuration: https://axodotdev.github.io/cargo-dist/book/reference/config.html
  * dist Homebrew installer: https://axodotdev.github.io/cargo-dist/book/installers/homebrew.html
  * Homebrew taps: https://docs.brew.sh/Taps
  * Windows Package Manager manifests: https://learn.microsoft.com/en-us/windows/package-manager/package/manifest
  * Scoop manifests: https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests
