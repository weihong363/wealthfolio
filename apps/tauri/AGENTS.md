# Tauri Agent Guide

本目录是桌面端边缘层。Tauri command 应保持薄封装，把业务逻辑交给 `crates/core`。

## 范围控制

- Command 负责参数转换、错误映射和调用 service。
- 不在 command 中实现财务计算、行情规则或页面逻辑。
- 新 command 需要在 `src/commands/mod.rs` 和 invoke handler 中注册。
- 桌面专属能力放 `src/context`、`src/services` 或平台模块，不污染 core。

## 常用入口

- Commands：`src/commands/*`
- App context：`src/context/*`
- Provider registry：`src/context/providers.rs`
- Domain events：`src/domain_events/*`
- Scheduler：`src/scheduler.rs`
- Secret store：`src/secret_store.rs`

## 推荐验证

- Rust 类型检查或相关 `cargo test`。
- 共享 API 变更时确认 Web handler 也同步。
