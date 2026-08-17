# Navis Go 架构审查报告

> 原始审查日期：2026-08-13
> 当前同步日期：2026-08-14
> 同步规则：仅根据当前代码事实修正文档；已完成项移出活动建议；未完成项保留为当前事实；不写兼容方案。

---

## 1. 总体结论

2026 年 8 月 13 日版本的审查报告里，有一批问题已经在代码中完成修正，因此不应再继续作为活动建议保留：

- 前端双重 Router 已移除。`src/App.tsx` 现在只负责挂载，唯一 `Router` 在 `src/router/index.tsx`。
- 前端本地注入体系已退出主链路，当前扩展 UI 入口唯一是 `contributes.views` + `HostView` surface。
- LSP 的 Tauri command 已统一收敛到 `src-tauri/src/ui/lsp.rs`，旧的 `tool/lsp/commands.rs` 事实已失效。
- `TimelineStatus` 与 `SessionEvent` 的存储归属已调整，旧的路径与依赖判断需要同步。
- Agent/MCP ownership 路径上原先的 `unsafe transmute` 建议已经失效。
- `tool/agent/pipeline/runner.rs` 已改为 `policy.overlay([extension_hooks])`，旧的“约束只累积不移除”判断已不成立。
- `tool/cron` 模块已删除，不能继续作为当前待修项保留。

当前仍需保留的架构主线是一条：

1. `ai/agent` 与 `tool/agent` 的运行时事件合同仍然偏紧；工具目录已先收敛为 `ToolAvailability` 能力接口，Todo/Sidechain application port 已建立并接入特殊工具，剩余 `AgentToolEvent` 主链路接线仍需单独完成。


## 2. 已完成同步项

### 2.1 Router 已收敛为单入口

当前事实：

- `src/App.tsx:16` 仅执行 `render(() => <AppRoutes />, root!)`
- `src/router/index.tsx:69` 承担唯一 `<Router root={MainLayout}>`

结论：原报告中的“双重 Router 嵌套”已完成修复，应从活动问题和优先建议中移除。

### 2.2 前端扩展 UI 已切到 HostView surface

当前事实：

- 前端扩展 UI 由 `HostView` surface 统一承接，不再保留独立的扩展组件入口
- `src/layouts/MainLayout.tsx:171,174` 使用 `HostViewSurface`
- `src/stores/menu-actions.ts` 与 `src/stores/app.ts` 承担 view 打开与 placement 管理

结论：原报告中的“扩展注入系统空转”描述已经过时。当前唯一模型是 `contributes.views`、HostView surface、宿主 renderer。

### 2.3 扩展 command/keybinding 投影已收敛到声明式 UI contract

当前事实：

- `src-tauri/src/ui/menus.rs::ui_list_extension_commands` 从已启用扩展 manifest 投影用户可见 command，并使用 `<extension_id>/<command_id>` namespaced command ID。
- `src-tauri/src/ui/menus.rs::ui_list_extension_keybindings` 复用同一套 command/action 映射和 HostView contract，不建立第二套快捷键 registry。
- 当前 `BuiltinAction` contract 只包含满足 HostView contract 的 `OpenView` / `ToggleView`；目标 view 不满足 renderer / placement / view 引用校验时，后端 projection 不输出 command、menu 或 keybinding。
- `keybinding.when` 或关联 `command.when` 存在时，当前没有上下文评估能力，后端投影 fail-closed；空 key、缺失 command、无 action 或未知 renderer/placement 同样不输出。
- `src-tauri/src/extension/host_view.rs` 是扩展域与 UI 域之间的 HostView contract；renderer/placement 语义不进入 Kernel。

结论：扩展 command 只是声明式 action，不是扩展 JS handler runtime；keybinding 是后端公共投影，前端只消费 DTO 并接入既有快捷键分发。新增扩展 UI 能力应复用现有 action、HostView contract 和投影，不复制 registry 或绕过 UI IPC。

### 2.3A Extension View projection 已统一

当前事实：

