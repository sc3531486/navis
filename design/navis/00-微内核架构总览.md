# Navis 架构概览

## 项目定位

Navis 是基于 Tauri 2 的通用桌面应用白板运行时（Microkernel Host）。底座只提供：
- 桌面白板容器（WhiteboardShell）
- Cordis 风格上下文服务总线（NavisContext）
- 动态命名插槽渲染器（SlotRenderer）
- 底层动态 RPC 路由分发器（ExtensionRegistry）

**底座不预设任何物理分区**（左侧栏、右侧栏、状态栏等），仅暴露：
- `root`：主视口（扩展自主决定布局）
- `overlay`：全局浮层（弹窗、悬浮菜单）

## 架构分层

```
┌─────────────────────────────────────────┐
│          Extensions (navis-code)        │
│  NavisPlugin → registerSlot('root',..)  │
├─────────────────────────────────────────┤
│          NavisContext (前端)             │
│  Services / Events / Slots / Commands   │
├─────────────────────────────────────────┤
│          ExtensionRegistry (后端)        │
│  DynamicRpcHandler / RPC Dispatch       │
├─────────────────────────────────────────┤
│          Tauri 2 + Webview              │
└─────────────────────────────────────────┘
```

## 前端核心

- `src/core/context.ts`：NavisContext 服务容器（DI + 事件总线 + 插槽 + 命令）
- `src/core/SlotRenderer.tsx`：动态命名插槽渲染，支持 Slot-in-Slot 递归
- `src/app/WhiteboardShell.tsx`：纯白板容器，root/overlay 根插槽

## 后端微内核

- `src-tauri/src/kernel/mod.rs`：ExtensionRegistry + DynamicRpcHandler
- `src-tauri/src/lib.rs`：单一 `navis_dispatch_rpc` 命令入口

## 万物皆插件

所有 Agent、Editor、Terminal、Git、LSP、MCP、Session、Settings 等业务代码全部收敛到 `extensions/navis-code/`。扩展通过 `NavisPlugin` 接口自主挂载布局树到 root 插槽。
