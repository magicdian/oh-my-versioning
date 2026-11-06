# Implement — OMV 配置能力门控

## 顺序

1. **新增 `src/core/versioning/capability.rs`**
   - `Capability { introduced_in }`
   - `CapabilityError::UnsupportedValue { field, value, current_version, min_version }`
   - `compare_versions` 与 `is_at_least`
   - `ensure_supported`
   - 单测：版本比较、命中、未命中（两个方向）

2. **为受门控枚举登记支持表**
   - `ReleaseMode::supported_capabilities()`
   - `BuildPolicy::supported_capabilities()`
   - `VersionOutput::supported_capabilities()`
   - 基线取值 `introduced_in = "2600.1.1"`；`pipeline` 用真实引入版本

3. **接入 `storage/config.rs`**
   - `release_mode` / `build_policy` / `version_output` 三个分支改为"先门控、再 parse"
   - 键缺失分支保持不动

4. **扩展 `errors.rs`**
   - `ConfigError::UnsupportedCapability { .. }`
   - `Display` 实现：同时给出当前版本与最低版本
   - `reason_code`：`unsupported_config_capability`

5. **提示词收敛（`src/adapter.rs`）**
   - `canonical_sources` 生成 finalize 指引时按 mode 分支，pipeline 下直接给出"无需本地版本操作"
   - `contract.json` 的 `finalize_boundary` 段在 pipeline 下注入 `read_only_in_release_mode`

6. **验证**
   - `cargo test`
   - `cargo clippy --all-targets`
   - 用 bedrock 配置在临时目录复现"旧版本拒绝、新版本放行"的对照

## 验收命令

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

## 回滚点

- 提交粒度：能力层 → 接入层 → 提示词层，各自独立可回滚。
- 任一阶段失败，`git revert` 该阶段提交即可，不影响其余。
