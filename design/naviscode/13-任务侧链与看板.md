# 17 - Task Sidechain 子任务编排详细设计

> 模块编号：17 | 层级：能力层
> 依赖：16-Agent, 08-Session, 13-MCP, Kernel Pipeline, Kernel EventBus, Kernel Policy
> 被依赖：ai/agent turn loop, tool/agent runtime, UI Timeline

---

## 一、模块概述

Task Sidechain 是 Navis Go 的子任务和后台执行编排层。它不是独立协作系统，也不是第二套任务事实源。

所有用户 turn、后台任务、sidechain 子任务、并行分支都归一为 `Task` 的不同 `kind`，并通过同一组事实源展示和恢复：

- `tasks`：任务状态、父子关系、依赖关系。
- `sessions`：父会话和 sidechain 子会话。
- `agent_timeline_parts`：当前 turn 的可视化执行过程。
- `session_events`：可重放的追加事件事实。
- `session_changes`：文件变更事实。

内核保持通用：Registry 发现能力，Pipeline 执行能力，EventBus 通知状态，Policy 判断权限。Task Sidechain 只是在业务层组合这些原语。

## 二、职责边界

负责：

- 创建 `Task.kind = "sidechain" | "parallel" | "background"` 的任务。
- 为子任务创建独立 sidechain session。
- 组装子任务上下文，只注入必要项目摘要、依赖结果、角色指导和相关文件。
- 限制并发、超时和取消。
- 禁止 sidechain 内再次创建 `task` / `task_output` / `task_stop` 子链工具调用。
- 聚合子任务摘要和结构化输出并回写父 session 的 AgentTimelinePart。

不负责：

- 不创建独立的子任务执行事实源。
- 不维护独立任务树 UI 状态。
- 不让子任务绕过 Tool Projection / MCP / Sandbox / Kernel Policy。
- 不把子 session 完整 transcript 塞回父 session。
- 不允许递归 sidechain，也不引入“最大深度 > 1”的旧路径。
- 不复制 Claude Code 或 Hermes 的历史路径、CLI stdout 协议、线程运行时或旧工具别名。

## 三、参考项目吸收点

| 来源 | 吸收 | Navis Go 落点 |
|------|------|---------------|
| Claude Code | 子任务 sidechain transcript、任务完成摘要、权限冒泡 | child `Session` 内保留 transcript，parent 只接收 summary / structured output |
| Claude Code | tool_use / tool_result 配对和可恢复 transcript | `agent_timeline_parts` + `session_events` |
| Hermes | 审批队列、危险操作 hard deny floor、工具生命周期事件 | Sandbox + Tool Executor + AgentTimelinePart metadata |
| Hermes | 子任务最大并发、工具集限制 | Task Sidechain scheduler；递归深度固定为 1 |

## 四、数据模型

```rust
struct Task {
    id: String,
    parent_task_id: Option<String>,
    parent_session_id: String,
    sidechain_session_id: Option<String>,
    kind: TaskKind,
    owner: Option<String>,
    active_form: Option<String>,
    status: TaskStatus,
    title: String,
    description: String,
    blocks: Vec<String>,
    blocked_by: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    metadata: Value,
}

enum TaskKind {
    UserTurn,
    Sidechain,
    Parallel,
    Background,
}

enum TaskStatus {
    Queued,
    Blocked,
    Running,
    WaitingPermission,
    Completed,
    Failed,
    Cancelled,
}
```

`owner` 是当前执行者或子 agent runtime id，用于 claim 语义和 UI 实时状态展示。任务只能被空 owner 或同一 owner 领取；不同 owner 不能抢占，终态和被阻塞任务不能被领取。

`active_form` 是运行中任务的短状态文本，例如当前工具、当前检查阶段或等待项。它是 UI 投影字段，不是模型上下文事实。

`blocks` / `blocked_by` 表达任务间依赖。被依赖阻塞的任务进入 `Blocked`，不能进入执行态；解除所有 blocker 后回到 `Queued/Pending`，等待调度器重新领取。删除或清理任务时必须同步清理其他任务中的依赖引用，避免 UI 面板展示悬空 blocker。

`TaskManager` 是 Agent / sidechain / background task 的唯一运行时投影来源。完整 UI 面板（例如 Background tasks、子 agent 实时状态、计划状态）读取 `ui_list_tasks` 或后续同源投影接口，不维护第二套前端假任务状态。

父 session 中的子任务只展示一个 `kind = "subtask"` 的 AgentTimelinePart：

```ts
input: {
  description: string
  taskTitle?: string
  agentName?: string
}
progress: {
  sidechainSessionId?: string
  activity?: string
  toolUses?: number
  totalTokens?: number
}
output: {
  sidechainSessionId: string
  summary: string
  structuredOutput?: Record<string, unknown>
  toolUses?: number
  totalTokens?: number
}
metadata: {
  taskId: string
  sidechainSessionId: string
}
```

