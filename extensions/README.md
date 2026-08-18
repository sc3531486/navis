# 扩展统一根目录（extensions/）

> 权威规范见 `design/36-extension-development.md` 与 `design/35-whiteboard-container.md` §三。

## 固定目录契约

每个扩展是一个独立分发单元，目录名必须等于 `extension.json` 中的 `id`。仓库中的产品扩展使用一层产品目录包装，因此开发期路径是 `extensions/<product>/<extension-id>/`；安装到运行时后统一落在 `<app_data>/extensions/<extension-id>/`：

```text
extensions/<product>/<extension-id>/
├── extension.json                 # manifest（唯一扩展元数据入口）
├── ExtensionUI/                   # 全部前端扩展代码、样式和资源
│   ├── index.html                 # html:sandbox 视图入口（如有）
│   ├── assets/                    # 静态资源
│   ├── scripts/                   # 前端胶水 worker / 前端逻辑组件
│   └── locales/                   # i18n 资源
└── ExtensionBackend/              # 全部后端扩展点代码
    ├── src/                       # Rust 扩展源码（如有）
    ├── logic/                     # 容器内逻辑组件
    └── native/                    # 协议子进程服务（可选）
```

## 产品组合契约

产品不是宿主层的特判，而是扩展组合：

```text
extensions/<product>/
├── product.json                   # id/name/version/entry，可选 default
├── <product>-ui.tsx               # 导出 ProductDefinition
└── <extension-id>/                 # 业务扩展集合
```

通用入口 `src/index.tsx` 只扫描产品清单并动态加载被选中的入口。没有产品清单时只显示 Navis 白板。新增产品不修改 `src/`、`src-tauri/src/` 或宿主产品选择代码。

## 当前业务扩展清单

- `navis-demo`：白板容器最小扩展骨架和全链路示例。
- `navis-code/`：Navis Code 产品扩展套件。
  - `navis-ai-platform`：AI 平台服务扩展（Gateway / MCP / LSP / Skills）。
  - `navis-agent-core`：Agent 引擎扩展（编排、上下文和行为模式）。
  - `navis-session` / `navis-project` / `navis-task` / `navis-knowledge` / `navis-memory`：会话、项目、任务、知识库、记忆业务扩展。
  - `navis-terminal` / `navis-editor` / `navis-settings`：终端、编辑器、文件工具、Git、剪贴板和设置扩展。

## 说明

- 仓库 `extensions/` 是开发期分发源；扫描器递归查找 `extension.json`，运行时从 `<app_data>/extensions/<extension-id>/` 装载。
- 所有业务领域都走同一 manifest、桥、生命周期和沙箱契约，官方扩展无特权。
- 扩展后端源码不得反向引用宿主已经移除的业务命名空间；跨扩展通信只能使用公开能力端口、Kernel EventBus、IPC、stream 和权限契约。
- `src/` 与 `src-tauri/src/` 是通用 Navis 宿主；不得把 Navis Code 或其他产品业务写回框架目录。
