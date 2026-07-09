# Wealthfolio Project Spec

Wealthfolio 是本地优先的个人财务与投资组合应用。核心数据保存在 SQLite；桌面端通过 Tauri 调用 Rust 服务，Web/self-host 模式通过 Axum HTTP API 调用同一套 core 服务。前端负责交互和展示，不应复制后端财务计算语义。

## 产品边界

- Portfolio Holdings 表示用户自己的资产持仓。
- Activities 表示用户录入、导入或同步的交易和现金流事件。
- Assets/Securities 表示证券、基金、现金、负债和自定义资产等可引用对象。
- Market Data 表示外部行情、资产画像和 provider 配置。
- Fund Research 表示基金内部持仓穿透、主题暴露和研究数据；不要把基金内部持仓写入用户 holdings。
- Market Intelligence 表示市场级别的资金流、轮动、情绪和组合暴露分析。
- Insights/Performance/Net Worth/Spending 表示从活动、持仓、行情和快照派生出的分析视图。

## 非目标

- 不把前端展示状态作为财务事实来源。
- 不在页面组件里硬编码行情、行业、主题或基金持仓规则。
- 不把 Provider 爬虫变成业务分析模块。
- 不自动交易，不输出确定性买卖建议。
- 不为单个页面问题重构跨端架构。

## 运行架构

```text
React UI
  -> apps/frontend/src/adapters
  -> Tauri IPC 或 Web HTTP
  -> apps/tauri/src/commands 或 apps/server/src/api
  -> crates/core 服务和 trait
  -> crates/storage-sqlite repository
  -> SQLite

Market/fund data:
crates/core quotes/fund_research/market_intelligence
  -> crates/market-data provider registry
  -> external provider
```

## 变更入口索引

| 需求类型 | 优先入口 | 其次检查 |
| --- | --- | --- |
| 页面展示、交互、i18n | `apps/frontend/src/pages`、`apps/frontend/src/features` | `apps/frontend/src/components`、`apps/frontend/src/lib/i18n` |
| 前端调用后端 | `apps/frontend/src/commands`、`apps/frontend/src/adapters` | `apps/tauri/src/commands`、`apps/server/src/api` |
| 账户、活动、资产、组合业务规则 | `crates/core/src/{accounts,activities,assets,portfolio}` | `crates/storage-sqlite/src/*` |
| 行情刷新、Provider、symbol 解析 | `crates/core/src/quotes`、`crates/market-data` | `crates/storage-sqlite/src/market_data` |
| 基金研究、穿透、主题暴露 | `crates/core/src/fund_research` | `crates/market-data/src/provider/eastmoney_fund`、`apps/frontend/src/pages/fund-research` |
| 股票画像、行业主题分类 | `crates/core/src/assets`、`crates/market-data/src/provider/eastmoney_stock` | `crates/storage-sqlite/src/assets` |
| 市场情报和资金轮动 | `crates/core/src/market_intelligence` | `apps/frontend/src/pages/market-intelligence` |
| 数据库字段或表 | `crates/storage-sqlite/migrations` | `crates/storage-sqlite/src/schema.rs`、对应 repository |
| 桌面命令 | `apps/tauri/src/commands` | `apps/tauri/src/context` |
| Web/self-host API | `apps/server/src/api` | `apps/server/src/main_lib.rs` |
| Addon 能力 | `apps/frontend/src/addons`、`crates/core/src/addons` | `packages/addon-sdk` |

## 重要模块文档

- Frontend 页面索引：`apps/frontend/src/pages/SPEC.md`
- Frontend commands：`apps/frontend/src/commands/SPEC.md`
- Frontend adapters：`apps/frontend/src/adapters/SPEC.md`
- Frontend agent 边界：`apps/frontend/AGENTS.md`
- Core 业务索引：`crates/core/SPEC.md`
- Core agent 边界：`crates/core/AGENTS.md`
- Activities：`crates/core/src/activities/SPEC.md`
- Assets：`crates/core/src/assets/SPEC.md`
- Portfolio：`crates/core/src/portfolio/SPEC.md`
- Quotes：`crates/core/src/quotes/SPEC.md`
- Fund Research：`crates/core/src/fund_research/SPEC.md`
- Market Intelligence：`crates/core/src/market_intelligence/SPEC.md`
- Market Data：`crates/market-data/SPEC.md`
- SQLite：`crates/storage-sqlite/SPEC.md`
- Tauri：`apps/tauri/SPEC.md`
- Server：`apps/server/SPEC.md`

## 修改原则

- 先从用户提到的页面、模块或命令开始，不做全仓库扫描。
- 能在当前层解决的问题，不跨层改动。
- 前端只做展示和调用编排；核心业务规则进入 `crates/core`。
- Provider 只负责获取和标准化外部数据；衍生分析进入 core 服务。
- 新字段从 core model/trait 开始，向 storage、Tauri/Web、frontend 逐层贯通。
- 任何迁移都要有 `up.sql` 和 `down.sql`，并更新对应 Diesel schema。

## 推荐验证

- 前端展示或 i18n：`pnpm type-check`
- Rust 业务逻辑：`cargo test -p wealthfolio-core`
- 行情/Provider：优先运行 provider 或 quotes focused tests，再视影响运行 `cargo test`
- 数据库迁移：`cargo test -p wealthfolio-storage-sqlite`
- 跨端 API：同时确认 Tauri command 和 Web handler 编译
