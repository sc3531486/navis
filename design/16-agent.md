# 16 - Agent 决策引擎 详细设计

> 模块编号：16 | 层级：AI 核心层
> 依赖：01-Logger, 02-IPC, 03-Config, 08-Session, 12-Gateway, 13-MCP, 19-Skills, 18-Context-Manager
> 被依赖：17-Task Sidechain
>
> **Skills 依赖说明**：Agent 通过 Skills 模块获取角色定义（RoleDefinition）。ModeConfig.role 引用 RoleDefinition.id，
> Agent 在初始化和模式切换时调用 `Skills::get_role_definition()` 获取角色的系统提示词模板和能力声明。

---

## 一、模块概述

### 1.1 定位

Agent 是 AI 决策核心，管理对话 turn loop、状态机、任务状态、工具调用决策和工作流调度。

Navis Go 不引入额外的用户侧“Agent 链路”概念。`Code`、`Cowork` 和每个 `Custom` 模式扩展本身就是 Agent 工作模式；它们共享同一套 `ai/agent` turn loop，并通过 `ModeConfig` / `AgentContext` 切换角色、工具边界、上下文策略、权限策略和默认 UI 入口。真实工具能力来源是 Kernel Registry，模型可见投影、执行管线、保护和 hook 统一归属 `tool/agent`。

### 1.2 职责边界

```
负责：
├── 状态机管理（Idle/Thinking/ToolCalling/WaitingPermission/Streaming/Recovering/Error）
├── 任务状态管理（单轮对话、多步任务）
├── 工具调用决策（LLM 返回工具调用时决定是否继续 turn loop）
├── 流式响应处理（逐 token 推送到前端）
├── 上下文压缩触发（长会话时通知 Context Manager）
├── 工作流执行（增强模式 Skills 的步骤调度）
├── 用户确认状态处理（等待工具审批结果继续 turn loop）
├── 任务取消/超时处理
├── 自我进化（经验捕获、模式提取、反思分析、策略优化）
├── 并行任务执行（Git Worktree 隔离的多任务并发）
├── Extended Thinking 深度推理（长思考链配置与展示）
└── Agent Turn Context（单轮模式上下文、ModeConfig、worktree_root、用户显式附件/选区/引用、工具边界注入）

不负责：
├── 上下文组装 → Context Manager
├── 模型调用 → Gateway
├── 工具执行 → tool/agent（MCP/file/git/lsp/terminal/edit 均属于 tool 域执行能力）
├── 子任务编排 → Task Sidechain
└── 消息存储 → Session/Storage
```

### 1.3 Claude Code 对齐后的核心 turn loop

Navis Go 的 `ai/agent` 采用 Claude Code 风格的单一 turn loop；工具模型侧投影和工具运行链由 `tool/agent` 承接，并保留 Navis Go 的可恢复事件入库和 Tool Projection：

```text
AgentTurnRunner
  -> PromptAssembler 构建上下文
  -> ToolPlanner 生成默认窄工具面（核心工具 + tool_search + execute_tool）
  -> Gateway model stream
  -> 解析 assistant text / thinking / tool_use
  -> tool/agent::runtime 规范化 provider-safe tool call
  -> tool/agent::pipeline 进入 Kernel Pipeline
  -> tool/mcp::executor 进入 MCP Kernel Pipeline
  -> MCP ToolCallResult 回注模型并写入 AgentTimelinePart
  -> 继续 model stream
  -> turn_finalizer
```

不再允许“先跑非流式工具循环，再启动最终流式回答”的两段式主路径。文本形式 `<tool_call>` 只能作为 provider adapter 内部解析策略，不能成为 `ai/agent` turn loop 之外的第二套工具协议。

#### 默认模型工具面

Agent 默认不给模型注入完整 MCP / 本地工具列表。每轮 Gateway function schema 只包含：

- 核心工具：由 ModeConfig 和 ToolPlanner 选择的低频变动、高价值基础能力，例如 `read`、`list`、`grep`、`edit`、`bash`、`todo` 等；具体集合仍受模式、权限和项目可信状态裁剪。
- `tool_search`：模型用 `toolQuery` 检索当前策略允许发现的 MCP / 本地 / Agent control 工具。
- `execute_tool`：模型用 `toolName` 和 `arguments` 执行 `tool_search` 返回的稳定工具名。

`tool_search` 和 `execute_tool` 是 Tool Projection 的模型可见查询/间接执行操作，不是旧工具别名，也不是绕过 MCP 的桥接入口。实际工具仍通过 Kernel Registry 注册，通过 Kernel Policy 裁剪，通过 Kernel Pipeline 执行，并通过 Kernel EventBus 写出授权、开始、完成、失败等事实。`tool/agent` runtime 负责把这些事实归一为 Gateway tool result 和 AgentTimelinePart；`ai/agent` 只消费结果继续 turn loop。

模型可见字段名固定为 `toolQuery`、`toolName`、`arguments`、`callId`。Provider adapter 可以在内部做一次性协议适配，但 `tool/agent` runtime、Tool Projection、AgentTimelinePart payload、审计和扩展包契约不得继续暴露 `query`、`name`、`tool`、`args`、`input`、`id` 等历史同义字段。

#### Tool Projection 与执行管线

Tool 是模型可调用能力的核心概念，但应用层不再维护独立 `AgentTool` trait 或第二张工具执行目录。MCP ToolDefinition 是主要能力声明来源；Tool Projection 只负责把 MCP canonical tool、少量 Agent control tool 和模式配置投影成 provider-safe 名称、Gateway function schema、风险、UI hint 和 displayKind。`ai/agent` 不能直接依赖裸 MCPTool，也不能把 `task/todo` 这类 Agent control tool 散落在 UI 特判里；它只通过 `tool/agent::runtime` 解析本轮 Tool Projection 返回的稳定工具名。

执行链路固定为：

```text
provider-safe tool call
  -> Tool Projection 反查 MCP canonical name / Agent control tool
  -> ToolPipelineData
  -> Kernel Pipeline:
       PolicyCheckStage
       ToolGuardrailStage
       AgentControlExecutionStage
       McpExecutionStage
       ObserveToolResultStage
       EmitEventStage
       AuditStage
  -> MCP ToolCallResult / Agent control result
  -> Gateway tool result
  -> AgentTimelinePart
```

这样内建工具、MCP 扩展工具、Agent control tool、未来 browser/computer-control 工具都走同一套权限、审计和 UI renderer。差异只体现在具体执行 stage：Agent control tool 先由 `AgentControlExecutionStage` 调用 `tool/agent/special.rs`，命中后写入同构 `ToolCallResult` 并跳过 MCP 执行；普通 MCP tool 继续由 `McpExecutionStage` 进入 MCP Executor；扩展外部工具通过 MCP server 调用。`ObserveToolResultStage` 只写 tracing 观察日志，不是事实源；AgentTimelinePart / Storage 是 UI 与回放事实源，Kernel Audit 是审计事实源。任何执行 stage 都必须输出同构 Gateway tool result 和 AgentTimelinePart，不得新增 `AgentTool`/`ToolBatchExecutor` 这类平行执行体系。

Agent turn loop 是 async 主路径。它调用 `tool/agent::runtime::*_async`，再通过 `run_standard_tool_pipeline(...).await` 进入 Kernel Pipeline；禁止在 turn loop、UI stream command、sidechain async runner 中反向调用同步 wrapper 或 `Pipeline::run_blocking()`。同步 wrapper 只保留给非 async 边界和测试兼容。

Stop/Cancel 必须同时取消流和关闭 Turn Timeline 事实。`ui_cancel_stream` 通过 Stream 活动索引定位 `session_id / turn_id / assistant_message_id`，将仍处于 `pending / running / retrying / waiting_permission` 的 AgentTimelinePart 写成 `aborted`；底层长任务还应继续接入 Pipeline cancellation token，以便后续真正中断执行。前端不能只本地标记 aborted，也不能让 running AgentTimelinePart 靠计时器继续增长。

`todo`、`task`、`task_output`、`task_stop` 是内建 Agent control tool，语义唯一落点是 `tool/agent/special.rs`，执行入口唯一落点是 Agent Tool Pipeline 的 `AgentControlExecutionStage`。UI 只能提供宿主服务：把 Todo 投影写回 Session UI metadata、创建 sidechain child session、启动 child turn loop、渲染 AgentTimelinePart。UI 不允许解析这些工具的 payload、不允许拼装模型可见 tool result，也不允许在调用 `tool/agent` runtime 之前先执行 control tool 形成第二套路径。

#### 工具调度

当前主路径按模型返回顺序串行执行工具，避免并发工具写入、权限审批和 AgentTimelinePart 顺序互相污染。后续如果需要 readonly 工具并发，只能作为 Kernel Pipeline 上的批处理执行策略加入，并由 Tool Projection 的风险/readonly 元数据和 Policy 决策共同裁剪；不能恢复独立 `ToolBatchExecutor` 或让工具直接操作 UI。每个工具运行都必须拥有同一套 callId、duration、progress、error kind 和 AgentTimelinePart 更新路径。

