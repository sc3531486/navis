# opencode Agent Flow Source Study

> 目标：完整说明 opencode 从收到用户指令到前端展示回复的主流程，并标明关键源码位置。本文只记录 opencode 源码事实和由源码直接推出的设计原则，不描述 Navis Go 当前实现。

## 1. 结论

opencode 收到用户指令后，不会先扫描当前项目所有文档，也不会把目录树、重要文件摘录或目录快照默认注入每一轮 prompt。本文中的 `workspace root` / `workspaceCreating` / `Workspace root folder` 是 opencode 源码里的第三方术语；映射到 Navis Go 时只能对应 `Session.worktree_root` 或创建会话目录的 UI 状态，不能引入新的业务域概念。

它的主设计是：前端提交用户消息，后端保存 user message，进入 `SessionPrompt.runLoop`，动态组装最小系统上下文、说明文件、技能摘要、历史消息和工具定义，然后让模型根据工具描述决定是否调用 `read`、`glob`、`grep`、`edit`、`shell` 等工具。文件上下文来自用户显式附件或模型显式工具调用，而不是每轮自动扫目录。

`Snapshot` 在 `SessionProcessor` 中出现，但用途是流式过程前后的变更追踪和 diff 关联，不是 prompt 上下文注入。

## 2. 源码索引

| 阶段 | 源码 | 职责 |
| --- | --- | --- |
| TUI 输入提交 | `packages/opencode/src/cli/cmd/tui/component/prompt/index.tsx` | 处理 Enter 提交、防重复提交、创建 session、区分 shell/slash/prompt、清空输入和写历史 |
| 非交互 CLI | `packages/opencode/src/cli/cmd/run.ts` | `opencode run` 把用户输入提交到 session prompt |
| Run 流式传输 | `packages/opencode/src/cli/cmd/run/stream.transport.ts` | 异步提交 prompt，配合流式消费 |
| HTTP handler | `packages/opencode/src/server/routes/instance/httpapi/handlers/session.ts` | `SessionHttpApi.prompt` / `promptAsync` 校验 session 并调用 prompt service |
| 用户消息构造 | `packages/opencode/src/session/prompt.ts` | `createUserMessage`、`resolveUserPart`、保存 user message 和 parts |
| Agent 主循环 | `packages/opencode/src/session/prompt.ts` | `SessionPrompt.prompt` 和 `SessionPrompt.runLoop` |
| 环境 prompt | `packages/opencode/src/session/system.ts` | 注入模型 ID、cwd、`workspace root`、git、platform、date；这是 opencode 第三方术语 |
| 指令文件 | `packages/opencode/src/session/instruction.ts` | 加载全局/项目/配置声明的 instruction 文件；读文件后才补附近 instruction |
| 工具注册 | `packages/opencode/src/tool/registry.ts` | 汇总内建工具、扩展/MCP 工具，并按模型、agent、权限过滤 |
| 工具执行桥 | `packages/opencode/src/session/tools.ts` | 把 LLM tool call 连接到真实工具执行和权限系统 |
| 请求准备 | `packages/opencode/src/session/llm/request.ts` | 合并 provider/agent/system/user system，准备参数、headers、tools |
| LLM 流 | `packages/opencode/src/session/llm.ts` | 选择 native runtime 或 AI SDK `streamText`，统一输出 LLMEvent |
| AI SDK 事件转换 | `packages/opencode/src/session/llm/ai-sdk.ts` | 将 provider stream part 标准化为 text/reasoning/tool/finish 事件 |
| 流处理与落盘 | `packages/opencode/src/session/processor.ts` | 将 LLMEvent 写入 assistant AgentTimelineParts，处理工具状态、错误、完成 |
| 事件 schema | `packages/llm/src/schema/events.ts` | LLMEvent 统一协议 |
| 工具循环 helper | `packages/llm/src/tool-runtime.ts` | 通用工具轮次调度辅助 |
| 前端同步 store | `packages/opencode/src/cli/cmd/tui/context/sync.tsx` | 根据 `message.updated`、`message.part.updated`、`message.part.delta` 更新 UI store |
| 前端消息渲染 | `packages/opencode/src/cli/cmd/tui/routes/session/index.tsx` | 渲染 assistant 的 text/reasoning/tool/error/footer |
| v2 消息渲染 | `packages/opencode/src/cli/cmd/tui/feature-extensions/system/session-v2.tsx` | 新事件系统下渲染 assistant content |

## 3. 用户提交流程

### 3.1 TUI Prompt

`prompt/index.tsx` 的 `submit()` 是 TUI 正常输入入口。它先用 `submitting` 防止重复提交，避免双击 Enter 生成空 prompt。

`submitInner()` 做这些事：

