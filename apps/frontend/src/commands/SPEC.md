# Frontend Commands Spec

`src/commands` 是前端调用后端能力的稳定入口。页面不应直接调用 Tauri invoke 或 fetch API；应通过 commands 保持桌面和 Web 路径一致。

## 职责

- 暴露页面可调用的 TypeScript 函数。
- 统一参数和返回类型。
- 根据运行环境走 adapter。
- 隔离 Tauri/Web 细节。

## 修改提示

- 新增后端能力：先确认 Tauri command 和 Web API 是否都存在，再新增 command wrapper。
- 页面只需要调用 command，不关心 `RUN_ENV`。
- 不在 command 中实现业务计算；只做轻量参数转换。
- 返回类型应与 core/API DTO 对齐，避免页面自行猜字段。

## 常见排查

- 页面无数据：页面 hook -> command -> adapter -> Tauri/Web 入口。
- 桌面正常 Web 异常：优先比较 adapter 两条实现。
- 类型错误：先修 command 返回类型，再修页面消费点。
