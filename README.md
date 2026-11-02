# oh-my-versioning

Date-based version management with one local source of truth.

## OMV CLI

`omv` is a local-first Rust CLI that treats `.omv/` as the canonical version
authority and synchronizes language-native manifests plus runtime-export files
from that state.

## Installation

Install the latest prebuilt OMV binary with npm:

```bash
npm install -g @magicdian/omv
```

The npm package installs the platform-specific `omv` binary from the matching
GitHub Release. End users do not need a Rust toolchain.

## Core Model

- `.omv/state.toml` is the only mutable version truth.
- Native manifests such as `Cargo.toml`, `CMakeLists.txt`, `pyproject.toml`,
  and `go.mod` are synchronized outputs.
- Runtime exports such as `src/generated/version.rs` and
  `include/omv_version.h` are generated read-only views for application code.
- Automation and AI tools should read version state with `omv current`. In
  `local` mode they advance it with `omv bump`; in `pipeline` mode only the
  release job writes it.

## Release Modes

OMV supports two project-level release policies in `.omv/config.toml`:

- `release_mode = "local"` (default): the current local workflow. `omv bump`
  and `omv sync` write for everyone.
- `release_mode = "pipeline"`: the release pipeline is the only writer of
  version truth. Contributors use `omv current`, `omv plan`, and
  `omv sync --check`; `omv bump` / `omv sync` require `--as-release` and are
  intended for the release job. Finalize events return a read-only `pending`
  result.

See [docs/RELEASE_PIPELINE.md](docs/RELEASE_PIPELINE.md) for the architecture,
CI job examples, permissions, idempotency, and migration guidance.

## Commands

- `omv init`: interactive menuconfig-style initialization
- `omv current`: print the current managed version and `.omv` state summary
- `omv bump`: compute the next version and sync enabled targets
  (`--expect <version>`, `--now <YYYY-MM-DD>`, `--dry-run`, `--as-release`)
- `omv sync`: re-sync manifests/runtime exports from current `.omv/state.toml`
  (`--check`, `--as-release`)
- `omv adapter install`: install agent/spec adapters into host frameworks
- `omv adapter refresh`: re-render previously installed adapters from registry
- `omv adapter list`: list available adapters
- `omv adapter status`: show installed adapters recorded by OMV
- `omv help`: print help
- `omv version` / `omv -V`: print CLI version

## Global Options

- `--locale <en-US|zh-CN>`: override locale for current run
- `--no-ntp`: skip NTP validation for current run (`bump`)
- `--json`: shortcut for structured JSON output
- `--output json`: extensible structured output form

## Structured Output

`omv current`, `omv bump`, `omv sync`, and `omv adapter ...` support
machine-readable output through `--json` or `--output json`.

Success envelope:

```json
{
  "ok": true,
  "contract_version": "1",
  "command": "current",
  "data": {
    "version": "2604.13.3"
  },
  "error": null
}
```

Failure envelope:

```json
{
  "ok": false,
  "contract_version": "1",
  "command": "runtime",
  "data": null,
  "error": {
    "code": "missing_state",
    "message": "missing state file: ...",
    "details": {
      "path": ".omv/state.toml"
    }
  }
}
```

Exit codes remain meaningful:

- CLI/parse failures exit non-zero with a structured `cli` error envelope
- runtime failures exit non-zero with a structured `runtime` error envelope

## Files Under `.omv/`

Canonical persisted state:

- `.omv/config.toml`
- `.omv/state.toml`
- `.omv/targets.toml`
- `.omv/adapters.toml`

Generated AI/spec contract surface:

- `.omv/ai/contract.json`
- `.omv/ai/instructions.md`
- `.omv/ai/adapters/...`

Generated helper guidance:

- `.omv/skills/README.md`
- `.omv/skills/bump-guidance.md`

`adapters.toml` records which host adapters OMV has installed so
`omv adapter refresh` can safely re-project them after OMV contract changes.

## Adapter Architecture

OMV uses installable adapters so host-framework integration stays explicit and
replaceable.

Adapter categories:

- agent adapters: inject OMV version rules into agent/IDE entrypoints
- spec adapters: inject OMV version rules into spec/governance frameworks

First-wave supported adapters:

- agent: `codex`, `opencode`, `claude`
- spec: `openspec`, `trellis`, `phronia`

Example:

```bash
omv adapter install --agent codex --spec openspec
```

Install behavior uses an `auto` backend:

- on Unix-like systems, OMV prefers symlinks when the target file shape allows it
- otherwise OMV materializes managed files or managed blocks
- OMV records the result in `.omv/adapters.toml`

## AI and Spec Flow

The reference flow is:

1. Read current truth via `omv current --json`
2. Plan/spec/work on code without hand-editing manifest versions
3. Advance version truth via `omv bump --json`
4. Let OMV sync manifests and runtime exports

Host files such as `AGENTS.md`, `CLAUDE.md`, `openspec/project.md`, or
Trellis/Phronia guide indexes are projections of `.omv/ai/*`, not independent
truth sources.

## Version Model

- version format: `YYMM.dd.BuildNumber` (numeric `x.y.z` compatible)
- `daily-reset`: reset `BuildNumber` to `1` when date changes
- `continuous`: keep incrementing `BuildNumber` across date changes

## Quick Examples

```bash
omv init
omv current
omv current --json
omv bump --json
omv adapter list
omv adapter install --agent codex --spec trellis
omv adapter refresh
```
