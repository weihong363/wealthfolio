# SQLite Storage Agent Guide

本 crate 是 SQLite/Diesel 持久化实现。它实现 core repository trait，不定义业务语义。

## 范围控制

- 先看 core trait 和 model，再改 repository。
- 新表或字段必须新增 migration，不直接手改生成 schema。
- 不在 repository 里实现财务计算、主题判断或 provider 解析。
- 保持 up/down migration 对称。

## 常用入口

- Migrations：`migrations/*/{up.sql,down.sql}`
- Diesel schema：`src/schema.rs`
- Repository：`src/*/repository.rs`
- DB/write actor：`src/db/*`
- Market data：`src/market_data/*`
- Fund research：`src/fund_research/*`
- Market intelligence：`src/market_intelligence/*`

## 推荐验证

- `cargo test -p wealthfolio-storage-sqlite`
- 涉及 schema 时确认 `src/schema.rs` 已更新。
