# Design — OMV 配置能力门控

## 问题本质

OMV 的配置 schema 是**二进制私有**的：`.omv/config.toml` 只是键值文本，能接受哪些值完全取决于跑的是哪个版本的 omv。版本错配时，旧二进制对**部分**未知值（如 `release_mode = "sometimes"`）会报 `InvalidReleaseMode`，对**另一部分**（如 `"pipeline"`，旧版本不认识但合法）则静默回落到 `Default`。

后者是最危险的：配置读成功了、命令退出码为 0、行为却是错的。这类缺陷无法靠提示词解决，因为出问题的是二进制本身的能力边界。

## 分层

```
core/versioning/capability.rs   能力声明：枚举取值 -> 引入版本
        |
storage/config.rs              解析时校验：取值是否被当前二进制支持
        |
errors.rs                      稳定的失败契约（reason_code + details）
```

三层职责单一：能力层只描述"什么东西从哪个版本开始存在"；存储层只做"当前值是否被支持"的判定；错误层只负责把判定结果转换成可诊断的输出。不把判断逻辑写进 `ReleaseMode::parse` 之类的原始解析函数，解析只管语法，门控管语义。

## 核心抽象

```rust
/// 一个配置取值的可用性声明。
pub struct Capability {
    /// 该取值首次出现的 omv 版本。
    introduced_in: &'static str,
}
```

每个受门控的枚举提供 `const SUPPORTED: &[(&str, Capability)]`（或用 trait 提供）。校验器：

```rust
pub fn ensure_supported(
    field: &'static str,
    value: &str,
    table: &[(&'static str, Capability)],
    current_version: &str,
) -> Result<(), CapabilityError>
```

命中则通过；未命中则返回 `CapabilityError::UnsupportedValue { field, value, current_version, min_version }`。

## 版本比较

版本号形如 `2611.2.1`（年-月.日.序号）与 `2610.27.1`。它们**不是语义化版本**，不能直接套 semver 库（虽然三段式可解析）。需要一个小而明确的比较器：

- 按 `.` 切分，逐段比较整数；
- 段数不等时缺省补 0（`2611.2` == `2611.2.0`）；
- 非数字段返回解析错误而非静默。

实现为 `core/versioning/capability.rs` 内的私有函数 `compare_versions(a, b) -> Option<Ordering>`，不引入新依赖。

## 与 `OMV_VERSION` 的关系

`src/generated/version.rs::OMV_VERSION` 已是编译期常量，是唯一真相源。门控在需要报错时才比对版本，**常规路径不调用版本比较**（只查表命中即可），避免给每次 `load_config` 增加无谓开销。

## 失败语义

| 情况 | 行为 |
| --- | --- |
| 键缺失 | 沿用该键默认值，不报错 |
| 值在支持表内 | 通过 |
| 值不在支持表内，但"当前版本 >= 该值引入版本" | 视为拼写错误，报 `Invalid<Field>`（与现状一致） |
| 值不在支持表内，且"当前版本 < 该值引入版本" | 报 `UnsupportedCapability`，提示升级 |

第四行是关键区分：同样是"不认识的值"，一个是你打错了，一个是你该升级了。错误消息不同，用户动作也不同。

## 兼容性

- `ReleaseMode`/`BuildPolicy`/`VersionOutput` 所有已发布取值登记为基线（`introduced_in` 设为一个早于任何发布的版本，如 `2600.1.1`，语义上表示"自始存在"）。
- 门控只读取配置，不写配置，不迁移。
- local 模式的产物（instructions/contract）完全不受影响，改动被限制在配置解析与 pipeline 分支。

## 提示词收敛

现有 `pipeline_safe_source` 用字符串 `.replace()` 做替换，脆且难维护——一旦源文本改一个字就静默失效。本次不重写整套机制（超出范围），但把 pipeline 下 finalize-boundary 的替换改为"生成期分支"：在 `canonical_sources` 生成 source 文本时按 mode 选择句子，而非事后替换。这消除了一类隐性失配。

同时 `contract.json` 的 `finalize_boundary` 段按 mode 注入只读标记，使"只读 contract 判断能力"的 agent 不再误判。
