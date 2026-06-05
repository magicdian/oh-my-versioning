# finalize-boundary 按 trellis 版本迁移，避免 finish-work 重复 bump

## Goal

修复同一工作单元被 OMV bump 两次的问题：Phase 3.4 commit 边界（`claude-commit`）已 bump 一次，`/trellis:finish-work` 边界（`trellis-finish-work`）又 bump 一次。按 backend spec 约定，Trellis v0.5+ 应迁移到 commit-time finalize-boundary，**不再** defer 到 finish-work。同时补充一个明确点：commit-time bump 之后必须触发项目编译（如 Rust `cargo build`）以更新并提交 lock 文件，否则会遗漏 `Cargo.lock`。

## What I already know

- 双 bump 根因：finalize-boundary 的 fingerprint 含 `provider`+`boundary`（identity_fields），commit 边界=`claude-commit`、finish-work 边界=`trellis-finish-work`，两者 fingerprint 不同 → 各 bump 一次，idempotency 不跨边界去重。实测 2606.4.2→4.3（commit）→4.4（finish-work）。
- **版本检测已实现但未使用**：`detect_trellis_version()` + `TrellisVersionInfo { version, is_v05_or_later }`（src/core/integration.rs:272-320）。`is_v05_or_later` 仅在 struct/detect/tests/spec doc 中出现，**runtime 行为从不分支**。v0.4 vs v0.5 的差异目前只体现在 codex 的 finish-work 文件路径解析（src/app/mod.rs:1569 `resolve_trellis_finish_work_path`），靠文件存在性而非 `.version`。
- finish-work 的 OMV-MANAGED 块文本硬编码在 `trellis_finish_work_finalize_block()`（src/adapter.rs:322-338），块标识 `spec-trellis-finalize-boundary-finish-work`（src/adapter.rs:16）。该块 v0.4/v0.5 **投影完全相同**，且其中明确要求「Run the project build command to update lock files（cargo build / npm install）」。
- 安装入口：`upsert_trellis_finish_work_finalize_block()`（src/app/mod.rs:1846）经 `IntegrationTargetBehavior::TrellisFinalizeBoundary` 写入 claude/opencode/codex 的 finish-work entrypoint。
- commit-time 指引：`.omv/ai/instructions.md:11` 与 `.omv/ai/adapters/project-instructions.md:10` 描述「commit confirmation 时调用 finalize-boundary」，但**均未提及 bump 后跑 build 更新 lock**。这是 Cargo.lock 漏提交的根因。
- backend spec（.trellis/spec/backend/index.md:96-102）已写明 v0.5+ 应 commit-time 触发、不 defer 到 finish-work；v0.4 `/finish-work` 可自动触发。

## Decision (ADR-lite)

**Context**: finalize-boundary 有 commit / finish-work 两个触发点，fingerprint 按 provider+boundary 区分 → 同一工作单元双 bump。版本检测 `is_v05_or_later` 已实现却从未驱动行为。
**Decision**:
1. `trellis_finish_work_finalize_block()` 接收 `is_v05_or_later`，按版本返回两种块文本：
   - **v0.5+ → 验证型块**：不再调用 `omv event finalize-boundary`；改为说明「version 已在 commit 边界 bump」，并保留 `omv sync --check --json` 验证 + 「若 lock 未更新则 build & commit」提醒。不产生 semantic bump。
   - **v0.4 → 现状 bump 型块**：保留 finalize-boundary 调用（向后兼容）。
2. 安装入口（src/app/mod.rs:1846 `upsert_trellis_finish_work_finalize_block`）经 `detect_trellis_version()` 取得 `is_v05_or_later` 并传入；无 `.trellis/.version` 时按现状默认（保守取 bump 型，保持兼容）。
3. commit-time 指引（`.omv/ai/instructions.md`、`.omv/ai/adapters/project-instructions.md`）补充：bump 后运行项目 build（cargo build / npm install）更新 lock，并连同 OMV 生成文件一起提交。
**Consequences**: v0.5+ 单 bump；v0.4 不变。需重新 apply/refresh 投影新块。fingerprint 机制不动（Out of Scope 保持）。

## Requirements (evolving)

- Trellis v0.5+：finish-work 边界不再产生重复的 semantic bump（同一工作单元只 bump 一次，发生在 commit 边界）。
- Trellis v0.4.x：保持现状（finish-work 触发 finalize-boundary），向后兼容。
- 行为分支必须真正消费 `detect_trellis_version()` / `is_v05_or_later`，而非仅靠文件路径推断。
- commit-time finalize-boundary 指引（instructions.md + project-instructions adapter，以及对应 host 投影）补充：bump 后运行项目 build 命令更新 lock，并连同 OMV 生成文件一起提交。
- 已安装项目可通过 `omv integrate apply`（或等价 refresh）重新投影获得新块文本。

## Acceptance Criteria (evolving)

- [ ] v0.5+ 项目走「commit bump → finish-work」后，版本只 bump 一次（finish-work 不再 semantic bump）。
- [ ] v0.4 项目 finish-work 仍触发 finalize-boundary（回归）。
- [ ] 行为由 `is_v05_or_later` 驱动，有单测覆盖两个版本分支。
- [ ] commit-time 指引（instructions.md/project-instructions）含「bump 后 build 更新 lock + 提交」步骤。
- [ ] 重新 apply/refresh 后，host finish-work 文件得到版本对应的新块。

## Out of Scope (explicit)

- 重新设计 fingerprint / finalization 去重机制（除非选定该方向）。
- 改动 commit 边界自身的 bump 时机。

## Technical Notes

- 关键文件：src/core/integration.rs:272（版本检测）、src/adapter.rs:16/322（块标识与文本）、src/app/mod.rs:1569/1846（路径解析与安装）、.omv/ai/instructions.md、.omv/ai/adapters/project-instructions.md、.omv/ai/adapters/trellis/guide.md。
- 现有 v0.4/v0.5 路径解析逻辑（resolve_trellis_finish_work_path）可作为版本感知的扩展点。
