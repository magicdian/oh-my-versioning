# OMV 配置能力门控与 pipeline 提示词收敛

## Goal

修复 bedrock 项目暴露的两个缺陷：旧版 omv 二进制读到 `release_mode = "pipeline"` 时静默降级为 `local` 并照常 bump（造成版本被本地改写），以及 pipeline 模式下注入给 agent 的提示词仍要求调用注定 no-op 的 finalize-boundary。

## Background

2026-11-06 在 `/Volumes/MagicWork/xp-non-aosp-codes/bedrock` 复现：

| 二进制 | `omv bump --json` |
| --- | --- |
| 全局安装 `@magicdian/omv@2610.27.2` | 成功执行，2611.5.6 → 2611.6.1，写入 `rust/Cargo.toml` |
| CI 同版本 `@magicdian/omv@2611.2.1` | 正确返回 `release_mode_denied` |

`release_mode` 是硬编码在二进制里的枚举，配置文件对它只是字符串。2610.27.2 的 `load_config` 用 `ReleaseMode::parse` 解析，pipeline 分支不存在，于是落到 `default()`（Local），守卫自然不生效。旧二进制并非"解析报错"，而是"认得 local，只是不认识 pipeline"——因此连 `InvalidReleaseMode` 都不会触发。

同类风险不限于 `release_mode`：`build_policy`、`version_output`、`project_profile` 以及未来的 `targets.toml` / `integrations.toml` 新字段都遵循同一模式——配置 schema 与二进制能力之间缺少显式协商。

## Requirements

### R1 配置能力门控（core 层）

引入一层能力声明机制，把"当前二进制支持的配置取值集合"与"配置文件里的实际取值"做显式比对。任一新版本引入的取值被旧二进制读到时，必须产生可诊断的硬失败，而不是退化为默认行为。

该机制应当是声明式的：新增枚举取值时，作者在单一位置登记其引入版本，而不是在每个消费点散落判断。

### R2 稳定的失败契约

失败必须走既有错误体系：新增错误变体 + 稳定的 `reason_code`，JSON 输出结构与现有 `{"ok": false, "error": {"code", "message", "details"}}` 一致。错误信息需同时给出当前二进制版本与所要求的最低版本，让人知道该升级。

### R3 pipeline 提示词收敛

pipeline 模式下生成的 AI 指令不得要求 agent 调用 finalize-boundary。替换后的文本应当明确"版本由发布流水线分配，本地无需任何版本写操作"，而不是仅仅陈述"finalize 事件是只读的"。

`contract.json` 中 `finalize_boundary` 段在 pipeline 模式下需反映其只读语义，使只读 contract 判断能力的 agent 也能得到正确结论。

### R4 不破坏既有行为

- `release_mode` 缺省时仍默认为 `local`（现有测试 `config_defaults_release_mode_to_local_when_absent` 必须继续通过）。
- local 模式生成的提示词与 contract 结构不变。
- 已发布的 2611.2.1 能正确解析的配置，本版本不得拒绝。

## Non-Goals

- 不改变 `release_mode` 的语义本身。
- 不引入运行时插件或外部 schema 拉取。
- 不处理 native manifest 的版本写入策略。
- **不追溯保护历史状态。** 一份由旧版 omv 写出、且不含 `requires_omv` 的配置，配上旧版二进制，行为不变——旧二进制不认识这个键，会忽略它。本机制保护的是"发布该键的版本之后"的所有版本。历史项目通过 `omv integrate apply` 获得声明后才进入保护范围。

## Acceptance Criteria

- [ ] `release_mode` 取尚未支持的未来取值时，`load_config` 返回携带 `reason_code` 的错误，消息中含当前版本与最低要求版本。
- [ ] `build_policy`、`version_output` 同样受门控保护（与 `release_mode` 走同一机制，非三份手写分支）。
- [ ] 缺少某个受门控键时仍沿用该键既有默认值，且不触发门控失败。
- [ ] pipeline 模式生成的 `.omv/ai/instructions.md` 不含 "call the OMV finalize-boundary helper" 字样。
- [ ] pipeline 模式生成的 `contract.json` 中 `finalize_boundary` 标注为只读/pipeline 归属，且 `bump`/`sync`/`finalize_boundary` 不再以可写命令形式出现在 `write` 下。
- [ ] pipeline 模式下 Trellis 与 Phronia 的 finish-work 托管块不包含任何 bump 指令（Trellis v0.4 注入点尤为关键）。
- [ ] local 模式的 `instructions.md` 与全部 `ai/adapters/*` 与改动前逐字节一致；`contract.json` 除新增的 `release_mode_behavior` 键外与改动前一致（按模式计算的字段无法让该文件整体不变）。
- [ ] `omv integrate apply` 能为缺少 `requires_omv` 的历史项目回填该键，且重复执行不产生改动。
- [ ] 畸形 `requires_omv` 返回 `malformed_config_requirement`，不得 fail-open 放行。
- [ ] `cargo test` 全绿；`cargo clippy` 无新增告警。

## Notes

- 门控的"引入版本"信息属于编译期常量，不得在运行时读取外部文件。
- 版本常量应复用 `src/generated/version.rs` 的 `OMV_VERSION`，避免第二处真相源。
