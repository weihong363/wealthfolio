# Market Data Agent Guide

本 crate 负责外部行情和资产画像 provider，不负责用户组合业务语义。

## 范围控制

- Provider 只能抓取、解析、标准化外部数据。
- 行业、主题、基金研究等衍生分析应回到 `crates/core`。
- 不为某个页面写一次性 symbol hack；新增规则应进入 resolver、provider capability 或持久化覆盖。
- 网络字段缺失时返回结构化缺失，不伪造默认值。

## 常用入口

- Provider trait：`src/provider/traits.rs`
- Provider 能力：`src/provider/capabilities.rs`
- Provider registry：`src/registry/provider_registry.rs`
- Symbol resolver：`src/resolver/*`
- Quote/profile/search 模型：`src/models/*`
- 东方财富基金：`src/provider/eastmoney_fund`
- 东方财富股票：`src/provider/eastmoney_stock`

## 推荐验证

- Provider 单测或 fixture test。
- 涉及 resolver 时测试 canonical id、provider symbol 和市场后缀互转。
- 不确定真实接口稳定性时，用 fixture 固定解析样例。