- `src-tauri/src/ui/extensions.rs::ui_list_extension_views` 是 enabled Extension View 列表的统一 UI projection 入口；前端 `src/stores/extension.ts` 通过 `invoke` 读取 `UiExtensionView[]`，不再从菜单、命令或快捷键 DTO 反推 view。
- projection 只读取 `ExtensionStatus::Enabled` 的 Extension，复用 `ui_extension_view` 与 `ui_extension_view_descriptor` 完成 HostView contract、placement、renderer 和 HTML entry 校验；非法 view 直接跳过，避免任何用户可见入口消费未渲染的 view。
- `UiExtensionViewDescriptor` 统一归一化 `allow_close`（默认 `true`）、`default_visible`（默认 `false`）以及 HTML view 的 `resource_path`；投影结果按 Extension 与 view 标识稳定排序。
- `default_visible=true` 只由统一 View projection 在首次加载、安装或启用后恢复，不依赖菜单/命令声明；禁用或卸载 Extension 时，前端清理全部 placement 的 HostView instance。

结论：Extension View 的发现、校验和生命周期状态由一条 projection 链路承接。新增 Extension View 只需声明现有 manifest contract 和资源，不应在宿主业务分支中添加 Extension ID、标题或 placement 特判。

### 2.4 LSP IPC 边界已回到 UI 域

当前事实：

- `src-tauri/src/ui/lsp.rs` 存在并承担 LSP Tauri commands
- `src-tauri/src/tool/lsp/commands.rs` 已不存在

结论：原报告中“tool 层定义 Tauri command”的 LSP 特例已完成清理，不再是活动问题。

### 2.5 TimelineStatus / SessionEvent 的存储入口已收敛

当前事实：

- `SessionEvent` 模型位于 `src-tauri/src/foundation/storage/session_store/models.rs`
- `SessionStore` 的 `append_event` / `list_events` 及事务辅助方法是 SessionEvent 唯一的持久化读写入口
- `TimelineStatus` 位于 `src-tauri/src/foundation/storage/session_store/timeline_status.rs`

结论：重复的裸连接事件读写路径已删除，SessionEvent 的事实存储统一归属于 SessionStore。

### 2.6 ownership 与 policy 两条旧建议已完成

当前事实：

- `src-tauri/src/tool/agent/special/host.rs` 当前只持有 `Option<Arc<AgentControlPorts>>`，不再直接持有 `TaskManager`、Todo projection sink 或 Sidechain starter
- `src-tauri/src/tool/agent/pipeline/data.rs` 中 progress sender 为 `UnboundedSender<Value>`
- `src-tauri/src/tool/agent/pipeline/runner.rs:78-79` 使用 `policy.overlay([extension_hooks])`

结论：

- 原报告中关于 ownership 路径的 `unsafe transmute` 建议已失效
- 原报告中关于 policy constraint 无法移除的建议已失效

### 2.7 Terminal ownership 与 PTY State 边界已同步

当前事实：

- Terminal 保持在 `tool/terminal` 域；命令执行与交互式 PTY 共享终端域的安全、Shell 和历史能力，但输出路由保持分离。
- `PtyService` 负责 PTY session、读写、resize、终止和输出桥接；`TerminalManager` 只负责终端实例、策略和业务协调。
- `PtyStreamManager` 在共享 State 中只保存可发送的内部 sender；非 `Send + Sync` 的 Tauri `Channel` 只由专属转发线程持有，不进入 `State<Arc<TerminalManager>>`。
- 现有 IPC contract 保持为 `ui_terminal_create_pty`、`ui_terminal_write_pty`、`ui_terminal_resize_pty` 和 `ui_terminal_close_pty`，本次没有保留第二套 PTY 入口或兼容 wrapper。

结论：

- PTY 生命周期已收口在终端域内部，后续扩展新的终端输出消费者只需扩展桥接层，不需要把 Channel 或 PTY 资源下沉到 Kernel。
- A4/P1 当前聚焦 `AgentToolEvent` 的 application/runtime port 主链路接线；Todo/Sidechain 特殊工具迁移已完成，不因本次 Terminal 改造改变状态。

---

## 3. 当前仍需处理的架构问题

### 3.1 Agent turn 编排已从 UI 入口下沉，但仍保留 UI runtime 事实

当前事实：

