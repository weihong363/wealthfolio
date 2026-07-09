# Portfolio Spec

`portfolio` 从 activities、assets、quotes、accounts 和 snapshots 派生用户组合视图。这里表示用户自己的资产、收益、配置和净值，不承载基金内部研究数据。

## 模块索引

| 模块 | 职责 |
| --- | --- |
| `holdings` | 当前持仓、成本、市值、估值质量 |
| `performance` | TWR/MWR、benchmark、账户/组合表现 |
| `income` | 分红、利息、收入归集 |
| `net_worth` | 净资产时间序列 |
| `allocation` | 当前配置分析 |
| `allocation_targets` | 目标配置、漂移、再平衡建议 |
| `valuation` | 估值和快照相关逻辑 |
| `fire` | FIRE/退休基础计算 |

## 语义边界

- Holdings 只表示用户真实持有的资产。
- 基金内部股票持仓通过 `fund_research/lookthrough` 分析，不写入 holdings。
- Performance benchmark 使用 quote history 计算 symbol price return，不使用前端伪造序列。
- Income 为真实收入现金流；历史累计收益不应简单用 dividend 补录，除非它确实是分红收入。
- 再平衡建议只做分析，不执行交易。

## 修改提示

- 持仓份额/成本异常：先查 activities compiler 和 holdings valuation。
- 今日收益或行情日期异常：先查 quotes refresh 和 valuation quality。
- Performance 异常：区分 portfolio/account cash-flow 口径和 symbol benchmark price-only 口径。
- 配置漂移：先查 allocation_targets，不改 holdings。

## 推荐测试

- holdings valuation tests。
- performance focused tests。
- allocation target/rebalance tests。
