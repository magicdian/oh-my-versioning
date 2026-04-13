# oh-my-versioning
Date-based version control system

## OMV CLI (MVP)

`omv` is a local-first Rust CLI that treats `.omv/` as the single source of truth
for version state and syncs language-native manifests/runtime exports from that
state.

### Commands

- `omv init`: interactive menuconfig-style initialization
- `omv bump`: compute next version and sync enabled targets
- `omv sync`: re-sync manifests/runtime exports from current `.omv/state.toml`
- `omv help`: print help
- `omv version` / `omv -V`: print CLI version

### Global Options

- `--locale <en-US|zh-CN>`: override locale for current run
- `--no-ntp`: skip NTP validation for current run (`bump`)

### Generated OMV Files

`omv init` creates:

- `.omv/config.toml`
- `.omv/state.toml`
- `.omv/targets.toml`

`omv sync` / `omv bump` also generates:

- `.omv/skills/README.md`
- `.omv/skills/bump-guidance.md`

### Version Model

- version format: `YYMM.dd.BuildNumber` (numeric `x.y.z` compatible)
- `daily-reset`: reset `BuildNumber` to `1` when date changes
- `continuous`: keep incrementing `BuildNumber` across date changes
