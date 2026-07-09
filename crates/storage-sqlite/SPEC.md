# SQLite Storage Spec

`crates/storage-sqlite` 保存本地应用数据，并实现 `crates/core` 定义的 repository trait。数据库是本地事实来源，但业务语义仍由 core 服务定义。

## 职责

- 管理 Diesel 连接、迁移和 schema。
- 实现 accounts、activities、assets、quotes、portfolio、fund research、market intelligence 等 repository。
- 提供本地优先数据读写，不引入云依赖。

## 数据库变更流程

1. 在 core model/trait 明确新语义。
2. 新增 migration 目录，包含 `up.sql` 和 `down.sql`。
3. 更新 Diesel schema。
4. 更新对应 `model.rs` 和 `repository.rs`。
5. 贯通 Tauri/Web/frontend。
6. 运行 storage 或相关 focused tests。

## 重要表域

- accounts/activities/assets：用户基础财务事实。
- lots/snapshots/valuation：持仓、批次和估值。
- market_data/quotes：行情和同步状态。
- fund_research：基金研究快照、内部持仓、主题暴露、提醒。
- stock classification overrides：用户手动覆盖的股票行业和主题。
- market_intelligence：资金流、轮动、证据和市场概览。
- spending：支出分类、规则、预算和事件。

## 边界

- Repository 不调用外部网络。
- Repository 不做 provider fallback。
- Repository 不把 unknown 分类替换成猜测分类。
- Migration 不应破坏已有本地用户数据；需要数据迁移时写显式 backfill。
