# Navis Go

Navis Go 是一个基于 Tauri 2 的通用桌面应用白板与扩展运行时。框架只提供窗口宿主、扩展发现与生命周期、Cordis 装配、能力注册、事件、权限、存储、IPC、流式通道和 UI 投影。

## 产品与扩展

Navis Code 是第一个产品，入口为 `extensions/navis-code/navis-code-ui.tsx`。其 Agent、AI 平台、会话、项目、任务、编辑器、终端、设置、知识库和记忆全部位于 `extensions/navis-code/<extension-id>/`，每个扩展包含 `extension.json`、`ExtensionUI/` 和 `ExtensionBackend/`。

`extensions/navis-demo/` 是独立的最小扩展示例。柜面系统、双录系统等其他产品应以新的扩展组合实现，不修改 Navis 通用框架来承载行业业务。

## 目录边界

- `src/`：通用宿主前端和扩展视图投影。
- `src-tauri/src/`：通用宿主后端和扩展运行时。
- `extensions/`：产品扩展和示例扩展的开发期分发源。

当前 `src/router/`、`src/layouts/`、部分 `src/stores/` 和 HostView 内置投影仍有 Navis Code 产品壳过渡代码；新的业务不得继续放入这些位置，迁移状态见 `ARCHITECTURE_REVIEW.md` 和 `MIGRATION-PLAN.md`。

## 开发

```bash
npm run dev
npm run build
npx tauri dev
cd src-tauri && cargo check
cd src-tauri && cargo test
```
