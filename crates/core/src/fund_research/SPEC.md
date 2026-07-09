# Fund Research Spec

`fund_research` 管理基金研究数据：基金净值、基金内部持仓、持仓变化、行业/主题/资产配置、风险指标、公告、穿透和提醒。它不表示用户自己的 holdings。

## 职责

- `models.rs`：基金研究 snapshot、内部持仓、主题暴露、轮动信号、提醒等领域模型。
- `repository.rs`：基金研究持久化 trait。
- `service.rs`：刷新、查询、组合穿透、对外服务入口。
- `lookthrough.rs`：把基金内部持仓按用户基金市值穿透为底层暴露。
- `theme_mapping.rs`：主题映射和规则加载。
- `rotation.rs`：主题轮动信号。
- `alerts.rs`：基金经理、持仓变化、风险和公告提醒。
- `metrics.rs`：收益、回撤、集中度等计算。
- `position_provider.rs`：从用户组合读取基金持仓市值的抽象。

## 语义边界

- 用户自己的持仓仍在 `portfolio/holdings`。
- 基金内部十大持仓、行业配置、资产配置和区域配置属于 Fund Research。
- 组合穿透只引用用户当前实际持有的基金，不统计仅录入研究数据但未持有的基金。
- 手动修正股票行业/主题时，应作为资产分类覆盖持久化，不应只存在前端 localStorage。
- 主题映射必须可配置或来自数据源，不在前端写关键词兜底。

## 数据来源

- 基金净值、持仓、公告等 provider 数据优先来自 `crates/market-data/src/provider/eastmoney_fund`。
- 底层股票行业和画像优先走资产分类/股票 provider，如 `eastmoney_stock`。
- 缺失分类保持 unknown/unclassified，并提供可编辑持久化入口。

## 修改提示

- 增加展示字段：先确认 `FundResearchSnapshot` 或相关模型是否已有语义，再逐层贯通。
- 改穿透占比：先看 `lookthrough.rs`，确认使用用户基金当前市值加权。
- 改主题暴露：先看 `theme_mapping.rs` 和 `ThemeExposure` 来源字段。
- 改提醒：先看 `alerts.rs`，不要输出确定性买卖建议。

## 推荐测试

- 持仓穿透计算。
- 主题映射和主题暴露。
- 缺失数据容错。
- 用户组合主题暴露。
- 调仓提醒生成。
