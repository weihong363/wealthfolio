# Tauri Spec

`apps/tauri` 是桌面应用壳和 IPC 层。它装配本地数据库、provider registry、scheduler、secret store 和 core services，并向前端暴露 Tauri commands。

## 职责

- 初始化桌面应用上下文。
- 暴露 command 给前端 adapter。
- 管理本地 secret、菜单、更新器、事件监听和 scheduler。
- 调用 core service，不复制业务规则。

## Command 索引

- `commands/portfolio.rs`：组合概览、performance、holdings、net worth。
- `commands/activity.rs`：活动 CRUD、导入、编辑。
- `commands/asset.rs`：资产、证券、画像和 provider 配置。
- `commands/market_data.rs`：行情刷新和同步。
- `commands/fund_research.rs`：基金研究和组合穿透。
- `commands/market_intelligence.rs`：市场情报。
- `commands/spending.rs`：支出模块。
- `commands/settings.rs`、`providers_settings.rs`：设置和 provider 配置。

## 边界

- Command 返回前端需要的 DTO，但 DTO 不应改变 core 语义。
- Tauri 专属错误要映射为可展示错误，不吞掉 core 诊断。
- 与 Web 共用的能力必须同时检查 `apps/server/src/api`。