#### ToolGuardrail

`ToolGuardrail` 位于 `src-tauri/src/tool/agent/guardrail.rs`，是 `tool/agent` 的执行前保护规则，不是 Kernel 新原语。它的执行落点必须是 Agent Tool Pipeline 的 `ToolGuardrailStage`，不能在 `tool/agent` runtime 中提前返回，也不能绕过后续观察、EventBus 和 Audit。

参考 Hermes 的工具循环 guardrail，Navis Go 在调用具体 MCP tool 之前检查最近 transcript：

- 同一 provider-safe tool name
- 同一归一化参数 fingerprint
- 最近窗口内连续返回错误 tool result

达到阈值时，`ToolGuardrailStage` 不调用底层工具，而是在同一 PipelineContext 中写入标准错误 `ToolCallResult`：

```json
{
  "isError": true,
  "guardrail": "repeated_tool_failure",
  "error": "Blocked repeated failing tool call..."
}
```

这个结果继续经过 `ObserveToolResultStage`、`EmitEventStage` 和 `AuditStage`，再由 runtime 回注模型，让模型换工具、换参数或向用户澄清。Guardrail 不写第二套状态源，不绕过 AgentTimelinePart，不直接操作 UI，也不改变 Kernel Pipeline / EventBus / Policy。

#### 工具结果

所有工具结果必须分层进入 Gateway tool result 和 AgentTimelinePart：模型可见内容保持紧凑，UI 展示内容放入 AgentTimelinePart `input/progress/output/metadata/detail`，需要长期查询或回滚的事实进入 SessionChange、audit_log 或对应领域表。大输出不能直接污染模型上下文。Terminal、grep、webfetch 等工具可把完整输出作为 artifact 或 detail 持久化，模型可见 tool result 只提供摘要、引用和截断提示。

#### Kernel Policy

审批 UI 不是安全边界本身。`tool/agent` runtime / guardrail 必须在工具语义层构造 `PolicyInput`，交给 Kernel Policy 和 Sandbox constraint 统一决策：

```text
ToolPipelineData + normalized args + ApprovalMode + Settings matrix + session/project grants
    -> PolicyInput
    -> Kernel Policy
    -> allow | ask | deny
```

`allow_once / allow_session / allow_project / deny_always` 是 UI 审批状态机和 Settings 规则的输入来源，最终都必须归一为 Kernel Policy 可评估的约束条件。一次性批准只作为 `ToolCallRequest._meta.navisPermissionGranted = true` 传递给当前运行时请求，不写入工具 `arguments`，不进入模型 schema，也不能放宽 Worktree 路径解析边界。

`bash` 必须优先补语义分析：解析命令、重定向、路径、子命令、管道和危险组合；无法解析时默认 ask 或 deny，不能只按字符串 risk 允许。

---

## 一、工作模式（Work Mode）

### 1.1 模式说明

Agent 的执行受当前工作模式影响，不同模式下行为策略不同。工作模式决定了可用的工具集、技能集、默认角色以及进化经验的存储位置。

```
Code 模式：代码开发
├── 默认角色：developer
├── 可用工具：read/write/edit/bash/git/lsp/mcp
├── 技能集：commit/refactor/test-gen/bug-fix
├── 进化存储：evolution/code/（全局+项目）
└── 模型偏好：支持 extended thinking

Cowork 模式：文档协作
├── 默认角色：technical-writer
├── 可用工具：file/lsp.diagnostic/clipboard
├── 技能集：review/explain/doc-gen
├── 进化存储：evolution/cowork/（全局+项目）
└── 模型偏好：更注重语言表达质量

Custom 模式：自定义扩展（扩展接入新分类）
├── 默认角色：由扩展定义
├── 可用工具：由扩展声明
├── 技能集：由扩展注册
├── 进化存储：evolution/custom/{mode_id}/
└── 扩展方式：扩展在 extension.json 的 contributes.work_modes 中声明 Custom 模式扩展
```

### 1.1.1 Code / Cowork / Custom 的真实差异

工作模式不是简单的 UI 标签。一个模式至少由以下差异组成：

| 差异项 | Code 模式 | Cowork 模式 | Custom 模式扩展是否可定义 |
|--------|-----------|-------------|----------------------------|
| 默认角色 | `developer`，偏代码实现、重构、测试 | `technical-writer`，偏文档、解释、协作表达 | 可以。通过 `work_modes.role` 引用内建或扩展注册的 RoleDefinition |
| 工具白名单 | `read/write/edit/bash/git/lsp/mcp`，允许代码修改和命令执行 | `read/lsp.diagnostic/clipboard`，默认弱化写代码和终端执行 | 可以。通过 `work_modes.available_tools` 定义，但最终仍受 Sandbox、Project Trust、扩展权限约束 |
| 默认技能集 | `commit/refactor/test-gen/bug-fix` | `review/explain/doc-gen` | 可以。通过 `work_modes.skills` 引用内建、项目、用户或扩展注册的 Skills |
| 命令入口 | 代码相关 Commands 优先 | 文档、说明、审阅相关 Commands 优先 | 可以。通过 `work_modes.commands` 和 `contributes.commands` 组合 |
| 模型偏好 | 低 temperature，可启用 extended thinking | 更高语言质量权重，通常关闭 extended thinking | 可以。通过 `work_modes.model_preferences` 提供建议，最终写入 Session 模型偏好 |
| 上下文策略 | 优先代码、Diff、测试、Git、LSP 诊断 | 优先文档、引用材料、说明目标、格式规范 | 可以。通过 `work_modes.context_policy` 声明 |
| UI 入口 | Code 模式菜单、代码编辑结果、Diff/终端/诊断面板 | Cowork 模式菜单、文档预览、引用说明 | 可以。通过 `work_modes.entry_view/default_views` 引用满足 HostView contract 的 view，并使用合法 placement（例如 `rightWorkspace`）打开对应 surface |
| 进化存储 | `evolution/code/` | `evolution/cowork/` | 自动分配 `evolution/custom/{mode_id}/` |
| 行为约束 | 改代码前检查、测试、lint、危险操作确认 | 输出结构清晰、引用路径、控制语气和格式 | 可以。通过 `work_modes.behavior_rules` 注入模式规则 |

因此，Custom 下的每一个模式扩展都应被视为“扩展贡献的完整工作模式”，而不是普通功能菜单项。它和 Code / Cowork 的区别只在于来源不同：Code / Cowork 是宿主内建模式，Custom 模式由扩展通过 `contributes.work_modes` 注册。

### 1.2 模式切换行为

- 切换工作模式时，自动重新加载对应模式的技能集和工具配置
- 切换不会中断当前正在执行的任务（任务在原模式下继续完成）
- 新任务在切换后的模式下执行
- 用户可通过 `agent.setWorkMode()` 显式切换
- Agent 可在回复中建议用户切换模式（如"当前任务更适合 Cowork 模式，是否切换？"），但不自动执行切换
- 当用户在左侧 `Custom` 页签下点击某个模式扩展时，当前会话的 `work_mode` 切换为 `Custom(mode_id)`，并按该扩展声明的 ModeConfig 重新加载角色、工具、技能、上下文策略和 UI 默认入口。

### 1.3 上下文感知

