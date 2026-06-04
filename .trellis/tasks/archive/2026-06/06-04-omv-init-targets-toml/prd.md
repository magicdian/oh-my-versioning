# omv init 重复执行不应覆盖已有 targets.toml

## Goal

用户在已配置 `.omv/targets.toml`（含手改路径、kind-based v2 targets）的项目里，为了新增 agent 集成而再次运行 `omv init` 时，不应破坏已有的 targets 配置。当前再次 init 会无条件用 draft + 默认路径重建并覆盖 `targets.toml`，导致用户手改丢失。

## What I already know

- `omv init`（非 TTY 走 from_discovery，TTY 走 init TUI）最终调用 `persist_init_state`（src/app/mod.rs:2759）。
- `persist_init_state` 中三种数据处理策略不一致：
  - config：load-then-merge（保留已有，只覆盖 locale/timezone/build_policy）
  - state：`ensure_state_exists`（仅缺失时创建，已有不动）
  - **targets：无条件 `build_targets_from_draft` + `save_targets(omv_root, &targets)` 覆盖** ← 问题根因
- `build_targets_from_draft`（src/app/mod.rs:2923）仅从 draft 重建 v1 targets，且字段用 `default_target_paths` 的默认值；并把 `v2_targets`、`unsupported_targets` 置空。
- 用户实测现象：手改的 `runtime_export_path = "sources/host/crates/xdb/src/generated/version.rs"` 被重置回默认 `src/generated/version.rs`。
- 已有 helper：`load_targets_if_exists`（src/app/mod.rs:2650）返回已有 targets 或 default。

## Decision (ADR-lite)

**Context**: 再次 init 需新增 agent 集成，但当前会无条件覆盖 targets.toml，丢失用户手改与 v2/unsupported 配置。
**Decision**: 采用「合并、已有优先」语义。在 `persist_init_state` 中先 `load_targets_if_exists`，按 target `id` 合并 draft 新发现的 v1 target：id 已存在则保留已有记录、忽略 draft 同 id 项；仅追加 id 不存在的新 target。`v2_targets` / `unsupported_targets` 原样保留。首次（文件不存在 → default 空）行为等价于现状。
**Consequences**: 用户手改字段与 kind-based 配置不再被重置；新增语言/agent 时新 target 仍可被追加。代价：合并逻辑与测试略复杂。

## Requirements

- 再次运行 `omv init` 不得覆盖或丢失用户已有的 `.omv/targets.toml` 内容（含手改字段、v2/kind-based、unsupported targets）。
- 合并以 target `id` 为键，已有记录优先；draft 中 id 不存在的 target 追加写入。
- `v2_targets` / `unsupported_targets` 在合并后原样保留。
- 首次运行（targets.toml 不存在）行为保持不变。

## Acceptance Criteria

- [ ] 已存在 targets.toml 且含手改 runtime_export_path 时，再次 init 后该字段保持不变。
- [ ] 已存在 v2_targets / unsupported_targets 时，再次 init 后不被清空。
- [ ] draft 含已有 targets.toml 中没有的新语言 target 时，再次 init 后该 target 被追加。
- [ ] 首次 init 仍正确生成 targets.toml。
- [ ] 新增/回归测试覆盖「重复 init 保留 + 追加 targets」。

## Out of Scope (explicit)

- 重新设计 init 的 target 发现/合并 UX（除非选定合并语义）。
- 修改 config/state 的现有持久化语义。

## Technical Notes

- 关键文件：src/app/mod.rs（persist_init_state 2759、build_targets_from_draft 2923、load_targets_if_exists 2650）。
- 现有测试：persist_init_state_writes_targets_initial_state_and_ai_artifacts（src/app/mod.rs:3061）。

## Open design question (for user)

再次 init 时 targets 的处理语义，三种候选见下方提问。