- `ui/mod.rs::ui_stream_session_message` 现在只负责 Tauri State 解包、权限配置装配和一次 use-case 调用。
- 完整 turn 初始化、Agent/tool loop、流生命周期、错误推流和收尾持久化位于 `ui/runtime/session_message_stream.rs` 与相邻 runtime contract；该文件仍直接承担 Tauri `Channel`、timeline 和持久化协调。
- `ui/runtime/agent_tool_loop.rs` 继续承担可复用的 Agent/tool 执行链，UI command 不再持有完整 orchestration。

结论：

- A1 已完成，完成范围是把 UI command 入口的 turn 编排下沉到可调用的 UI runtime use-case；这不等于 turn 已完全迁移到独立 application 层。
- 后续批量运行、后台任务或无 UI 入口执行仍需继续抽取 Channel、timeline、持久化等 port 后才能复用；该边界不应下沉到 kernel，kernel 只保留通用事件、策略和执行原语。
### 3.2 审批运行时已收成单点

当前事实：

- `src-tauri/src/ui/runtime/tool_approval_flow.rs::resolve_tool_approval_for_event` 是审批协调入口。
- `agent_tool_loop.rs` 不再直接调用 `request_tool_approval`，也不再重复构造 WaitingPermission 或拒绝处理。
- 该入口统一负责审批策略、缓存恢复、用户等待、结果证据注入和 permission timeline 事实。

结论：

- A2 已完成。新增审批策略或审计语义只需扩展该 runtime 协调面，UI command 和 Agent loop 不再各自维护一套审批流程。

### 3.3 Storage facade 已部分收口

当前事实：

- `SessionStore` 与 `MemoryStore` 持有领域内部的数据库能力，业务调用通过明确的领域方法完成读写。
- `SessionStore` 负责锁和事务，底层 history/snapshot/checkpoint/worktree binding helper 只依赖 `&Connection`，可被普通连接和事务复用。
- 通用 `Storage::connection()` 已删除；Auth 使用 security 域专属 `AuthStore`，不再从 Storage 取得裸连接。
- 测试需要底层连接时只通过 `cfg(test)` 的 `test_lock_connection`，不进入生产接口。
- `Storage` 仍是全库存储入口，内部仍持有共享数据库连接，并负责创建 `SessionStore`、`MemoryStore` 等领域 facade；`SessionStore` 等 facade 的进一步领域化抽取尚未完成。

结论：

- A3 保持 `reduced`：通用裸连接逃生口、Auth 从 Storage 取连接的路径已删除，MemoryStore 已完成领域收口；但 Storage 仍是全库入口，SessionStore 等 facade 仍待继续抽取，因此不能标记为 `completed`。
- 新增持久化能力应在对应 Store 增加明确操作并由上层组合，不应复制锁管理或直接操作连接。
### 3.4 `ai/agent` 与 `tool/agent` 边界已部分收敛

当前事实：

- `tool/agent/contract.rs` 定义了工具目录所需的最小 `ToolAvailability` 能力，`catalog`、mode filter、resolver 和 runtime 不再依赖完整 `ModeConfig`。
- `tool/agent/contract.rs` 同时定义 `AgentExecutionContext`；工具 runtime 只接收 `session_id` 与 `worktree_root`，由 UI/application 边界从 Storage `Session` 投影最小执行事实。
- `TaskManager`、Todo、Sidechain 和 `AgentToolEvent` 的内部实现仍属于既有 Agent/UI runtime，但独立的 application/runtime contract 已建立在 `src-tauri/src/application/runtime/agent_control.rs`。`tool/agent/special` 已改为只依赖 Todo/Sidechain application port；UI/Storage 事实不再进入特殊工具 host。

影响：

- 新增工具目录能力已经可以通过 `ToolAvailability` 接入；特殊控制工具依赖 `application::runtime` 中的 Todo、Sidechain port，不再把 `TaskManager`、Storage Session 或 UI 输出结构带入工具合同。Sidechain 快照只包含模型事实，UI 与模型输出由特殊工具边界分别投影。