- 同步 IME 输入内容，防止组合输入还没 flush 就提交。
- 检查 disabled、`workspaceCreating`、autocomplete、空输入、agent、model；`workspaceCreating` 是 opencode 前端状态名。
- 如果当前没有 session，则调用 `sdk.client.session.create()` 创建 session，并绑定 agent 和 model。
- 读取当前输入文本、附件 parts、编辑器选区 synthetic part。
- 根据输入模式分支：shell、slash command、normal prompt。
- normal prompt 调用 `sdk.client.session.prompt(...)`，传入 `sessionID`、`messageID`、agent、model、variant 和 parts。
- 提交后写入 prompt history，清空输入框，清理 extmarks。

这里前端不会自己构造 assistant 回复，也不会自己伪造工具过程；它只是提交用户输入并等待后端事件同步。

### 3.2 CLI / API

非交互模式通过 `run.ts` 提交 prompt。run transport 通过 `stream.transport.ts` 提交 async prompt。

HTTP 层在 `handlers/session.ts` 中提供两个入口：

- `SessionHttpApi.prompt`：同步调用 `promptSvc.prompt` 后返回 JSON stream。
- `SessionHttpApi.promptAsync`：后台 fork `promptSvc.prompt`，失败时发布 session error。

这说明 opencode 的客户端入口可以多样，但最终都收敛到同一套 `SessionPrompt`。

## 4. 用户消息构造

`session/prompt.ts` 的 `createUserMessage` 负责把前端 payload 转为 session 中的 user message 和 parts。

关键点：

- 文本 part 直接进入 user message。
- 文件 part 会走 `resolveUserPart`。
- MCP resource 会被读取并转为文本/附件。
- 本地 file URL 会根据 mime、目录、LSP symbol 等做解析。
- 当文件附件需要读取内容时，opencode 实际调用的是 read tool 语义，不是静默扫项目。
- `referenceContextFromFilePart` 只围绕用户提供的 file part 建 reference context。

所以“上下文获取”有明确触发源：用户附件、MCP resource、模型工具调用。没有看到“每轮先扫描所有文档”的逻辑。

## 5. Agent 主循环

`SessionPrompt.prompt` 的顺序：

1. 读取 session。
2. 清理 revert 状态。
3. 调用 `createUserMessage` 保存用户消息。
4. touch session。
5. 如果 `noReply` 为 true，直接返回 user message。
6. 否则进入 `loop({ sessionID })`。

`SessionPrompt.runLoop` 是主状态机：

1. 设置 session status 为 busy。
2. 读取 `MessageV2.filterCompactedEffect(sessionID)`。
3. 计算 latest user、latest assistant、finished 状态和 pending tasks。
4. 如果 assistant 已完成且没有未完成工具调用，退出循环。
5. 第一轮 fork session title 生成。
6. 加载用户选择的 provider/model。
7. 如果有 subtask task，执行 subtask 分支。
8. 如果有 compaction task，执行 compaction 分支。
9. 如果 token 溢出，创建自动 compaction task。
10. 解析 agent，计算 maxSteps。
11. 应用 session reminders。
12. 创建 assistant message。
13. 创建 `SessionProcessor` handle。
14. 解析本轮 tools。
15. 并发组装 skills、environment、instructions、model messages。
16. 调用 `handle.process(...)` 进入 LLM stream。
17. 如果产生 tool calls，processor 写入工具 part，工具执行结果进入下一轮模型消息，循环继续。
18. 如果 finish/stop/error/compact，结束或进入压缩。

这个循环是“模型-工具-模型”的真实 agent loop，不是单次 Gateway chat。

## 6. System / Context 组装

`SystemPrompt.environment(model)` 只返回环境事实：

- 当前模型名和 provider/model ID。
- Working directory。
- `Workspace root folder`，即 opencode 环境 prompt 的目录边界字段；Navis Go 设计中映射为当前 `Session.worktree_root`。
- 是否 git repo。
- Platform。
- Today's date。

它不包含文件树，不包含项目摘要，不包含当前目录所有文档。

`instruction.system()` 加载全局、项目、配置声明的 instruction 文件。`instruction.ts` 还有“附近 instruction 文件”的逻辑，但那是在 `read` 工具读取具体文件后才解析，用来给该文件阅读结果补充局部规则，不是每轮提前扫描。

`SystemPrompt.skills(agent)` 只注入可用 skill 摘要，并提示模型在任务匹配时调用 skill tool 加载完整 skill。它也不是把所有 skill 内容全部塞进上下文。

最终 system 顺序在 `prompt.ts` 中是：

```text
environment + instructions + skills
```

再由 `LLMRequestPrep.prepare` 合并 provider prompt、agent prompt、system 和 user system。

## 7. 工具设计

opencode 的工具语义非常明确：

- `read.txt`：读取已知文件或目录；如果不确定路径，先用 `glob`。
- `glob.txt`：根据文件名 pattern 查找文件。
- `grep.txt`：按内容搜索文件。
- `read.ts`：只有被调用时才读文件或目录。
- `read.ts`：读取具体文件后，会解析附近 instruction 文件作为 system reminder 附加到读取结果。
- `registry.ts`：按 provider/model/agent/permission/extension/MCP 过滤工具。
- `registry.ts`：对 GPT 模型族选择 `apply_patch` 或 `edit/write` 的可见性。