- Agent 在处理用户请求时，自动分析任务类型
- 若任务类型与当前模式不匹配，可主动提示用户切换模式
- 模式信息作为上下文的一部分传递给 Gateway 的模型调用
- 主对话发送不能绕过 Agent。即使当前轮尚未进入完整工具循环，也必须先通过 Agent Turn Context 注入当前 `ModeConfig`、`Session.worktree_root` 和用户显式提供的附件 / 编辑器选区 / 引用片段，避免模型表现成看不到项目的通用聊天机器人。
- Agent Turn Context 不自动注入 worktree tree，也不预扫项目文档；文件内容必须来自附件、编辑器选区或后续 `read`、`edit`、`bash` 等真实工具调用。当上下文不足时，Agent 应先用自然语言说明下一步要读取或执行什么，再让真实工具事件进入 Turn Timeline。这个说明必须来自模型输出的 assistant text content，不允许前端或后端用固定模板伪造“准备工作”；如果模型第一块就是 tool call，UI 只展示轻量 Thinking 和真实工具行。
- Agent Turn Context 必须读取持久 memory snapshot，但 memory 不是进程缓存，也不能在流式 turn 中热替换系统提示。`memory.store` 通过 MCP 写入 SQLite `memories` 表后立即持久化，并按 `scope_type + scope_id + category + key` 覆盖同一记忆；PromptAssembler 在每轮上下文组装这个安全刷新点读取 bounded snapshot，并以 `scope = persistent / source = sqlite / trust = untrusted` 注入系统上下文末尾。`memory.recall` 是普通工具结果，显式传 scope 时只查指定层，默认在当前 session 下按 `global -> mode -> project -> session` 查完整 scope stack，按 JSON untrusted data 回注模型，不直接拼接成系统 prompt。
- Memory snapshot 的作用域按 `global -> mode -> project -> session` 合并。`global` 存长期用户偏好；`mode` 存 `code / cowork / custom mode` 行为差异；`project` 存项目事实；`session` 存当前会话临时上下文。同一 `category + key` 以后层覆盖前层，避免 global 偏好污染项目/会话特例，也避免模式记忆看不到全局偏好。
- 当前主对话发送链路借鉴 Claude Code 的 `assistant text / thinking / tool use / tool progress / tool result` 内容块分离方式。后端先用 ToolPlanner 规则引擎根据 `ModeConfig.available_tools`、用户输入意图、有效风险等级和当前 Project / Worktree 状态生成默认窄工具面：核心工具 + `tool_search` + `execute_tool`。ToolPlanner 不调用 LLM，避免额外延迟和不稳定性。若模型需要非常驻 MCP / 本地工具，必须先调用 `tool_search({ toolQuery, callId })` 获取可用 `toolName`，再调用 `execute_tool({ toolName, arguments, callId })` 执行；`tool/agent` runtime 注入当前 Session 的 `worktree_root` 并通过 Kernel Registry / Policy / Pipeline / EventBus 编排真实 MCP 或本地工具，再把工具结果作为 `tool` 消息回注给 Gateway。工具循环结束后，最终回答仍使用 Stream 模块推送到前端。
- `AgentTimelinePart` 不是 Claude Code 的源码命名。Claude Code 源码使用 provider content block、`tool_use` / `tool_result`、`toolUseID` 和 ACP `tool_call` notification 表达执行阶段；opencode 使用 `message + part`、`PartID`、`message.part.updated` 表达可恢复事实源。Navis Go 的公共概念叫 **Turn Timeline / AgentTimelinePart**：它吸收 Claude Code 的阶段语义和 opencode 的入库可恢复性，但作为 Navis Go 自己的产品级协议命名。数据库表、Rust storage 类型、Stream payload、前端 Store 和扩展 API 必须统一使用 `agent_timeline_parts` / `AgentTimelinePart` / `AgentTimelinePart(s)` 语义，不保留旧协议命名。
- 流式阶段如果 provider 以文本形式输出 `<tool_call>`，后端必须先缓冲，不得发布为 assistant 正文；确认是工具请求后写入/更新对应 AgentTimelinePart、执行工具并继续工具循环，直到得到自然语言回复或触发工具恢复熔断。
- 对齐 Claude Code 的错误恢复链路：工具失败不是直接终止本轮，而是写入 `ToolResult.status = "error"`，把错误内容作为模型可见的 tool result 回注，再让模型决定说明原因、读取文件、编辑或重新运行命令。若 Gateway 在同一条 assistant 响应中同时返回解释文本和 tool calls，Navis Go 必须保留这段文本，作为工具调用前的 assistant text AgentTimelinePart 按顺序展示，不能把 assistant tool-call message 的 content 置空后吞掉。
- 工具调用前的 assistant text 只属于 Turn Timeline，不参与最终 assistant message content 重复拼接；工具前说明按 `AgentTimelinePart.sequence` 出现在真实工具前，普通最终 assistant text 则属于 final phase：当前 turn 若存在工具/权限/错误等 action AgentTimelinePart，普通最终 assistant text 必须等所有 action step 非运行态且 `turn_finalizer` 到达后，统一释放到所有 action step 后、`turn_finalizer` 前。这样保证“说明原因 → 读取/编辑/运行工具 → 最终回答 → turn summary”的顺序一致，不能让命令还在执行时下方先出现“成功/完成”的最终回复。顺序分配采用单轮事件创建顺序：模型文本 prelude 创建时占用下一个 sequence，新 tool call 第一次出现时占用下一个 sequence，同一 `callId` 的 started/progress/completed/error 后续只更新原 AgentTimelinePart 并保留首次 sequence；不得用数字间隔、前端排序猜测或 `callId` 兜底合并修补顺序。前端不得把 finalizer 抽离到正文前面，也不得在外层重复渲染已经进入 Turn Timeline 的 assistant text。空的 running text AgentTimelinePart 只是后端已创建文本通道的事实，不渲染为 `Waiting for response` 或其他正文，也不能压掉真实 `turn_prelude` Thinking 状态；`turn_finalizer` 即使已到达，也只有在所有 action AgentTimelinePart 离开 `pending / running / retrying / waiting_permission` 后才对 UI 生效，token、耗时和 `Finished response` 只能随这个有效 finalizer 一起出现。
- `turn_prelude` 是运行态 reasoning AgentTimelinePart，不是 assistant 自然语言。若本轮已有真实 assistant text（包括工具前说明或最终文本）可显示，前端不再显示 prelude；若 provider 第一块就是 tool call，前端可以同时显示 active `Thinking` 和真实工具行。后端写入 `turn_finalizer` 前必须先把同轮 `turn_prelude` 更新为 `completed`，不能靠前端隐藏或历史刷新时 stale-abort 来收口。
- Code 模式只开放已经接入真实执行、审批和 Turn Timeline 的工具。模型默认可见 function tools 不再等同于完整工具目录，而是核心工具 + `tool_search` + `execute_tool`。核心工具使用 Claude Code / opencode 风格名称，例如 `list`、`read`、`glob`、`grep`、`search`、`inspect`、`write`、`edit`、`bash`、`todo`；Tool Projection 在启用阶段固定映射到 MCP canonical name：`fs.list_files`、`fs.read_file`、`fs.glob`、`fs.grep`、`fs.search_files`、`fs.file_info`、`fs.write_file`、`fs.replace_in_file`、`terminal.run_command`、`navis.todo`。`git`、`lsp`、`task`、`task_output`、`task_stop`、`webfetch`、`websearch`、真实扩展包 `browser` 和其他 MCP / 本地工具默认通过 `tool_search` 发现、通过 `execute_tool` 执行；模式配置可以把其中少数高频工具提升为核心工具，但仍必须保留同一 Tool Projection 映射和权限链路。`git` 是单一真实工具，通过 `operation=status|diff|commit` 区分行为，不再向模型暴露 `git.status/git.diff/git.commit`。`write` 可创建新文件或完整重写已有 UTF-8 文件，但已有文件必须先有同路径 `read` 记录，并确认当前内容仍等于读取内容后再写入和形成可见 diff；普通局部修改必须优先使用 `edit` 做 exact replacement，避免把小改动升级成全量覆盖。`bash` 对齐 Claude Code 的模型侧命令语义，但 Navis Go 运行时会根据系统选择真实 shell：Windows 按 `pwsh -> powershell -> Git Bash -> cmd`，非 Windows 按 `$SHELL -> bash -> sh`。`webfetch/websearch` 由内建 HTTP MCP 工具真实请求网络，网络失败必须作为 tool error 回注模型；`browser` 控制只通过真实 MCP 扩展包工具发现后进入 Tool Projection，不内建假 browser 工具。写文件、代码编辑、`git`、`bash` 和网络工具都必须进入 permission AgentTimelinePart、审批缓存和审计链路，不能以骨架、假执行或绕过审批的形式进入主对话。
- Task Sidechain loop 使用收紧后的 Tool Projection：子链 session 不暴露也不执行 `task` / `task_output` / `task_stop`，文本形式 `<tool_call>` 恢复路径同样按该投影 fail-closed。子链完成结果归一为 `SidechainOutcome { summary, structuredOutput }`；父链通过 `task_output` 只能把该摘要和结构化输出回注模型，不得把 child transcript、child AgentTimelinePart 列表或原始工具日志拼回父消息。
- Agent loop 必须携带当前 Session 的 `ApprovalMode` 和 Settings 中的工具级权限矩阵。正常 tool loop、文本形式 `<tool_call>` recovery、Task Sidechain loop 都使用同一份 `(policy, permission_rules)` 判断 `allow / ask / deny`，并把 `allow_once / allow_session / allow_project / deny_always` 写入运行时审批缓存。审批缓存键必须至少包含 `session/worktree + permission + pattern`，不能只按路径或命令字符串缓存，否则不同工具类型会互相污染。权限被拒绝时写入 permission/tool error AgentTimelinePart，并把错误作为 tool result 回注模型；不得直接吞掉工具调用。
- 扩展 `PreToolUse` / `Deny` hook 不由 UI 或 Agent loop 手动执行。`tool/agent` runtime 在进入 Agent Tool Pipeline 时读取已启用 hook 声明快照，注册为 `agent.extension_hooks` Kernel Policy constraint；`PolicyCheckStage` 统一产出 deny，后续以普通工具错误进入 AgentTimelinePart 和 Gateway tool result。
- MCP Executor 必须作为最后一道 fail-closed 防线。若 Sandbox 返回 `require_confirm = true`，只有 Agent UI 权限状态机在本轮 tool call 上注入一次性内部 grant 后才允许执行；没有 grant 的直接 MCP 调用、扩展绕行调用或测试调用都必须返回错误并写入失败结果。这个 grant 不是模型可见字段：`tool/agent` runtime 在生成 UI input 和读写校验前先从模型参数中移除，再写入当前 `ToolCallRequest._meta.navisPermissionGranted`。MCP Executor 只读取请求级内部 `_meta`，不得从工具 `arguments` 读取授权或审批模式，防止模型伪造、污染展示或改变 Worktree 路径边界。
- Agent 工具执行过程必须进入前端 **Navis Go Turn Timeline**，展示模式对齐 Claude Code 的 `ToolUse / Progress / Result` 三段语义，并吸收 opencode/Hermes 的紧凑折叠与真实事件驱动原则。一轮回复以用户消息 `turnId` 为锚点，后端先创建 assistant shell，再写入轻量 `Thinking` 状态；该状态只在本轮尚无真实 assistant text 可显示且 prelude 仍 active 时显示，不能做成“准备本轮任务”这类模板工作项。随后工具节点、审批等待、错误节点和最终 assistant 文本组成同一条 Navis Go 时间线；每个工具 AgentTimelinePart 必须携带 `input/output/metadata/progress/time`，前端通过 `ToolRendererCatalog` 选择内建或扩展包 renderer。宿主真实内建 displayKind 集合为 `read/list/glob/grep/search/inspect/edit/write-as-edit/bash/git/lsp/todo/task/task_output/task_stop/webfetch/websearch/permission/error`；`skill`、`mcp_resource`、`browser` 只有在真实 Skill/MCP 扩展包工具注册后才进入 renderer 体系，不能作为假内建工具占位。前端可以把同族 displayKind 映射到共享 renderer，但必须有 displayKind 级注册，不能只落到 `other` fallback。扩展包新增工具只接入 MCP ToolDefinition、Tool Projection 的 provider-safe 名称映射和可选 renderer，不得改写对话主流程或直接操作 chat DOM。文件创建、整文件覆盖和局部替换都归入同一类文件变更语义，统一显示为 `Edit/Edited`，不要再单独派生 `Wrote` 分支。Timeline 标题优先来自 MCP ToolDefinition 的 `ui_hint.title`，摘要优先来自工具参数和结果；错误详情单独进入 `detail`，不挤占列表可见标签；没有 `ui_hint` 时才使用 `tool/agent` runtime 的宿主默认 fallback。
- `AgentTimelinePart` 只负责“展示执行过程”，不能承担文件回滚事实源。`edit/write` 成功写盘后，`tool/agent` 与 `project/session` change recorder 必须记录 `SessionChange`，保存 `turnId / messageId / partId / callId / operation / beforeContent / afterContent / diff / insertions / deletions`；Review、Diff、Revert 都读取 `session_changes`。`SessionCheckpoint` 仍只负责 Agent / 会话恢复，不能用于文件变更回滚。这样吸收 opencode 的 patch/revert 可恢复性和 Hermes 的写入前后记录，但不引入 `worktree_checkpoint` 这类一次性概念。
- bash / terminal 行对齐 Claude Code 的 shell progress：行内摘要必须展示真实 `command`，不能用固定文案或 description 替代；命令运行中由 `terminal.run_command` 通过 `execute_with_progress` 生成真实 stdout/stderr 快照，写入同一 `callId` 的 `AgentTimelinePart.progress`，字段包含 `output`、`fullOutput`、`elapsedTimeSeconds`、`totalLines`、`totalBytes`、`timeoutMs`。前端只能消费后端 progress，不得用本地 timer、spinner-only placeholders 或模板文案伪造工具状态。命令输出以 detail/block 形式保存 `$ command`、shell、exit、stdout、stderr，错误状态只改变状态和颜色，不吞掉可展开输出。Terminal detail 在聊天区只能作为可展开的内部滚动块展示，不能把整段输出撑满对话流；展开块右下角显示由 `AgentTimelinePart.status` 映射出的 Running / Success / Failed 状态。运行中每个工具行展示实时耗时，完成 / 失败后展示后端入库的 `durationMs`。聊天主轨道必须固定宽度并与用户消息气泡右边界对齐，工具行固定为 icon / title / expand chevron / result meta / duration / status 六列，duration/status/result meta 为空也保留列宽，不同工具必须使用可区分的内建、同族复用或扩展 glyph。展开 detail、内部滚动条、diff 行号和长命令都只能在详情块内部滚动，不能让整条回复横向变宽或让 chevron 漂移。
- Agent 对 UI 的公开流式协议使用 `AgentTimelinePart` / `AgentTimelinePartDelta`，历史 payload 暴露 `AgentTimelineParts`。旧协议命名不作为前端 Store、扩展 API、数据库表名或对外文档命名；后端问题必须改后端，不新增旧命名分支。

