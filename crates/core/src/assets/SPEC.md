# Assets Spec

`assets` 管理可被活动、持仓、行情和分析引用的资产对象，包括股票、基金、现金、负债和 alternative assets。资产分类和 provider 配置应持久化在资产域或相关表中。

## 职责

- `assets_model.rs`：资产主模型。
- `assets_service.rs`：资产 CRUD、查询、provider 配置。
- `asset_id.rs`：资产标识和 canonical id。
- `asset_resolution.rs`：资产解析。
- `classification_service.rs`：资产分类读取和覆盖。
- `auto_classification.rs`：自动分类规则。
- `alternative_assets_*`：非证券资产。

## 语义边界

- Asset 是可引用对象，不等于用户当前持仓。
- 行业、主题、sector、profile 等应作为资产信息或分类覆盖持久化。
- 用户手动覆盖优先于外部 provider 抓取结果。
- 未知分类应保持 unknown/unclassified，不在前端硬编码猜测。
- Fund 类型应与 equity 区分，避免基金被当成普通股票处理。

## 修改提示

- 资产详情页字段缺失：先查 asset model/service，再查 quote/profile provider。
- 分类编辑：必须写回持久化覆盖，并影响 Fund Research/Lookthrough 等消费方。
- 新 asset type：同步 Rust enum、DB migration、前端 schema、表单和显示。

## 推荐测试

- `assets_model_tests.rs`
- 分类服务和资产解析相关 focused tests。
