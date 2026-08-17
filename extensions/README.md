# 扩展统一根目录（extensions/）

> 权威规范见 `design/36-extension-development.md` 与 `design/35-whiteboard-container.md` §三。

## 固定目录契约（铁律）

每个扩展是一个独立分发单元，目录名必须等于 `extension.json` 中的 `id`：

```
extensions/{id}/
├── extension.json                 # manifest（唯一元数据入口）
├── ExtensionUI/                   # ★ 前端扩展面：全部前端代码
│   ├── index.html                 # html:sandbox 视图入口
│   ├── assets/                    # 静态资源
│   ├── scripts/                   # 前端胶水 worker / 前端逻辑组件 .wasm
│   └── locales/                   # i18n 资源
└── ExtensionBackend/              # ★ 后端扩展面：全部后端扩展点代码
    ├── logic/                     # 容器内 WASM 组件轨（components[kind:logic]）
    └── native/                    # 协议子进程逃生舱（contributes.backendServices[]）
```

## 当前业务扩展清单

- `navis-demo`：白板容器最小扩展骨架（已实现全链路演示）。
- `navis-ai-platform`：AI 平台服务扩展（Gateway / MCP / LSP / Skills）。
- `navis-agent-core`：Agent 引擎扩展（turn 编排 / 上下文组装 / 行为模式）。
- `navis-session` / `navis-project` / `navis-task` / `navis-knowledge` / `navis-memory`：会话、项目、任务、知识库、记忆业务扩展。
- `navis-terminal` / `navis-editor` / `navis-settings`：终端、编辑器、设置 UI 扩展。

## 说明

- 仓库 `extensions/` 是**分发源**；运行时从 `<app_data>/extensions/{id}/` 装载。
- 所有业务领域都走同一 manifest / 桥 / 生命周期 / 沙箱契约，官方扩展无特权。