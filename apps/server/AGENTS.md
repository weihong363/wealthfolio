# Server Agent Guide

本目录是 self-host/Web 模式的 Axum API。API handler 应保持薄封装，业务逻辑交给 `crates/core`。

## 范围控制

- Handler 负责 path/query/body 解析、auth、错误映射和 service 调用。
- 不在 handler 中写财务计算、行情规则或前端展示逻辑。
- 新 API route 需要接入对应模块 `router()`，并在 `src/api.rs` merge。
- 与桌面共用的能力必须同步检查 Tauri command。

## 常用入口

- Router 聚合：`src/api.rs`
- App state：`src/main_lib.rs`
- API modules：`src/api/*`
- Auth/OIDC：`src/auth.rs`、`src/oidc.rs`
- Domain events：`src/domain_events/*`
- Scheduler：`src/scheduler.rs`

## 推荐验证

- Rust 类型检查或相关 API test。
- 共享 command/API 变更时确认 frontend adapter 的 Tauri/Web 两条路径。
