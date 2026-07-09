# Core Spec

`crates/core` 是数据库无关的业务核心。它定义领域模型、服务、错误和 repository trait。Tauri/Web 入口只能薄封装 core service，不应在边缘层复制业务规则。

## 模块索引

| 模块 | 职责 | 常见修改 |
| --- | --- | --- |
| `accounts` | 账户模型、余额和账户设置 | 新账户类型、账户字段、账户校验 |
| `activities` | 买卖、分红、费用、转账等活动语义 | 活动类型、导入解析、幂等、现金流 |
| `assets` | 证券/基金/负债/现金资产模型和分类 | asset type、行业主题覆盖、手动分类 |
| `portfolio/holdings` | 用户真实持仓和估值 | 持仓展示、份额、成本、市值 |
| `portfolio/performance` | TWR/MWR、benchmark、历史表现 | 收益算法、benchmark 数据、时间区间 |
| `portfolio/income` | 分红/利息/收入分析 | income 归集、税费、时间序列 |
| `portfolio/net_worth` | 净资产聚合 | 快照、账户净值、负债 |
| `portfolio/allocation*` | 配置目标、漂移、再平衡建议 | 配置模型、漂移阈值 |
| `quotes` | quote history、刷新策略、provider 协调 | 行情同步、staleness、symbol quote |
| `fund_research` | 基金研究和穿透 | 基金内部持仓、主题暴露、调仓提醒 |
| `market_intelligence` | 市场资金流和轮动分析 | 主题轮动、市场概览、组合暴露 |
| `spending` | 支出分类、预算和规则 | spending tab、分类规则、预算 |
| `goals` / `planning` | 目标和退休模拟 | FIRE、退休引导、目标进度 |
| `health` | 数据健康检查和修复 | quote stale、分类、转账一致性 |
| `addons` | 插件域能力 | addon 权限、事件和服务 |

## 数据流

```text
Command/API DTO
  -> core service input
  -> repository trait
  -> storage implementation
  -> core domain result
  -> Command/API DTO
```

## 边界

- Core 可以依赖 trait 和 provider 抽象，不依赖 SQLite schema。
- Core 不关心 React route、Tauri invoke 名称或 HTTP path。
- Core 中的时间、货币、数量计算要保持可测试。
- `portfolio/holdings` 不承载基金内部持仓穿透。
- 外部行情不可用时，返回空数据、诊断或 warning，避免伪造数据。

## 推荐测试

- 服务规则：同模块 `*_tests.rs`。
- 财务算法：构造最小活动/quote/position 数据，不依赖真实 provider。
- Provider 相关逻辑：优先 mock trait 或 fixture provider。
