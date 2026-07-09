# Activities Spec

`activities` 定义用户录入、导入或同步的财务事件。Activities 是 holdings、income、performance、cash balance 等派生结果的重要事实来源。

## 职责

- `activities_model.rs`：活动类型、字段、状态和 DTO。
- `activities_service.rs`：活动 CRUD、校验和业务规则。
- `compiler.rs`：把活动编译为持仓/现金流可消费的事件。
- `csv_parser.rs`：CSV 导入解析。
- `idempotency.rs`：导入幂等和重复识别。
- `transfer_pairs.rs`：转账配对。
- `import_run_model.rs`：导入批次记录。

## 语义边界

- Buy/Sell 表示真实交易，应影响份额、成本和现金。
- Dividend/Interest/Income 表示收入现金流，不应用来伪造买入成本。
- Fee/Tax 应明确影响现金和收益口径。
- 定投应表现为多笔 Buy 活动；金额变化不需要新增特殊功能，除非要做计划/自动生成。
- 编辑活动时必须保留关联资产/账户语义，不应只显示 code。

## 修改提示

- 新活动类型：同步 model、service、compiler、前端 schema 和活动文档。
- 收益异常：先确认活动是否被 compiler 正确转换，再查 performance/income。
- 导入问题：先看 CSV parser 和 idempotency，不直接改 holdings。

## 推荐测试

- `activities_service_tests.rs`
- `activities_model_tests.rs`
- 涉及持仓影响时同时跑 portfolio holdings/performance focused tests。