### 1.4 模式配置文件（navis_{mode}.md）

每个工作模式拥有独立的 Markdown 配置文件，存放在项目的 `.navis/modes/` 目录下。用户可通过编辑这些文件自定义各模式的行为，未配置的字段回退到内置默认值。

**目录结构：**

```
<project>/
├── navis.md                   # 项目通用配置（不变）
└── .navis/
    ├── modes/
    │   ├── navis_code.md      # Code 模式配置
    │   ├── navis_cowork.md    # Cowork 模式配置
    │   └── navis_{mode_id}.md # 自定义模式配置（由扩展或用户创建）
    ├── evolution/
    │   ├── code/
    │   ├── cowork/
    │   └── custom/{mode_id}/
    └── skills/
```

**navis_code.md 示例：**

```markdown
# Code 模式配置

## 角色
你是一个资深全栈开发工程师。
编写代码时遵循项目规范，注重可维护性和测试覆盖。

## 可用工具
- read, write, edit, bash, git, lsp.*, mcp.*

## 默认技能集
- commit, refactor, test-gen, bug-fix

## 模型偏好
- temperature: 0.2
- extended_thinking: true

## 行为约束
- 修改代码前先运行相关测试
- commit 前自动 lint
- 重大变更需用户确认
```

**navis_cowork.md 示例：**

```markdown
# Cowork 模式配置

## 角色
你是一个技术文档专家。
编写文档时注重清晰度、结构化和可读性。

## 可用工具
- read, write, lsp.diagnostic, clipboard

## 默认技能集
- review, explain, doc-gen

## 模型偏好
- temperature: 0.5
- extended_thinking: false
- language_quality_emphasis: 0.9

## 行为约束
- 输出格式统一使用 Markdown
- 引用代码时附带文件路径和行号
```

**配置字段规范：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `## 角色` | 文本块 | 否 | 角色设定提示词，注入 system prompt |
| `## 可用工具` | 列表 | 否 | 该模式下 Agent 可调用的工具列表，支持通配符 `*` |
| `## 默认技能集` | 列表 | 否 | 该模式下自动加载的 Skills |
| `## 默认命令` | 列表 | 否 | 该模式下优先展示或启用的 Commands |
| `## 上下文策略` | Key-Value | 否 | 上下文来源、优先级、裁剪和注入位置 |
| `## 模型偏好` | Key-Value | 否 | temperature、max_tokens、extended_thinking 等 |
| `## 行为约束` | 列表 | 否 | 注入 system prompt 的行为规则 |
| `## 默认视图` | 列表 | 否 | 进入模式时建议打开的右侧面板视图或入口视图 |

会话运行时还必须读取 Session UI 元数据中的 `reasoningEffort`（`low / medium / high / extra-high / max`）。它与会话模型选择同级，属于当前 Session 的模型调用意图，不是全局设置；工具循环、最终流式回复和审批恢复请求必须使用同一个 Effort。Gateway 适配层只在 `ModelConfig.supports_reasoning_effort = true`，或模型显式要求 `reasoning_effort` 字段时，把它映射到 Provider 请求；无法支持时保留 UI 选择，不向模型注入未知字段。

**加载与合并规则（md 优先，缺省回退默认值）：**

```
Agent 初始化 / 模式切换
     │
     ▼
查找 .navis/modes/navis_{mode}.md
     │
     ├── 文件存在 → 解析 Markdown 各字段
     │   ├── 解析成功 → 用解析值构建 ModeConfig
     │   │   └── 某字段缺失 → 该字段回退到内置默认值
     │   └── 解析失败 → 记录错误日志，整体回退到内置默认值
     │
     └── 文件不存在 → 直接使用内置默认值
     │
     ▼
构建最终 ModeConfig → 注入 AgentContext
```

**合并优先级（高 → 低）：**

```
1. navis_{mode}.md 中显式配置的字段   ← 最高优先级
2. 系统内置默认值（ModeConfig 硬编码）   ← 回退默认
```

**与 navis.md 的关系：**