工具不是前端菜单行为的临时解释器，而是统一注册、统一过滤、统一执行、统一写 session part。

## 8. LLM 请求与流式处理

`LLMRequestPrep.prepare` 做以下事情：

- 把 provider prompt 或 agent prompt 放在最前。
- 拼接 system、user.system。
- 触发扩展 `experimental.chat.system.transform`。
- 按 provider/model/agent variant 合并参数。
- 触发扩展 `chat.params`、`chat.headers`。
- 用 permission 和 user tool flags 二次过滤工具。
- 生成 provider headers，包括 session/request/client 标识。

`LLM.stream` 做以下事情：

- 尝试 native runtime，支持则直接返回标准 LLMEvent stream。
- native 不支持时使用 AI SDK `streamText`。
- 对 tool call 做修复，例如小写工具名映射。
- 将 AI SDK `fullStream` 转为统一 LLMEvent。

`SessionProcessor.process` 消费 LLMEvent：

- `reasoning-start/delta/end` 写 reasoning part。
- `tool-input-start/end` 写工具输入过程。
- `tool-call` 写 running tool part，并做 doom loop 防护。
- `tool-result` 写 completed tool part。
- `tool-error` 写 error tool part。
- text delta 写 assistant text part。
- finish 写 assistant message finish、tokens、cost、time。

这一层是前端展示过程的来源，前端不应该猜工具状态。

## 9. 前端展示流程

opencode TUI 的展示链路是：

1. 输入区提交后清空 prompt，历史记录保存本次输入。
2. 后端保存 user message 并发布 message 事件。
3. `context/sync.tsx` 收到 `message.updated`，按 message id 插入或 reconcile。
4. 后端处理 LLM stream 时持续发布 part 事件。
5. `context/sync.tsx` 收到 `message.part.updated`，按 part id 插入或 reconcile。
6. `context/sync.tsx` 收到 `message.part.delta`，对文本类 part 增量更新。
7. session 页面从 `sync.data.message[sessionID]` 和 `sync.data.part[message.id]` 派生视图。
8. `AssistantMessage` 按 part 类型渲染 `TextStep`、`ReasoningStep`、`ToolStep`。
9. `ToolStep` 再按工具名选择 Shell、Glob、Read、Grep、WebFetch、WebSearch、Write、Edit、ApplyPatch、Task、Question、Skill 或 GenericTool。
10. footer 显示 agent、model、耗时、interrupted 等状态。
11. message error 用独立错误块显示，不混进最终文本。

前端展示效果的核心不是“漂亮控件”，而是数据结构稳定：assistant message 是一个容器，text/reasoning/tool/error 都是 part。工具过程天然和最终回复在同一条消息时间线里。

## 10. 压缩与摘要

opencode 的 compaction 不等于“每轮读项目”。它发生在会话 token 压力或用户触发摘要时。

主循环中如果 latest finished tokens 对当前 model 溢出，会创建 auto compaction task。compaction 读取历史消息，选择保留头尾，调用 compaction agent 生成摘要，并处理媒体、工具输出截断。压缩后的历史再进入后续 model messages。

这保证上下文管理针对的是会话历史，而不是每轮重新扫描当前 `Session.worktree_root`。

## 11. 扩展与扩展点

opencode 在多个层面提供扩展：

- system transform：扩展可以改 system。
- params/header transform：扩展可以改模型参数和 header。
- tool.definition：扩展可以改工具定义。
- chat.messages.transform：扩展可以改消息。
- MCP：外部工具可以进入 tool registry。
- TUI 扩展：前端可通过 sync data 和事件访问 session/message/part。

重要的是：扩展点围绕统一数据结构展开，而不是每个 UI 区域各自解释一套行为。

## 12. 对 Navis Go 的直接启示

Navis Go 不应该继续使用“每轮 Agent Turn Context 自动注入 worktree tree 和关键文件摘录”的设计。这个设计会制造三个问题：

- 延迟和 token 浪费：每个用户指令都付出项目扫描成本。
- 行为污染：模型会把旧快照当成事实，修改后更容易答错。
- 交互错误：用户只是说一句普通话，也触发“扫描全部文档”的 agent 行为。

应该采用 opencode 的结构基线：

- 每轮只注入环境事实、模式角色、用户/项目 instruction、技能摘要、历史消息。
- 文件上下文必须来自用户附件、编辑器选区、明确的 read/search/list 工具调用。
- 工具执行结果必须写入同一条 workflow/message part 时间线。
- 前端只订阅后端事件并渲染，不伪造工具过程。
- 压缩只处理会话历史，不处理 worktree tree。

Navis Go 可以在这个基础上优化 UI、扩展契约、Gateway 边界和 AgentTimelineParts，但不能回到“每轮先扫项目”的设计。
