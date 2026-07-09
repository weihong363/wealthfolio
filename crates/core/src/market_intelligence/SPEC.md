# Market Intelligence Spec

`market_intelligence` 表示市场级别的情报分析，包括市场概览、资金流、主题轮动、行业轮动、组合暴露和证据。它可以引用 portfolio/fund research 数据，但不改变用户活动或持仓事实。

## 职责

- `service/mod.rs`：聚合入口和跨模块编排。
- `eastmoney.rs`：东方财富相关数据适配和转换。
- `market_overview`：市场宽度、指数、概览指标。
- `capital_flow`：资金流模型、provider、repository、service。
- `sector_rotation`：行业轮动模型和信号。
- `theme_rotation`：主题轮动模型和信号。
- `portfolio_exposure`：用户组合对主题/行业/底层资产的暴露。
- `evidence`：支撑结论的证据记录。
- `repository`：市场情报持久化入口。

## 边界

- 不执行交易。
- 不把市场观点写回 activities 或 holdings。
- 不在 provider 层做长期业务判断；provider 只返回标准化数据。
- 结论应带 evidence、时间戳和 source，便于 UI 解释。
- 数据缺失时输出降级状态，不伪造资金流。

## 修改提示

- 页面显示空数据：先看前端 hook，再看 `service/mod.rs` 聚合是否返回空集合。
- 东方财富字段变化：先看 `eastmoney.rs` 和 provider 输出契约。
- 组合暴露异常：先区分用户 portfolio exposure 与 fund research lookthrough。
- 新信号类型：新增模型、repository、service，再接 Tauri/Web 和 frontend。

## 推荐测试

- provider 数据转换。
- 轮动信号阈值。
- 组合暴露聚合。
- 缺失数据和空数据降级。
