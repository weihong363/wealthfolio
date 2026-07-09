# Frontend Agent Guide

本目录是 Wealthfolio React 前端。修改前先定位具体 route、page、command wrapper 或 adapter，不要为了一个页面问题全局扫描。

## 范围控制

- UI 问题优先看 `src/routes.tsx` 中对应页面，再看页面直接引用的组件。
- 数据不刷新或来源异常时，先看页面 hook，再看 `src/commands` 和 `src/adapters`。
- i18n 问题先看可见页面文案和 `src/lib/i18n/locales/{en,zh-CN}.json`。
- 不在组件里硬编码行业、主题、行情、基金持仓或收益计算规则。
- 不把后端字段名、provider 细节直接扩散到多个页面；需要转换时放在局部 mapper/helper。

## 常用入口

- 路由：`src/routes.tsx`
- 页面：`src/pages/*`
- Feature 页面：`src/features/*/pages`
- 后端调用：`src/commands/*`
- 运行环境适配：`src/adapters/{tauri,web,shared}`
- 通用组件：`src/components/*`
- i18n：`src/lib/i18n/*`
- 表单 schema：`src/lib/schemas.ts`
- Addon runtime：`src/addons/*`

## 修改提示

- 新增页面时同步更新 `src/routes.tsx` 和导航入口。
- Tauri/Web 都需要的调用应先放 `src/commands`，再由 adapter 分发。
- 展示中文时使用 i18n key，不新增硬编码中文或英文。
- 图表问题优先检查传入数据形状、时间排序、空值过滤和单位格式化。
- 表格行点击、编辑弹窗、详情页共享数据时，优先复用已有 query key 和 mutation invalidation 模式。

## 推荐验证

- `pnpm type-check`
- 触及测试文件或纯函数时运行对应 Vitest focused test。
- 大面积 UI 变更再跑浏览器或 Playwright 验证。
