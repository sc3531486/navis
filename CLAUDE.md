# Navis Go 开发说明

## 项目定位

Navis Go 是通用 Navis 框架与 Navis Code 产品的同一仓库：

- `src/`、`src-tauri/src/` 是通用宿主框架，只提供桌面白板、扩展运行时和基础能力。
- `extensions/navis-code/` 是 Navis Code 产品套件，所有 Agent IDE 业务都在这里按扩展拆分。
- `extensions/navis-demo/` 是不依赖 Navis Code 的最小扩展示例。

柜面系统、双录系统等其他产品应复用 Navis 宿主能力，以新的产品扩展组合实现，不应修改框架层来承载业务。

## 常用命令

```bash
npm run dev
npm run build
npx tauri dev
npx tauri build
cd src-tauri && cargo check
cd src-tauri && cargo test
```

## 架构边界

### 通用 Navis 宿主

前端 `src/` 只负责通用 HostView 投影、命令/菜单/对话框壳、扩展桥、通用 UI、主题、国际化、热键和流式基础设施。后端 `src-tauri/src/` 只负责 Kernel、Cordis Context/Fiber/Scope、扩展加载与生命周期、能力注册、事件、配置、存储、IPC、权限、安全和 Tauri 启动。

### Navis Code 产品

Navis Code 的产品入口是 `extensions/navis-code/navis-code-ui.tsx`。产品业务扩展包括：

| 扩展 | 业务范围 |
|---|---|
| `navis-agent-core` | Agent 编排、上下文、Composer、时间线 |
| `navis-ai-platform` | Gateway、Provider、MCP、LSP、Skills |
| `navis-session` | 会话、聊天、消息、工作树关联 |
| `navis-project` | 项目、工作树、项目面板 |
| `navis-task` | 任务和子任务投影 |
| `navis-editor` | 编辑器、文件、Git、剪贴板 |
| `navis-terminal` | 终端和 PTY |
| `navis-settings` | 设置 |
| `navis-knowledge` | 知识库 |
| `navis-memory` | 记忆 |

每个扩展使用固定目录：`extension.json`、`ExtensionUI/`、`ExtensionBackend/`。

## 当前迁移事实

Navis Code 业务已物理迁入 `extensions/navis-code/`，后端框架顶层已收敛为通用目录。但前端仍有过渡性产品组合代码：`src/router/`、`src/layouts/`、部分 `src/stores/` 聚合导出和少量内置视图投影仍直接引用 Navis Code 扩展。它们不能被视为通用框架 API，新的业务不得继续添加到这些位置；后续迁移以 `ARCHITECTURE_REVIEW.md` 和 `MIGRATION-PLAN.md` 为准。

## 扩展通信

扩展通过 manifest contribution、HostView、命令投影、能力端口、Kernel EventBus、Tauri IPC、stream 和权限契约通信。扩展不得直接访问其他扩展的内部 store、数据库或组件实现。

## 开发规则

- Rust 日志统一使用 `tracing`。
- 前端状态使用 `createStore` 和纯 action 函数。
- Rust 模块文档和新增代码注释使用中文。
- 不把产品 ID、Agent、Session、Project、Terminal 等业务实现写入 Navis 宿主。
- 文档必须区分当前事实和目标态；不得以旧路径描述已迁移的业务。
