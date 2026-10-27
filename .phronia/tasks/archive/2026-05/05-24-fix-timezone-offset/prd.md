# 修复 UTC+8 时区偏移未应用到逻辑日期

## Goal

`config.timezone = "UTC+8"` 被存储但从未在日期计算中使用。NTP 返回 UTC 时间，系统时间也基于 UTC 计算 `unix_days`，导致 `logical_date` 始终是 UTC 日期而非偏移后的本地日期。

## Root Cause

`validate_current_date()` 和 `LogicalDate::from_unix_days()` 没有接收/应用时区偏移。

## Fix

1. 解析 `timezone` 字符串提取偏移小时数（`"UTC+8"` → `8`）
2. 在 `LogicalDate` 新增 `from_unix_days_with_offset(days, offset_hours)` 方法
3. `validate_current_date()` 读取 `config.timezone` 并传递偏移

## Requirements

- [ ] 解析 `timezone` 字符串 → `offset_hours` 函数
- [ ] `LogicalDate::from_unix_days_with_offset()` 
- [ ] `validate_current_date()` 传入 offset
- [ ] 更新现有测试

## Acceptance Criteria

- [ ] UTC+8 时区：23日18:28 UTC → logical_date = 2026-05-24
- [ ] UTC+0 时区：行为不变
- [ ] lint / type-check / test 通过