## 五、执行流程

```text
父 Agent 决定派生子任务
  -> Agent Tool Pipeline
  -> AgentControlExecutionStage 调用 tool/agent/special.rs 执行 task control tool
  -> UI/app 宿主服务创建 child session 并启动 child turn loop
  -> TaskManager 创建 sidechain task
  -> Context Manager 组装子任务上下文
  -> ai/agent turn loop 在 child session 内继续决策
  -> tool/agent runtime 继续走 Tool Projection / MCP / Sandbox / Policy
  -> child session 写入自己的 AgentTimelinePart / SessionEvent
  -> 父 session 的 subtask AgentTimelinePart 只更新摘要、结构化输出和进度
```

`task` / `task_output` / `task_stop` 的参数解析、TaskManager 状态变更、模型可见输出裁剪都属于 `tool/agent/special.rs`，并且只能由 Agent Tool Pipeline 的 `AgentControlExecutionStage` 触发。UI 层只提供 sidechain child session 创建、后台 turn loop 启动、Timeline 渲染和 Todo metadata 投影，不拥有这些工具的执行语义，也不能直接拼装 tool result。

### 5.1 隔离契约

Sidechain session 是执行隔离上下文，不是可递归的多 Agent 协作系统。

- 子链 Tool Projection 必须移除 `task`、`task_output`、`task_stop`，包括 provider 原生 tool call 和文本 `<tool_call>` 恢复路径。
- 子链可以继续使用 `todo`、只读工具、经权限策略允许的写入/命令/网络工具，但不得启动新的 sidechain。
- 子链完成时只产出 `SidechainOutcome { summary, structuredOutput }`。
- 父链通过 `task_output` 读取的模型可见 payload 只包含 `summary` 和 `structuredOutput`；`taskId`、`sidechainSessionId`、活动状态、工具数、token、耗时等只作为 UI / Task 投影元数据，不进入父模型结果。
- child session 的 messages、AgentTimelinePart、SessionEvent 是独立事实源，用于审计、恢复和 UI 跳转；不得复制到父 session transcript 或父模型上下文。

## 六、上下文组装

子任务不继承父会话完整历史。Context Manager 只注入：

- `projectSummary`：从 Navis Go 项目配置和知识源提取的短摘要。
- `dependencyResults`：已完成依赖任务的摘要。
- `roleGuidance`：来自 Skills / RoleDefinition 的行为指导。
- `relevantFiles`：父 Agent 建议并经文件系统验证存在的路径。
- `background`：父 Agent 为该子任务写入的约束和注意事项。

超过预算时按以下顺序裁剪：

```text
projectSummary -> relevantFiles -> background -> dependencyResults -> roleGuidance
```

## 七、权限和审计

子任务默认继承父任务的权限模式，但每次高风险能力仍必须经过 Kernel Policy。父任务授予的 grant 必须带 scope 和 constraint，不能用全局 bypass 扩权。

Sidechain 默认是非交互执行上下文：没有可响应的前端 approval channel 时，运行时不得创建等待用户响应的审批请求，也不得把 `full-auto` 当作风险工具的静默审批证据。非交互 sidechain 只允许 UI approval prompt policy 明确 `allow` 且不属于风险类的工具继续进入 Kernel Policy / Sandbox 执行链；任何 `ask`、`deny`、未知 permission、风险类自动审批请求都直接拒绝并作为 tool result 回注子会话。

Hardline blocklist 是审批下限而不是普通规则。命中 hardline 的危险命令（例如根目录递归删除、sudo 删除、fork bomb、破坏性 infra 命令等）在 `suggest` / `auto-edit` / `full-auto`、session grant、project grant 和 sidechain 中都必须直接拒绝，不允许弹窗升级为 allow。

审批请求写入同一条时间线：

- child session 写入 `waiting_permission` AgentTimelinePart。
- parent session 的 subtask AgentTimelinePart 显示 `Waiting permission` 摘要。
- 审批结果进入 Audit 和 SessionEvent。

## 八、事件和流

离散状态事件通过 Kernel EventBus：

```text
task.enqueued
task.blocked
task.unblocked
task.started
task.progress
task.waiting_permission
task.completed
task.failed
task.cancelled
```

高频文本、终端输出、模型 delta 不走 EventBus，继续走 Stream Channel。

## 九、测试策略

- 创建 sidechain task 后必须有 child session。
- 子任务失败不会污染父 session transcript，只更新父 subtask step。
- 取消父 task 会取消所有未完成子 task。
- grant scope 不匹配时必须 fail-closed。
- 子任务摘要可恢复：重启后从 SQLite facts 重建 UI。
