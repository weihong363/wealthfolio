# Market Data Spec

`crates/market-data` 提供 provider-agnostic 行情、搜索、资产画像和工具数据。它把外部 provider 差异收敛到稳定模型，供 `crates/core/src/quotes`、资产分类、基金研究和市场情报使用。

## 核心概念

- `InstrumentId`：应用内部的 provider-agnostic 标识。
- `ProviderInstrument`：provider 实际请求所需的 symbol、market、exchange 等参数。
- `Quote`：OHLCV 行情。
- `AssetProfile`：资产画像，如名称、行业、主题或交易所信息。
- `ProviderCapabilities`：provider 支持的资产类型、市场和能力。
- `ResolverChain`：把用户 symbol/canonical id 解析成 provider instrument。

## 模块索引

| 模块 | 职责 |
| --- | --- |
| `models` | 行情、画像、搜索、覆盖范围等公共模型 |
| `provider` | 各 provider 实现和 trait |
| `provider/eastmoney_fund` | 东方财富基金净值、持仓、公告、风险等 |
| `provider/eastmoney_stock` | 东方财富股票行情和画像 |
| `provider/yahoo` | Yahoo 行情和搜索 |
| `registry` | provider 选择、限流、熔断、诊断 |
| `resolver` | symbol、MIC、交易所后缀和 provider symbol 解析 |

## 边界

- 不知道用户持仓、账户、活动或组合权重。
- 不做买卖建议、主题轮动判断或组合穿透。
- 不把页面展示格式写进 provider 返回值。
- 不通过硬编码短期兜底隐藏 provider 数据缺失。

## 修改提示

- 新 provider：实现 trait，声明 capabilities，接入 registry，补 fixture/test。
- 新市场规则：优先改 resolver 和 exchange metadata。
- 新画像字段：先扩展模型，再由 core 决定如何持久化和覆盖。
- 东方财富链接规则变化：集中在对应 provider，不散落到 frontend。
