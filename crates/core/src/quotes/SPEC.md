# Quotes Spec

`quotes` 协调行情读取、刷新、staleness 判断和 provider registry。它连接 core 业务和 `crates/market-data` provider，但不负责页面展示。

## 职责

- `service.rs`：quote 查询、刷新、symbol-only quote history。
- `client.rs`：provider registry 调用和错误转换。
- `errors.rs`：行情错误语义。
- 与 storage market_data repository 协作保存历史行情和同步状态。

## 语义边界

- Quote history 是行情事实来源，不是收益计算本身。
- Benchmark/symbol performance 使用 quote close price 计算价格收益。
- Provider 缺失数据时应返回空数据、诊断或明确错误，不伪造价格。
- 刷新策略要考虑交易日、staleness、provider 能力和手动刷新。

## 修改提示

- 今日收益没刷新：先查 quote sync freshness 和 refresh trigger。
- 图表线缺失：先确认 quote points 数量、日期排序、价格字段和过滤条件。
- 新 provider 接入：先在 market-data 实现 provider，再在 quotes/provider registry 使用。
- 用户手动刷新按钮：前端触发 quote refresh，不绕过 quotes service。

## 推荐测试

- quote service focused tests。
- provider registry/fixture tests。
- storage market_data repository tests。
