# 添加 opencode agent 平台支持（含统一 ProjectInstructions 架构）

## Goal

1. 新增 opencode 作为 agent host 平台
2. 将 `ProjectInstructions` capability 改为跨 agent host 的统一 managed block，避免多平台时 AGENTS.md 内容重复浪费 token
3. 提供旧格式（`integration-codex-project-instructions`）→ 新格式（`integration-project-instructions`）的自动迁移

## What I already know

- 当前 agent host 只有 **Codex**（`IntegrationProvider::Codex`），Trellis 是 spec workflow host
- Codex 有 2 个 capability：`ProjectInstructions`（→ `AGENTS.md`）和 `HostSkill`（→ `.codex/skills/omv-versioning/SKILL.md`）
- Block ID 在 `src/app/mod.rs:1618` 生成：`format!("integration-{}-{}", plan.provider, plan.capability)`
- `ProjectInstructions` 行为是 `FullFileOrManagedBlock`，即文件不存在时全量写入，存在时插入 managed block
- `HostSkill` 行为是 `DedicatedFile`，即全量写入独立文件（不涉及 block 命名）
- 旧格式 block: `integration-codex-project-instructions`，新格式: `integration-project-instructions`
- 当前没有 block 移除逻辑：deselect capability 后会留下 orphan block

## Decision (ADR-lite)

1. **capabilities 对齐 Codex**: opencode 同样有 `ProjectInstructions` + `HostSkill`
2. **统一 ProjectInstructions**: 所有 agent host 共用 block ID `integration-project-instructions`，block 内容来自新通用源文件
3. **新建通用源文件**: `.omv/ai/adapters/project-instructions.md`，替代现有的 `codex/AGENTS.md` 作为 agent 通用指令
4. **自动迁移**: `omv integrate apply` 时检测并移除旧格式 block `integration-codex-project-instructions`

## Requirements

### A. 统一 ProjectInstructions managed block
- [ ] 修改 `write_integration_managed_block()`（`src/app/mod.rs`）：对 `ProjectInstructions` capability，block ID 改为 `integration-project-instructions`（去掉 provider 前缀）
- [ ] 创建通用源文件 `.omv/ai/adapters/project-instructions.md`（内容通用，标题为 "OMV Agent Instructions" 而非 "OMV Codex Adapter"）
- [ ] 修改 `integration_target(Codex, ProjectInstructions)` 指向新通用源文件
- [ ] 在 `canonical_sources()` 中新增 `project-instructions.md`

### B. 旧格式迁移
- [ ] `apply_integration_capability()` 中，写入 `integration-project-instructions` block 后，扫描并移除旧 `integration-codex-project-instructions` block
- [ ] 迁移自动触发，无需用户手动操作

### C. 新增 opencode provider
- [ ] `src/core/integration.rs`: 新增 `IntegrationProvider::OpenCode` variant + `as_str()` / `parse()`
- [ ] `src/core/integration.rs`: `mvp_provider_descriptors()` 中新增 opencode descriptor（`AgentHost` + `BootstrapLightweightHost` + capabilities: `ProjectInstructions` + `HostSkill`）
- [ ] `src/core/adapter.rs`: 新增 `AgentAdapter::OpenCode`（legacy）
- [ ] `src/adapter.rs`: 新增 opencode 的 `CanonicalTarget` 映射
- [ ] `src/app/mod.rs`: 新增 `integration_target(OpenCode, ProjectInstructions)` → 通用源文件 → `AGENTS.md`；`(OpenCode, HostSkill)` → `.opencode/skills/omv-versioning/SKILL.md`
- [ ] `src/ui/discovery.rs`: 检测 `.opencode/` 目录存在
- [ ] `src/ui/runtime.rs`: 新增 `provider_label(OpenCode)` → i18n key
- [ ] `resources/i18n/en-US.toml` + `zh-CN.toml`: 新增 `integration.provider.opencode`

### D. 新建 opencode adapter 源文件
- [ ] 创建 `.omv/ai/adapters/project-instructions.md`（通用 ProjectInstructions）
- [ ] 创建 `.omv/ai/adapters/opencode/SKILL.md`（HostSkill，内容参考 codex/SKILL.md）
- [ ] 在 `canonical_sources()` 中添加这两个源文件

### E. contract.json / canonical artifacts
- [ ] 更新 `ensure_canonical_artifacts()` 中的 `integration_model.providers` 加入 opencode 条目

## Acceptance Criteria

- [ ] `omv integrate status --json` 列出 opencode provider 及其 capabilities
- [ ] TUI 中可见 opencode 选项，可独立勾选/取消
- [ ] 同时勾选 Codex 和 OpenCode 时，AGENTS.md 中只有 1 个统一 ProjectInstructions block
- [ ] 已有 `integration-codex-project-instructions` block 的项目，运行 `omv integrate apply` 后自动迁移为新 block
- [ ] `omv integrate apply --json` 正确安装 opencode adapter 文件
- [ ] 检测到 `.opencode/` 时标记为 detected
- [ ] `omv integrate apply --json` 正确安装 opencode HostSkill 到 `.opencode/skills/omv-versioning/SKILL.md`
- [ ] lint / type-check / test 通过

## Definition of Done

- Tests 覆盖新增 + 迁移逻辑
- Lint / typecheck / CI green
- i18n en-US + zh-CN 同步

## Out of Scope

- 不修改 TUI 多选逻辑（已有）
- 不新增 capability 类型
- 不修改 `contract.json` 结构（由代码静态生成）
- 不修改 `HostSkill` 的 block 命名（HostSkill 是 DedicatedFile，不使用 managed block）
- 不处理 Claude Code 的旧 managed block 迁移（Claude 当前是 legacy adapter，不在 integration model 中）

## Technical Notes

- Block ID 生成：`src/app/mod.rs:1618`
- Provider descriptors：`src/core/integration.rs:172-224`
- `integration_target()`：`src/app/mod.rs:1358-1398`
- 检测逻辑：`src/ui/discovery.rs:45-58`
- Provider label：`src/ui/runtime.rs:368-374`
- 旧 block 格式：`integration-codex-project-instructions`
- 新 block 格式：`integration-project-instructions`
- Codex adapter 参考：`.omv/ai/adapters/codex/AGENTS.md`，`codex/SKILL.md`