- `navis.md` 是项目通用配置（项目信息、代码规范、知识库路径等）
- `navis_{mode}.md` 是模式专属配置（角色、工具、技能、模型偏好）
- 两者独立加载，互不干扰
- `navis.md` 中的 `默认模式：code` 决定启动时加载哪个 mode.md

**与 Skills 模块角色定义的关系：**

- Skills 模块的 `RoleDefinition` 是角色的**唯一真相源**（包含角色名称、系统提示词模板、能力声明、行为指导等）
- `ModeConfig.role` 字段引用的是 `RoleDefinition.id`（如 `"developer"`、`"technical-writer"`）
- `navis_{mode}.md` 中的 `## 角色` 字段**仅引用** RoleDefinition.id，不直接定义角色内容
- 如需自定义角色，通过 Skills 模块的 `roles.update()` 接口修改 RoleDefinition
- Agent 获取角色定义的流程：
  1. 从 `ModeConfig.role` 获取角色 ID
  2. 调用 `Skills::get_role_definition(role_id)` 获取 `RoleDefinition`
  3. 将 `RoleDefinition.system_prompt` + `RoleDefinition.guidance` 注入 Agent 的 system prompt

---

## 二、架构设计

```
ai/agent/
├── mod.rs              # 模块入口
├── state_machine.rs    # 状态机
├── task_manager.rs     # 任务管理器
├── workflow.rs         # 工作流执行器
├── goal_runner.rs      # GoalRunnerCommand / StatePatch / RDT 续跑决策和 TaskManager 状态写入
├── sidechain.rs        # Task Sidechain 父任务通知、结构化 metadata 和可见任务描述
├── turn_context.rs     # 单轮 Agent 上下文与工具边界组装
├── turn_output.rs      # assistant 可见输出清洗；过滤 provider/tool 内部转录，避免污染 UI 文本
├── context_compress.rs # 上下文压缩触发
├── confirm_handler.rs  # 用户确认处理
├── task_scheduler.rs   # Task 并行执行分支调度适配器
├── thinking.rs         # Extended Thinking 深度推理
└── self_evolution/     # 自我进化系统（经验日志）
    ├── mod.rs                  # 子模块入口
    └── experience_logger.rs    # 执行经验日志记录

tool/agent/
├── catalog.rs          # Tool Projection 入口：provider-safe 名称映射、本轮 schema 投影和 Gateway ToolDefinition 生成
├── catalog/
│   ├── constants.rs        # 内建 Agent/MCP 工具稳定名称
│   ├── schemas.rs          # 模型可见参数 schema
│   ├── mode_filter.rs      # ModeConfig 工具可见性裁剪
│   ├── naming.rs           # provider-safe 名称和 display kind 推导
│   └── specs.rs            # 内建 fallback specs 与完整投影集合
├── pipeline/           # 工具执行管线（已从 pipeline.rs 拆分为目录）
│   ├── mod.rs              # re-export + build_standard_tool_pipeline + run 函数
│   ├── data.rs             # ToolPipelineData 及其 Debug/Clone/Default
│   ├── policy_check.rs     # PolicyCheckStage + AgentDefaultAllowConstraint
│   ├── guardrail.rs        # ToolGuardrailStage
│   ├── agent_control.rs    # AgentControlExecutionStage
│   ├── mcp_execution.rs    # McpExecutionStage
│   ├── observe.rs          # ObserveToolResultStage
│   ├── emit_events.rs      # EmitStartEventStage + EmitEventStage
│   ├── audit.rs            # AuditStage
│   ├── skill.rs            # SkillMatchStage + SkillStepStage
│   └── runner.rs           # 标准 Agent tool pipeline 构建和 async/blocking runner
├── runtime.rs          # Agent 工具运行时入口：执行编排、progress 回调、结果回注
├── runtime/
│   ├── events.rs           # AgentToolEvent / AgentToolExecution / progress callback 契约
│   ├── messages.rs         # assistant tool message 构造
│   ├── resolver.rs         # provider-safe 名称反查 MCP canonical / execute_tool 目标解析
│   ├── session_context.rs  # Session metadata → ModeConfig、worktree_root 注入、一次性授权提取
│   └── tool_search.rs      # tool_search 检索与模型可见结果
├── result.rs           # 任务/经验中的轻量工具调用快照
├── hooks.rs            # host-owned 扩展 hook 执行策略
├── special.rs          # Agent control tool 分发入口
├── special/
│   ├── host.rs             # UI host 提供的 todo 持久化和 sidechain 启动回调
│   ├── response.rs         # 同构 ToolCallResult / AgentToolExecution 构造
│   ├── todo.rs             # todo 输入解析和投影输出
│   └── sidechain.rs        # task/task_output/task_stop 输出契约、结果解析、等待逻辑
└── tool_call_utils.rs  # ToolCall 参数读取和摘要辅助
```

Goal continuation 的唯一决策点是 `ai/agent/goal_runner.rs`。UI command 只提交 start / pause / resume / stop 意图并读取 `UiComposerRunState` 投影；runner 以 RDT（Request / Decision / Task）方式读取 Session goal 状态和 `TaskManager` autonomous task 状态，只有 `Continue` 决策才生成下一条 composer task。前端不得在 stream complete 后自行循环发送，也不得拼接隐藏 next-step prompt。

Goal start / pause / resume / stop 不作为四个散落的跨模块函数暴露。跨域入口统一是 `GoalRunnerCommand`，`goal_runner.rs` 负责修改 `TaskManager` 中的 autonomous task，并返回 `GoalRunnerStatePatch` 描述可恢复 goal metadata 应如何变化。UI command 只把该 patch 写入 Session composer projection，不直接决定 goal task 生命周期。

`turn_output.rs` 只负责把 provider 输出中不应进入用户可见正文的内部转录剥离出来，例如文本形式 tool call、内部 details/task output transcript 和重复工具 prelude。它不是旧协议兼容层；长期目标仍是让 provider adapter / turn loop 在源头把 tool request 归一为 AgentTimelinePart。

Agent Tool Pipeline 的 `SkillMatchStage` 消费应用启动时托管的共享 `Arc<Mutex<Skills>>` 状态，生成 Skill activation plan 或步骤提示。Skills 不执行工具、不写第二套过程事实；真实工具仍通过 Tool Projection、Kernel Policy、Agent Tool Pipeline 和 MCP / builtin executor 执行。

---

## 三、数据模型

