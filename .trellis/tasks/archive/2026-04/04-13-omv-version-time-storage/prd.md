# Implement Version Engine, Time Validation, and Storage

## Goal

Implement the core versioning engine, time validation flow, and atomic `.omv`
persistence for `omv`.

## Requirements

- Support date-derived `x.y.z` output such as `2604.13.1`
- Support `daily-reset` and `continuous` build policies
- Validate time with NTP by default without changing system time
- Persist `.omv/config.toml`, `.omv/state.toml`, and `.omv/targets.toml`
  atomically

## Acceptance Criteria

- [ ] Version engine computes correct next versions
- [ ] Future stored date conflicts are blocked correctly
- [ ] `.omv` files round-trip through load/save
- [ ] Atomic-write behavior is tested