本轮完成了控制合同建立、`AgentExecutionContext` 边界收口以及 Todo/Sidechain 特殊工具适配切换，并用 application、UI 和 tool 测试验证。Agent turn 主链路仍直接维护 `AgentToolEvent`、Channel、timeline 和审批投影；`AgentToolEventPort` 保持为独立的后续 turn use-case 合同，但当前没有伪装成已接线的 UI 适配器，因此 A4/P1 仍保持 `reduced`。

---

## 4. D1-D5 死代码判断复核

以下结论基于 2026-08-14 当前实现事实，记录已完成的文件级清理结果。

| 编号 | 结论 | 当前证据 | 建议 |
|------|------|---------|------|
| D1 | 已完成 | `tool/edit/*` 已删除；实际编辑链路统一由 `tool/mcp/builtin/filesystem.rs` 承担 | 保持单一编辑能力入口，不恢复平行实现 |
| D2 | 已完成 | `src-tauri/src/tool/cron/mod.rs` 已不存在，`pub mod cron` 也已移除 | 保持删除状态 |
| D3 | 已完成 | `ai/agent/workflow.rs` 及相关 re-export 已删除 | 工作流能力后续应通过现有 Agent runtime contract 扩展，不恢复孤立执行器 |
| D4 | 已完成 | 无生产入口的 `provider/openai.rs` 与 `provider/anthropic.rs` 已删除；`provider/profile.rs` 继续承担活跃配置能力 | 新 Provider 通过 `ProviderProfile` 与协议适配器注册扩展，不恢复双轨 Provider 对象 |
| D5 | 已完成 | clipboard 旧 manager/provider/history/watcher 栈已删除；活链路保留 `tool/clipboard/policy.rs` 与 `tool/mcp/builtin/clipboard.rs` | 保持 policy 与 MCP builtin 的职责分离，不恢复旧 manager 栈 |

补充说明：原报告中“`tool/edit`、`tool/cron`、`tool/memory`、`tool/clipboard` 生产引用都不存在”这一行已不符合当前事实。至少 `tool/memory` 与 clipboard policy / MCP builtin 现在都在活链路里。

---

## 5. 更新后的优先处理建议

### P0

已完成：工具审批运行时已由 `resolve_tool_approval_for_event` 收成单点。

### P1

保留：先把 `AgentToolEvent` 主 turn 链路接到 application/runtime port，再继续扩展事件投影；Todo/Sidechain 特殊工具迁移已完成，不再作为待迁移项重复列出。新增工具目录能力统一依赖 `ToolAvailability`。


### P2

3. 已完成 D1-D5 文件级死代码清理；后续新增能力必须接入现有生产 contract，禁止保留未注册的平行实现。

## 6. 当前索引

| 编号 | 状态 | 问题 |
|------|------|------|
| A1 | completed | UI command 入口已下沉到 UI runtime use-case；独立 application 层迁移仍未完成 |
| A2 | completed | 审批运行时统一由 `resolve_tool_approval_for_event` 协调 |
| A3 | reduced | 通用连接逃生口已删除，MemoryStore/AuthStore 已收口；Storage 仍是全库入口，SessionStore 等 facade 仍待继续抽取 |
| A4 | reduced | 工具目录已依赖 `ToolAvailability`，Todo/Sidechain 特殊工具已接入 application port；`AgentToolEvent` 主 turn 链路仍待接线 |
| S1 | synced | 双重 Router 已解决 |
| S2 | synced | 前端扩展 UI 主链路已切换到 HostView |
| S3 | synced | LSP command 已回到 UI 域 |
| S4 | synced | TimelineStatus / SessionEvent 旧路径判断已失效并完成同步 |
| S5 | synced | ownership/transmute 旧建议失效 |
| S6 | synced | policy overlay 旧建议失效 |
| S7 | synced | `tool/cron` 已删除，不再列为活动 dead-code 项 |
| S8 | synced | Terminal 保持在 `tool` 域，PTY 生命周期收口到 `PtyService`，Tauri Channel 不进入共享 State |
| D1 | completed | `tool/edit/*` 已删除 |
| D3 | completed | `ai/agent/workflow.rs` 已删除 |
| D4 | completed | 未接线 Provider 实现已删除，`provider/profile.rs` 保留 |
| D5 | completed | clipboard 旧 manager 栈已删除，policy 与 MCP builtin 保留 |