```rust
// ============ 工作模式数据模型 ============

// 工作模式枚举
enum WorkMode {
    Code,              // 代码开发模式
    Cowork,            // 文档协作模式
    Custom(String),    // 自定义模式（mode_id）
}

// 模式配置信息
// role 字段引用 Skills 模块的 RoleDefinition.id（如 "developer"、"technical-writer"）
// Agent 通过 Skills 模块的 Skills::get_role_definition(role_id) 获取完整的 RoleDefinition
struct ModeConfig {
    mode: WorkMode,
    role: String,                          // 默认角色（引用 Skills 模块 RoleDefinition.id）
    available_tools: Vec<String>,          // 可用工具列表
    skill_refs: Vec<String>,               // 技能引用，可为 skill id 或技能分类
    command_ids: Vec<String>,              // 模式优先命令入口
    context_policy: Option<String>,        // 上下文策略 ID
    behavior_rules: Vec<String>,           // 模式行为约束
    entry_view: Option<String>,            // 进入模式时默认打开的视图
    default_views: Vec<String>,            // 进入模式时建议打开的右侧面板视图
    evolution_path: String,                // 进化存储路径
    model_preferences: ModelPreferences,   // 模型偏好
}

struct ModelPreferences {
    supports_extended_thinking: bool,      // 是否支持深度推理
    language_quality_emphasis: f64,        // 语言表达质量权重 0.0~1.0
    // ... 其他偏好
}

// Agent 上下文
struct AgentContext {
    session_id: String,
    work_mode: WorkMode,           // 当前工作模式
    current_role: Option<String>,  // 当前角色（可覆盖默认）
    mode_config: ModeConfig,       // 当前模式配置
    // ... 其他字段
}

// Agent 状态机
enum AgentState {
    Idle,           // 空闲，等待用户输入
    Thinking,       // 思考中（调用 LLM）
    ToolCalling,    // 工具调用中
    WaitingPermission, // 等待用户确认权限 / 工具调用
    Streaming,      // 流式输出中
    Recovering,     // 恢复中
    Error,          // 错误状态
}

// Task 是唯一任务概念。
// TaskRecord + TaskKind 是唯一任务事实源。
// Sidechain / Parallel / Background / Autonomous 只能是 Task.kind 或执行适配器投影，
// 不能扩展成多套事实源。
struct TaskRecord {
    id: String,
    session_id: String,
    kind: TaskKind,                    // turn | sidechain | parallel | background | autonomous
    turn_id: String,                  // 本轮用户消息 ID；Turn Timeline 以它为锚点
    status: TaskStatus,
    messages: Vec<ProviderChatMessage>,
    tool_calls: Vec<tool::agent::ToolCallRecord>, // tool/agent 工具调用投影
    sidechain_session_id: Option<String>, // kind=sidechain 时存在
    parent_task_id: Option<String>,    // 子任务 / 并行任务归属
    current_activity: Option<String>,
    token_count: i64,
    result: Option<String>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

enum TaskKind {
    Turn,        // 用户发起的一轮 Agent turn
    Sidechain,   // task 工具启动的 Task Sidechain 子任务
    Parallel,    // 同一目标下的并行执行分支
    Background,  // UI 后台展示投影，不代表独立运行时
    Autonomous,  // cron / webhook / remote control 触发的自治入口
}

enum TaskStatus {
    Pending,
    Running,
    WaitingConfirm,
    Completed,
    Failed { error: String },
    Cancelled,
}

// ToolCallRecord / ToolCallResult / ToolCallStatus 由 tool/agent 定义。
// TaskRecord 只持有 tool::agent::ToolCallRecord 作为任务面板和恢复投影。

// ============ Navis Go Turn Timeline ============

// 后端新增 agent_timeline_parts 表，作为 UI Turn Timeline 的持久化来源。
// 每个 AgentTimelinePart 都归属于一条用户消息 turn_id；同一轮中的工具节点和最终 assistant 文本按 sequence 排序。
struct AgentTimelinePartRecord {
    id: String,                    // partId
    session_id: String,
    turn_id: String,               // 用户消息 ID；一轮回复的稳定锚点
    message_id: String,            // assistant shell 消息 ID；创建 AgentTimelinePart 前必须已存在
    kind: AgentTimelinePartKind,
    status: AgentTimelinePartStatus,
    sequence: i64,
    summary: String,
    detail: Option<Value>,
    payload: Value,                // 工具参数、结果摘要、token 增量聚合等结构化数据
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

enum AgentTimelinePartKind {
    Reasoning,
    Tool,
    Text,
    Permission,
    Sidechain,
    Error,
    Summary,
}

enum AgentTimelinePartStatus {
    Pending,
    Running,
    WaitingPermission,
    Completed,
    Error,
    Denied,
    Retrying,
    Aborted,
    Interrupted,
    Reused,
    Compacted,
}

struct AgentTimelinePartPayload {
    messageId: String,
    turnId: String,
    partId: String,
    kind: AgentTimelinePartKind,
    status: AgentTimelinePartStatus,
    sequence: i64,
    summary: String,
    detail: Option<Value>,
    toolName: Option<String>,       // MCP canonical name，如 fs.read_file
    gatewayTool: Option<String>,     // 模型可见工具名，如 read
    createdAt: DateTime<Utc>,
}

struct ToolSearchCallPayload {
    callId: String,
    toolQuery: String,
    limit: Option<usize>,
    scope: Option<String>,
}

struct ExecuteToolCallPayload {
    callId: String,
    toolName: String,
    arguments: Value,
}

// ============ 自我进化系统数据模型 ============

// 执行经验记录
struct ExecutionExperience {
    id: String,
    task_id: String,
    session_id: String,
    outcome: ExperienceOutcome,          // 成功 / 失败
    task_description: String,            // 任务描述
    tool_chain: Vec<tool::agent::ToolCallRecord>, // 使用的工具链投影
    execution_time_ms: u64,
    error_message: Option<String>,       // 失败时的错误信息
    created_at: DateTime<Utc>,
}

enum ExperienceOutcome {
    Success,
    Failure,
}

// ============ 经验查询数据模型 ============

// 经验查询报告
struct ExperienceReport {
    query: String,                              // 查询类型
    time_range: String,                         // 时间范围
    summary: String,                            // 一句话总结
    insights: Vec<ExperienceInsight>,            // 具体发现
    recommendations: Vec<Recommendation>,        // 推荐建议
    data_points: usize,                          // 分析的数据点数量
}

// 经验洞察
struct ExperienceInsight {
    category: String,         // 分类：usage / failure / style / performance
    description: String,      // 描述
    confidence: f64,          // 置信度 0.0~1.0
    supporting_data: Value,   // 支撑数据
}

// 推荐建议
struct Recommendation {
    type_: String,            // "skill" | "workflow" | "config"
    title: String,
    description: String,
    action: Option<String>,   // 可执行的动作（如 "安装 Skill X"）
}

// ============ 并行执行数据模型 ============

// 并行不是第二套任务事实源。每个并行分支都必须是 Task(kind=parallel)，
// parent_task_id 指向父 Task，状态 / 结果 / UI 投影仍由 TaskManager、AgentTimelinePart
// 和 SessionChange 统一承接。
struct TaskBranchSpec {
    id: String,                          // 对应 Task(kind=parallel).id
    description: String,                 // 分支描述
    context: TaskBranchContext,          // 分支执行上下文（框架自动组装）
}

struct TaskBranchContext {
    project_summary: String,             // navis.md 精简版（框架自动注入）
    relevant_files: Vec<String>,         // 相关文件路径（框架验证存在）
    role_guidance: String,               // 角色行为指导（框架自动注入）
    background: String,                  // 任务背景、约束（LLM 补充）
}

struct TaskBranchResult {
    branch_id: String,                   // 对应 Task(kind=parallel).id
    summary: String,                     // 执行摘要
    files_changed: Vec<String>,          // 变更文件，最终写入 SessionChange
    success: bool,
    error: Option<String>,
}

// ============ Extended Thinking 数据模型 ============

struct ThinkingConfig {
    enabled: bool,                       // 是否启用深度推理
    budget: u32,                         // 最大思考 Token 数
    show_to_user: bool,                  // 是否展示思考过程给用户
}

struct ThinkingTrace {
    id: String,
    task_id: String,
    thinking_content: String,            // 思考过程文本
    token_count: u32,                    // 实际消耗的思考 Token
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
}
```

---

## 四、接口定义

### 4.1 IPC 命令

```typescript
// 发送消息（支持多模态输入）
// SessionMessageContent = string | { parts: ContentPart[] }
// ContentPart = TextStep | ImagePart | FilePart
// Agent 收到多模态消息后，进入 ui_stream_session_message，
// 后端先创建 user message + assistant shell，再通过 AgentTimelinePart/messages/toolApproval 推送进展。
ui_stream_session_message(payload: { sessionId: string; content: SessionMessageContent }): Channel<SessionMessageStreamChunk>

// 取消任务
agent.cancelTask(sessionId: string, taskId?: string): Promise<void>

// 确认工具调用；决策只允许四态审批动作
agent.respondToolApproval(
  requestId: string,
  decision: 'allow_once' | 'allow_session' | 'allow_project' | 'deny_always'
): Promise<void>

// 获取状态
agent.getState(sessionId: string): Promise<AgentState>
agent.getTask(sessionId: string, taskId: string): Promise<Task | null>

// 会话模型偏好切换（写入 Session，后续请求显式传给 Gateway）
agent.setModel(sessionId: string, model: string): Promise<void>

// ============ 工作模式接口 ============

// 获取当前工作模式
agent.getWorkMode(sessionId: string): Promise<WorkMode>

// 设置工作模式
agent.setWorkMode(sessionId: string, mode: WorkMode): Promise<void>

// 获取模式配置信息
agent.getModeConfig(mode: WorkMode): Promise<ModeConfig>

// 获取所有可用模式列表
agent.getAvailableModes(): Promise<WorkMode[]>

// ============ 并行 Task 接口 ============

// 创建并行 Task 分支；返回的仍是 Task.id，不返回独立 ParallelTask 类型。
agent.createParallelTasks(parentTaskId: string, branches: TaskBranchSpec[]): Promise<string[]>

// 状态读取统一走 Task 查询。
agent.getTask(sessionId: string, taskId: string): Promise<Task | null>
agent.listTasks(sessionId: string, kind?: 'parallel'): Promise<Task[]>

// 取消仍取消 Task，不引入 cancelParallelTask。
agent.cancelTask(sessionId: string, taskId?: string): Promise<void>

// 并行分支的文件变更统一进入 SessionChange，Review/Diff 负责展示与合并。

// ============ Extended Thinking 接口 ============

// 配置深度推理
agent.setThinkingConfig(config: ThinkingConfig): Promise<void>

// 获取当前思考配置
agent.getThinkingConfig(): Promise<ThinkingConfig>

// 获取思考过程追踪（用于展示给用户）
agent.getThinkingTrace(taskId: string): Promise<ThinkingTrace | null>

// ============ 自我进化接口 ============

// 获取执行经验列表
agent.getExperiences(filter?: { outcome?: ExperienceOutcome; limit?: number }): Promise<ExecutionExperience[]>

// 自我进化查询
agent.analyzeExperience(query: string, timeRange?: string): Promise<ExperienceReport>
```

---

## 五、状态机

