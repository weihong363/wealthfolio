# Core Agent Guide

本目录是 Wealthfolio 业务语义层。这里定义 model、service 和 repository trait；不能依赖 Tauri、Axum 或 SQLite 具体实现。

## 范围控制

- 先定位对应 domain module，不要默认修改多个服务。
- 新业务规则优先放 service；repository trait 只表达持久化能力。
- Provider 原始字段不要直接污染 core model；先标准化成稳定语义。
- 不在 core 里写 UI 文案、HTTP 状态码或 Tauri command 参数。
- 财务计算改动必须明确影响 holdings、performance、income、net worth 或 allocation 中哪一类语义。

## 常用入口

- 账户：`src/accounts`
- 活动和现金流：`src/activities`
- 资产和分类：`src/assets`
- 用户组合：`src/portfolio`
- 行情协调：`src/quotes`
- 基金研究：`src/fund_research`
- 市场情报：`src/market_intelligence`
- 支出：`src/spending`
- 目标和退休规划：`src/goals`、`src/planning`
- 健康检查：`src/health`

## 修改提示

- 新增持久化字段：先改 core model/trait，再改 storage repository、migration、API/command、frontend。
- 新增分析能力：service 返回结构化结果，不返回面向页面的临时字符串。
- 缺失外部数据应降级为缺失字段或 warning，不应让整个分析失败。
- 用户持仓和基金内部持仓必须保持语义隔离。

## 推荐验证

- `cargo test -p wealthfolio-core`
- 影响 trait 时同时检查 `crates/storage-sqlite` 编译和测试。
