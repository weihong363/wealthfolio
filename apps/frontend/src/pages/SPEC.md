# Frontend Pages Spec

`src/pages` 是主应用页面层，负责 route 级 UI、页面状态、query 调用和用户交互编排。业务事实和财务计算应来自后端 command/API，不在页面里重新实现。

## Route 索引

| Route | 页面入口 | 主要数据域 |
| --- | --- | --- |
| `/dashboard` | `dashboard/portfolio-page.tsx` | 组合概览、账户摘要、目标、持仓排行 |
| `/activities` | `activity/activity-page.tsx` | 活动记录、收入、支出 tab |
| `/activities/manage` | `activity/activity-manager-page.tsx` | 活动管理和导入后处理 |
| `/holdings` | `holdings/holdings-page.tsx` | 用户当前持仓 |
| `/holdings/:assetId` | `asset/asset-profile-page.tsx` | 单个资产/基金/股票详情 |
| `/fund-research` | `fund-research/fund-research-page.tsx` | 基金研究、前十大持仓、组合穿透 |
| `/market-intelligence` | `market-intelligence/market-intelligence-page.tsx` | 市场情报、轮动、资金流 |
| `/performance` | `performance/performance-page.tsx` | 组合/账户/benchmark 表现 |
| `/insights` | `insights/portfolio-insights.tsx` | 分析 tab、配置、漂移 |
| `/income` | `income/income-page.tsx` | 分红和收入历史 |
| `/health` | `health/health-page.tsx` | 数据健康检查 |
| `/assistant` | `ai-assistant/ai-assistant-page.tsx` | AI 助手 |
| `/settings/*` | `settings/*` | 设置、行情、分类、账户、插件 |

## 页面边界

- `dashboard` 只做概览，不应成为业务聚合服务。
- `holdings` 只展示用户真实持仓，不展示基金内部穿透为用户持仓。
- `fund-research` 可以引用用户持仓做穿透分析，但基金内部持仓仍属于研究数据。
- `asset` 详情展示资产画像、行情、历史和 provider 配置；分类编辑应写回持久化资产信息。
- `performance` 的 symbol/benchmark 表现来自 quote history，不使用前端伪造收益。
- `insights` 聚合配置、分配、表现和支出分析，不直接改写底层活动语义。

## 修改提示

- 页面数据异常：先查该页面 hook/query，再查 `src/commands`。
- 文案遗漏 i18n：页面和子组件一起改，避免 tab 内仍保留英文。
- 新增 tab：同步 URL query、默认 tab、空状态和 i18n key。
- 新增图表：明确单位、空值策略、排序策略和 tooltip 格式化。

## 推荐测试

- 页面纯逻辑：运行同目录 `*.test.ts(x)`。
- 全局类型：`pnpm type-check`。