```
         ┌──────────┐
         │   Idle   │ ←────────────────────────────┐
         └────┬─────┘                              │
              │ sendMessage                        │
              ▼                                    │
         ┌──────────┐                              │
    ┌───►│ Thinking │ ← 调用 LLM                   │
    │    └────┬─────┘                              │
    │         │ LLM 返回                            │
    │         ├── 文本回复 → Streaming              │
    │         └── 工具调用 → ToolCalling            │
    │         ▼                                    │
    │    ┌──────────┐                              │
    │    │ToolCalling│ ← 调用 MCP 工具              │
    │    └────┬─────┘                              │
    │         │                                    │
    │         ├── 需要确认 → WaitingPermission      │
    │         ├── 工具完成 → 继续调用 LLM → Thinking│
    │         └── 所有工具完成 → Streaming          │
    │         ▼                                    │
    │    ┌──────────┐                              │
    │    │WaitingPermission│ ← 等待用户确认          │
    │    └────┬─────┘                              │
    │         │ 用户确认/拒绝                       │
    │         └── 确认 → ToolCalling                │
    │         └── 拒绝 → Thinking（告知 LLM 被拒绝）│
    │         ▼                                    │
    │    ┌──────────┐                              │
    │    │Streaming │ ← 流式输出中                   │
    │    └────┬─────┘                              │
    │         │ 输出完成                           │
    │         └────────────────────────────────────┘
    │
    │ 任意状态异常
    ▼
   ┌──────────┐
   │  Error   │
   └────┬─────┘
        │ 重试/用户干预
        ▼
   ┌──────────┐
   │Recovering│
   └──────────┘
```

**Error / Recovering 恢复策略：**
- 任意状态发生异常时，状态转为 Error
- Error 状态根据错误类型决定恢复策略：

| 错误类别 | 示例 | 策略 |
|---------|------|------|
| 可重试 | 网络超时、Provider 限流（429）、临时不可用（503） | 自动重试，最多 3 次，间隔递增（1s / 2s / 4s） |
| 不可重试 | 认证失败（401）、权限拒绝（403）、参数错误（400）、用户主动拒绝工具调用 | 不重试，直接转为 Idle，通知用户错误详情 |

- 重试成功则恢复到异常前的状态继续执行
- 3 次重试均失败 → 状态转为 Idle，通知用户错误详情
- Recovering 状态下不接受新消息（`sendMessage` 返回错误提示）

### 5.2 并行任务状态机

```
  parallelExecute()
        │
        ▼
   ┌─────────┐
   │ Pending  │ ← 排队等待
   └────┬────┘
        │ 调度执行
        ▼
   ┌─────────┐      用户取消      ┌───────────┐
   │ Running  │ ─────────────────► │ Cancelled │
   └────┬────┘                     └───────────┘
        │
        ├── 执行成功 ──► Completed
        │                    │
        │                    ├── mergeParallelResult() ──► Merging ──► Completed（已合并）
        │                    └── 不合并，保留 Worktree
        │
        └── 执行失败 ──► Failed
```

---

## 六、事件定义

EventBus 中的 `action.*`、`authorization.*`、`reasoning.*` 事件只服务 Agent 内部状态、任务面板和审计联动，不是对话区公开 UI payload。对话区只消费 Stream 模块中的 `AgentTimelinePart` / `AgentTimelinePartDelta` envelope，并在历史查询中读取 `AgentTimelineParts`。后端必须先创建 assistant shell，再写入任何 AgentTimelinePart；`agent_timeline_parts.message_id` 为必填并指向该 assistant message。`tool/agent` runtime 在工具调用开始、等待审批、完成、失败时同步写入 `agent_timeline_parts`，`ai/agent` turn loop 在最终 assistant 文本落库时同步写入 `agent_timeline_parts`；同一 `turnId` 的 AgentTimelinePart 构成 Turn Timeline。向 UI 发布完整快照前必须以数据库 canonical `AgentTimelinePart` 为准，确保流式订阅视图和刷新后的历史视图一致。高频 assistant 自然语言 token 不发布裸 provider delta，而是发布绑定 `messageId + turnId + partId + field` 的 `AgentTimelinePartDelta`。工具调用 token、原生 tool_call delta 或文本形式 `<tool_call>` 不属于自然语言，必须在后端归一化为 tool AgentTimelinePart，不能进入 assistant text AgentTimelinePart。终止路径必须把已累计 partial assistant response 落成 text AgentTimelinePart 快照，再写 error/aborted AgentTimelinePart。

```typescript
type AgentEvents = {
  'agent.state.changed':     { sessionId: string; previous: AgentState; current: AgentState }
  'agent.task.started':      { sessionId: string; taskId: string }
  'agent.task.progress':     { sessionId: string; taskId: string; progress: number; message: string }
  'agent.task.completed':    { sessionId: string; taskId: string; duration: number }
  'agent.task.failed':       { sessionId: string; taskId: string; error: string }
  'agent.task.cancelled':    { sessionId: string; taskId: string }
  // agent.message.stream 已移出 EventBus，改走 Stream 模块，见 02b-stream.md
  'agent.message.complete':  { sessionId: string; messageId: string; content: string }
  'action.started':          { sessionId: string; callId: string; tool: string; args: any }
  'action.completed':        { sessionId: string; callId: string; tool: string; result: any }
  'authorization.requested': { sessionId: string; callId: string; tool: string; message: string }
  'action.failed':           { sessionId: string; callId: string; tool: string; error: string }
  'action.cancelled':        { sessionId: string; callId: string; tool: string }
  'agent.workMode.changed':    { sessionId: string; previous: string; current: string }
  'agent.parallel.cancelled':  { sessionId: string; taskId: string }
  'agent.retry.attempted':     { sessionId: string; attempt: number; maxAttempts: number; nextRetryIn: number }
  'agent.error.recovered':     { sessionId: string; fromState: string; retryCount: number }

  // ============ 并行任务事件 ============
  'agent.parallel.started':    { sessionId: string; taskIds: string[] }
  'agent.parallel.progress':   { sessionId: string; taskId: string; progress: number; message: string }
  'agent.parallel.completed':  { sessionId: string; taskId: string; summary: string }
  'agent.parallel.failed':     { sessionId: string; taskId: string; error: string }
  'agent.parallel.merged':     { sessionId: string; taskId: string; commitSha: string }

  // ============ Extended Thinking 事件 ============
  'reasoning.started':       { sessionId: string; taskId: string; budget: number }
  // reasoning.chunk 已移出 EventBus，改走 Stream 模块，见 02b-stream.md
  'reasoning.completed':     { sessionId: string; taskId: string; tokenCount: number }
  'reasoning.budgetExceeded': { sessionId: string; taskId: string; used: number; budget: number }

  // ============ 自我进化事件 ============
  'agent.experience.captured': { sessionId: string; experienceId: string; outcome: ExperienceOutcome; taskId: string }
}

// ============ Agent 监听的外部事件 ============
// Agent 订阅以下外部模块事件，用于响应跨模块状态变化：

type AgentSubscribedEvents = {
  // 项目切换 → 响应角色、模式和会话模型偏好（Agent 自行监听，非 Project 主动协调）
  // 处理逻辑：读取目标项目 navis.md 配置，更新 default_role、default_mode、session model preference，
  //           检查 Gateway 在线状态，必要时使用 offline_fallback_model，重置 Agent 状态为 Idle
  'project.switched':            { fromId: string; toId: string; fromPath: string; toPath: string }

  // Gateway 模型可用性变化 → 校验当前 Session 模型偏好
  // 处理逻辑：如果当前 Session 模型不可用，则提示用户或切换到 offline_fallback_model
  //           若切换原因是 "offline_fallback"，通知用户当前处于离线模式
  'gateway.model.switched':      { from: string; to: string; reason: string }

  // Gateway 离线 → 切换到离线模式
  // 处理逻辑：标记 Agent 为离线状态，尝试切换到 offline_fallback_model 指定的备用 Provider 模型，
  //           禁用当前不可用模型依赖的能力，通知用户离线状态
  'gateway.offline':             { reason: string; fallbackModel?: string; timestamp: number }

  // Gateway 恢复在线 → 恢复正常模式
  // 处理逻辑：取消离线状态标记，恢复依赖云端模型的功能，
  //           如果 Gateway 恢复的模型与当前 Session 模型偏好不一致，以 Session 偏好为准重新校验
  'gateway.online':              { fallbackModel?: string; restoredModel: string }
}
```

---

## 七、子系统设计

### 7.1 经验日志系统（Experience Logger）

记录 Agent 的执行经验，为后续版本的自动学习提供数据基础。

**记录时机：**
- 任务执行成功时，自动记录成功经验（工具链、执行耗时等）
- 任务执行失败时，自动记录失败经验（工具链、错误信息等）

**存储方式：**
- 按工作模式分目录存储，不同模式的经验完全隔离
- 分为全局级（`~/.navis/evolution/{mode}/experiences/`）和项目级（`.navis/evolution/{mode}/experiences/`）两层
- 项目级经验跟随项目目录，全局级经验跨项目共享

