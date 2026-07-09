# Frontend Adapters Spec

`src/adapters` 根据构建目标连接 Tauri IPC 或 Web HTTP API。`src/adapters/index.ts` 默认导出 Tauri 类型，构建时由 Vite alias 切换到实际实现。

## 职责

- `tauri/*`：调用 Tauri commands。
- `web/*`：调用 `/api/v1/*`。
- `shared/*`：两端共用的 mapper、proxy 或工具。
- `index.ts`：统一 re-export。

## 边界

- Adapter 不做页面状态管理。
- Adapter 不做财务计算。
- Adapter 不做 provider 业务判断。
- 两端返回结构应尽量一致，差异在 adapter 内收敛。

## 修改提示

- 新 command/API 要同时补 Tauri 和 Web adapter。
- 如果只支持桌面或只支持 Web，返回清晰错误或能力标记。
- 与东方财富相关的浏览器 CORS/proxy 逻辑集中在 shared 或 server proxy，不散落页面。
