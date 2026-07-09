# Server Spec

`apps/server` 提供 Wealthfolio Web/self-host HTTP API。它和 Tauri command 调用同一套 core service，目标是保持桌面和 Web 行为一致。

## 职责

- 暴露 `/api/v1/*` REST API。
- 管理 auth、OIDC、CORS、rate limit 和 OpenAPI 基础定义。
- 装配 AppState、数据库、scheduler 和 domain events。
- 调用 core service 并返回 API DTO。

## API 模块索引

- `api/accounts.rs`、`api/portfolios.rs`：账户和投资组合。
- `api/activities.rs`：活动记录。
- `api/assets.rs`、`api/market_data.rs`：资产和行情。
- `api/portfolio.rs`、`api/holdings/*`、`api/performance.rs`、`api/net_worth.rs`：组合分析。
- `api/fund_research.rs`：基金研究。
- `api/market_intelligence.rs`：市场情报。
- `api/spending.rs`：支出模块。
- `api/settings.rs`、`api/custom_providers.rs`、`api/taxonomies.rs`：设置和分类。
- `api/eastmoney_proxy.rs`：东方财富相关代理接口。

## 边界

- API handler 不直接访问 provider；通过 core service。
- API handler 不直接访问 SQLite repository，除非已有 AppState/service 模式如此要求。
- 桌面和 Web 行为应保持一致；新增 Web endpoint 时同步考虑 Tauri command 或前端 adapter。