```
存储路径：
  ~/.navis/evolution/{mode}/experiences/    ← 全局经验
  <project>/.navis/evolution/{mode}/experiences/  ← 项目经验
```

**用途（v1~v2 为数据积累阶段）：**
- v1~v2：仅记录和存储执行经验，通过 `agent.getExperiences()` 查询
- v3+：基于积累的经验数据，启用自动 Skill 生成、LLM 辅助反思分析、策略优化等高级功能

#### v1 实用化：经验查询工具

v1 阶段虽然不启用自动优化，但经验日志应提供查询能力，避免成为无用数据。

**MCP 工具：**

```
agent.analyzeExperience(query: string, timeRange?: string) → ExperienceReport
```

**查询示例：**

| 用户提问 | Agent 调用 | 返回 |
|----------|-----------|------|
| "我最近一周的使用习惯是什么？" | `agent.analyzeExperience("usage_pattern", "7d")` | 最常用工具 Top 5、平均任务耗时趋势 |
| "哪些操作经常失败？" | `agent.analyzeExperience("failure_pattern", "30d")` | 频繁失败的操作列表、失败原因分类 |
| "推荐什么 Skill？" | `agent.analyzeExperience("skill_recommendation")` | 基于使用频率推荐的 Skill 列表 |
| "我的代码风格偏好？" | `agent.analyzeExperience("code_style", "30d")` | 命名风格、缩进偏好、常用模式 |

**返回结构：**

```rust
struct ExperienceReport {
    query: String,
    time_range: String,
    summary: String,                          // 一句话总结
    insights: Vec<ExperienceInsight>,          // 具体发现
    recommendations: Vec<Recommendation>,      // 推荐建议
    data_points: usize,                        // 分析的数据点数量
}

struct ExperienceInsight {
    category: String,         // 分类：usage / failure / style / performance
    description: String,      // 描述
    confidence: f64,          // 置信度 0.0~1.0
    supporting_data: Value,   // 支撑数据
}

struct Recommendation {
    type_: String,            // "skill" | "workflow" | "config"
    title: String,
    description: String,
    action: Option<String>,   // 可执行的动作（如 "安装 Skill X"）
}
```

**实现要点：**
- 查询基于本地经验日志文件，不需要 LLM 参与（纯统计分析）
- 置信度低于 0.3 的 insight 不展示（避免误导）
- 时间范围默认 7 天，最大 90 天
- 数据点不足 10 条时返回"数据不足，继续使用后可获得分析"

**推迟到 v3+ 的功能：**
- 自动 Skill 生成（基于多次成功经验的交叉验证）
- 失败反思循环（LLM 分析失败根因并生成改进建议）
- 策略优化（根据反思结果更新决策偏好）

**设计要点：**
- 经验捕获在任务完成/失败时自动触发，异步写入，不阻塞主流程
- 经验捕获开销 < 20ms

### 7.2 Task 并行执行

并行不是新的核心概念，而是 `Task.kind = parallel` 的执行形态。对标 Claude Code 的 task 工具、opencode 的 session event 订阅和 Hermes 的自治任务时，Navis Go 只保留一个 Task 底座：父 Task 派生子任务，子任务可以并行执行，后台面板只是这些 Task 的 UI 投影。

**核心机制：**
- 每个并行 Task 拥有独立的会话上下文，互不干扰
- 使用 Git Worktree 隔离写操作，避免冲突
- 执行完成后，可选择性地将 Worktree 变更合并回主分支
- 支持单个 Task 独立取消，不影响同一父 Task 下其他分支

**隔离策略：**

```
主工作目录（main）
  ├── .git/worktrees/task-1/   ← 任务 1 的 Worktree
  ├── .git/worktrees/task-2/   ← 任务 2 的 Worktree
  └── .git/worktrees/task-3/   ← 任务 3 的 Worktree
```

**约束：**
- 最大并行 Task 数由配置项 `max_parallel_tasks` 控制，默认 3
- 各任务共享只读的全局上下文（知识库、配置等）
- 写操作完全隔离在各自 Worktree 中

**Task 执行形态：**

```
Task(kind=turn)
├── 用户发起的一轮 Agent loop
├── 产生 Message / AgentTimelinePart / SessionChange
└── 可派生子 Task

Task(kind=sidechain)
├── 由 task 工具启动
├── 绑定 sidechain_session_id
└── 父会话只展示摘要、活动、工具次数、token、耗时

Task(kind=parallel)
├── 同一父 Task 下多个独立分支
├── 调度器负责并发、超时、取消和 worktree 隔离
└── 结果仍回写 Task / AgentTimelinePart / SessionChange

Task(kind=autonomous)
├── cron / webhook / remote control 等外部入口创建
├── 真正执行仍进入 tool/agent catalog / pipeline / runtime / guardrail / hooks / special
└── 不绕过 SessionEvent、AgentTimelinePart 和 Kernel Policy
```

### 7.2.1 TaskScheduler adapter（执行适配器）

`TaskScheduler` 是执行适配器，不是事实源。Agent 可以通过它调度并行 Task，但 Task 的创建、状态、取消、结果和 UI 投影仍由统一 Task 底座管理。

```rust
/// Task 调度适配器，不能持有第二套任务事实源。
trait TaskScheduler: Send + Sync {
    /// 执行一组并行 Task 分支
    async fn execute_branches(
        &self,
        branches: Vec<TaskBranchSpec>,
        config: SchedulerConfig,
    ) -> Result<Vec<TaskBranchResult>>;

    /// 取消指定执行分支
    async fn cancel_branch(&self, branch_id: &str) -> Result<()>;

    /// 获取执行分支进度
    fn get_progress(&self, branch_id: &str) -> Option<TaskProgress>;
}

/// 分支规格（Agent 传递给调度器的输入）
struct TaskBranchSpec {
    id: String,
    description: String,
    context: TaskBranchContext,
    mode_config: ModeConfig,
}

/// 调度器配置
struct SchedulerConfig {
    max_concurrency: usize,         // 最大并发数（默认 3）
    single_task_timeout: Duration,   // 单任务超时（默认 5 分钟）
    total_timeout: Duration,         // 总超时（默认 30 分钟）
}
```

**依赖方向：**
```
Agent 模块
    │
    ├── 持有 Task 事实源
    ├── 依赖 TaskScheduler adapter（只负责执行）
    │
    └── 不依赖 Task Sidechain 的状态模型

Task Sidechain 模块
    │
    └── 实现 TaskScheduler adapter（SidechainTaskScheduler）
        只返回执行进度和结果，不创建第二套用户可见任务概念
```

### 7.3 Extended Thinking（深度推理）

长思考链模式，用于处理复杂问题时进行深度推理。

**工作原理：**
- 通过 API 的 thinking 参数开启深度推理模式
- 模型在返回最终答案前，先进行内部思考链推理
- 思考过程消耗的 Token 计入专用的 thinking budget
- 思考内容可选择性地流式展示给用户

**配置项：**

```rust
ThinkingConfig {
    enabled: true,         // 是否启用深度推理
    budget: 16000,         // 最大思考 Token 数（默认 16K）
    show_to_user: true,    // 是否将思考过程展示给用户
}
```

**使用场景：**
- 复杂架构设计决策
- 多步骤推理问题
- 代码调试中的根因分析
- 需要反复推敲的方案评估

**与普通 Thinking 状态的区别：**
- 普通 Thinking 状态是调用 LLM 时的等待状态
- Extended Thinking 是 LLM 内部的深度推理过程，输出额外的 thinking_content
- 两者可以叠加：进入 Thinking 状态后，若开启了 Extended Thinking，会先输出思考链，再输出最终回复

**Token 计费：**
- `thinking_tokens` 计入 Gateway `TokenUsage` 统计
- 思考 Token 受 Quota 限制，与其他 Token 共享配额管控
- 思考 Token 单独统计成本，便于用户了解深度推理的额外消耗

---

## 八、性能指标

| 指标 | 要求 | 说明 |
|------|------|------|
| 状态切换延迟 | < 5ms | 内存操作 |
| 任务启动 | < 50ms | 含上下文组装 |
| 流式 chunk 推送 | < 2ms | 从 Gateway 到前端 |
| 并行任务调度 | < 100ms | 含 Worktree 创建 |
| 经验捕获开销 | < 20ms | 异步写入，不阻塞主流程 |
| 深度推理额外延迟 | 取决于模型 | 思考链长度由 budget 控制 |

---

## 九、测试策略

```
单元测试：状态机转换、工具调用调度、取消/超时处理
集成测试：完整对话流程、多轮工具调用、用户确认流程
自我进化测试：经验日志记录准确性
并行任务测试：多任务并发执行、Worktree 隔离与合并、单任务取消不影响其他任务
Extended Thinking 测试：深度推理开关、Token 预算控制、思考过程流式输出
```
