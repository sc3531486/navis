# Navis Go 架构审查

> 审查日期：2026-08-18
> 范围：通用 Navis 框架、`extensions/navis-code/` 产品套件及其文档边界。
> 原则：以当前目录和入口为事实；目标态与已完成项分开记录。

## 1. 结论

仓库已经完成 Navis Code 业务扩展的主要物理迁移：业务扩展位于 `extensions/navis-code/<extension-id>/`，每个扩展按 `extension.json`、`ExtensionUI/`、`ExtensionBackend/` 组织；`src-tauri/src/` 的顶层目录已经以 Kernel、Extension、Foundation、Security、App、UI 为主。

但前端宿主尚未完全业务无关。Navis Code 的工作台组合仍部分位于 `src/router/`、`src/layouts/`、`src/stores/` 聚合导出以及少量 `src/components/HostView/` 内置投影中。`extensions/navis-code/navis-code-ui.tsx` 已是产品入口，但它当前仍导入 `extensions/navis-code/ExtensionUI/src/router`，并承担产品样式组合。

因此当前状态是：**后端框架边界基本成立，前端产品壳仍在迁移中；不能把 `src/router` 和 `src/layouts` 写成通用框架层。**

## 2. 当前目录事实

### 2.1 通用框架

| 目录 | 当前职责 |
|---|---|
| `src/` | 宿主前端基础设施、扩展视图投影、通用壳和基础 UI |
| `src-tauri/src/kernel/` | Kernel、Cordis 原语、事件、注册表、策略和审计 |
| `src-tauri/src/extension/` | 扩展发现、清单、加载、生命周期、组件和技能 |
| `src-tauri/src/foundation/` | 配置、存储、IPC、日志、流和状态基础能力 |
| `src-tauri/src/security/` | 认证、沙箱、权限和安全策略 |
| `src-tauri/src/app/` | Tauri 启动与通用基础设施装配 |
| `src-tauri/src/ui/` | 通用扩展桥、路由、HostView、网络、存储和权限命令 |

### 2.2 Navis Code 产品

`extensions/navis-code/` 下的十个业务扩展承载 Agent IDE 的业务领域。产品入口为 `extensions/navis-code/navis-code-ui.tsx`；产品专属路由、布局、样式组合和跨扩展工作台编排应归属于该产品目录，而不是 Navis 宿主。

### 2.3 当前过渡区

以下位置存在 Navis Code 组合逻辑，属于待迁移内容：

- `src/router/`：当前仍是产品工作台的路由组合入口。
- `src/layouts/`：当前仍组合会话侧栏、Agent 状态、项目面板、编辑器生命周期等产品区域。
- `src/stores/`：部分文件仍是业务 store 的聚合 re-export 或产品状态协调器。
- `src/components/HostView/`：仍有少量 Navis Code 视图的内置投影和设置入口。

这些路径可以暂时保留以支持迁移，但不得继续新增业务；通用框架 API 必须通过扩展契约表达。

## 3. 边界判定

### 应留在 Navis 框架

窗口/白板宿主、扩展发现与生命周期、manifest 校验、Cordis Context/Fiber/Scope、能力注册与调用、事件总线、权限/沙箱、通用配置/存储/日志/流、HostView 投影、通用命令/菜单/对话框壳、通用 IPC。

### 应归属扩展

Agent 循环和提示词、模型 Gateway、MCP/LSP、会话消息、项目和工作树、编辑器、终端、任务、知识库、记忆、设置，以及任何行业业务流程。

### 不应成为框架特例

不得在 Navis 宿主中增加 `navis-code`、Agent、Session、Project、Terminal 等产品 ID 或业务类型的分支；扩展之间不得通过直接导入内部 store 建立隐式耦合。

## 4. 文档一致性问题

本次审计前的主要错误是：

1. 把 `src/router/index.tsx` 描述为通用框架的“所有路由入口”，实际它仍承担 Navis Code 产品组合。
2. 把 `src/layouts/` 描述为通用布局层，实际其内部包含产品工作台区域。
3. 使用迁移前的 `extensions/navis-agent-core`、`extensions/navis-session` 等顶层路径，忽略当前的 `extensions/navis-code/<id>` 嵌套结构。
4. 把“业务已迁移”写成“前端宿主已完全无业务依赖”，缺少过渡区说明。

## 5. 后续架构动作

1. 将 Navis Code 的路由、布局和工作台组合完整迁入 `extensions/navis-code/` 产品壳。
2. 将 `src/stores/` 中的业务聚合导出拆回各扩展，仅保留宿主状态和通用投影。
3. 将 HostView 中的产品内置投影改为 manifest/能力驱动的扩展 renderer 注册。
4. 增加文档和契约检查，禁止框架目录新增业务 import、产品 ID 特判和业务模块。

## 6. 验证口径

文档同步后，应以以下事实复核架构边界：

```text
rg -n "navis-code|agent|session|terminal|project|task|composer|gateway" src src-tauri/src
```

命中只能来自通用协议、桥接类型、迁移注释或必要的扩展投影；新增业务实现应出现在 `extensions/navis-code/` 对应扩展中。
