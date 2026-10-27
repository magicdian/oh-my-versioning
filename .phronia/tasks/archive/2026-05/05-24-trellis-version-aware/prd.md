# Trellis v0.4 vs v0.5 版本感知处理

## Goal

OMV 识别 Trellis 版本，并在 AI 指令中指导 agent 在 **Phase 3.4 commit 确认时**（而非等到 /finish-work）调用 `omv event finalize-boundary`，让每次功能提交都触发版本更新，BuildNumber 精确反映开发迭代。

## Decision (ADR-lite)

**Context**: v0.4 的 `/finish-work` 自动触发 finalize-boundary；v0.5 架构变了，不再自动触发。
**Decision**: 将 `finalize-boundary` 前移到 Phase 3.4 commit 确认时触发。用户确认提交 → AI 调 `omv event finalize-boundary` → 再 commit 版本 bump 文件。每轮功能提交都有独立版本号。
**Consequences**: BuildNumber 累加反映同一 calendar day 内的迭代次数，版本号精确对应已完成的功能单元。

## What I already know

### Trellis v0.4 → v0.5 关键变化
- v0.4: `/finish-work` 是完整命令，调用链可自动触发 `finalize-boundary`
- v0.5: 5 个命令转为 auto-triggered skills（`before-dev`/`brainstorm`/`break-loop`/`check`/`update-spec`），`/start`/`/continue`/`/finish-work` 保留为用户 slash command
- v0.4.0: 2026-04-15 发布，是 v0.5 前最后一个版本，可用作前向兼容锚点
- v0.5.0: 2026-05-06 发布，breaking change（skill-first 架构）

### OMV 现有 Trellis 处理
- 已有两个 finish-work skill 路径：
  - `TRELLIS_FINISH_WORK_V05_PATH = ".agents/skills/trellis-finish-work/SKILL.md"`
  - `TRELLIS_FINISH_WORK_V04_PATH = ".agents/skills/finish-work/SKILL.md"`
- `resolve_trellis_finish_work_path()` 已做路径选择（优先 v05，回退 v04）
- `probe_trellis_finalize_boundary()` 检测 block 是否已安装
- 但**缺少 Trellis 实际版本号的检测**，无法区分 v0.5.7 / v0.5.19 / v0.4.0

### 当前项目 Trellis 版本
- 当前项目使用 v0.5.19
- `.trellis/.version` 文件存储了 Trellis 版本号

## Assumptions (temporary)

- Trellis 版本号可从 `.trellis/.version` 读取
- v0.4.0 是分界线：< 0.5.0 为旧版，>= 0.5.0 为新版

## Requirements

### A. 版本检测
- [ ] OMV 读取 `.trellis/.version` 获取 Trellis 版本号
- [ ] 分类：v0.4.x（旧版）vs v0.5.x+（新版）

### B. 更新 AI 指令中的 finalize-boundary 时机
- [ ] trellis/guide.md: "after /trellis:finish-work succeeds" → "during Phase 3.4 commit confirmation, before /trellis:finish-work"
- [ ] project-instructions.md: "At finalize boundaries" → 明确在 commit 确认时调用
- [ ] 更新 `canonical_sources()` 中这两个文件的内容

### C. 版本感知指引（trellis/guide.md）
- [ ] v0.5+：AI 在 Phase 3.4 用户确认提交时，调 `omv event finalize-boundary --change-type <type>`，再 commit bump 文件
- [ ] v0.4：保持向后兼容（finish-work hook 可能自动触发）

## Acceptance Criteria

- [ ] OMV 能读取 `.trellis/.version` 并正确解析版本号
- [ ] v0.5+ 项目的 AI agent 在 Phase 3.4 commit 确认后调 `omv event finalize-boundary`
- [ ] lint / type-check / test 通过

## Definition of Done

- Tests 覆盖版本检测和分类
- Lint / typecheck 通过

## Out of Scope

- 不修改 OMV 的 finish-work hook 逻辑
- 不修改 `finalize-boundary` 本身的行为

## Technical Notes

- Trellis 版本文件：`.trellis/.version`
- 当前路径选择：`src/app/mod.rs:1438-1455`
- Adapter 源文件：
  - `.omv/ai/adapters/trellis/guide.md`（Trellis spec guide）
  - `.omv/ai/adapters/project-instructions.md`（agent 通用指令）
  - `.omv/ai/instructions.md`（OMV 核心指令）
