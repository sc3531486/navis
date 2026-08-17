# Navis Go Agent Flow Optimized From opencode and Claude Code

> 目标：在 opencode 与 Claude Code 源码事实基础上，设计 Navis Go 的 agent、Gateway、工具、前端交互和回复展示流程。本文是后续重构 Navis Go 的基准设计，不表示当前代码已经全部实现。

## 1. 设计立场

Navis Go 应该吸收 opencode 的核心结构：用户消息进入统一 session prompt，后端保存消息，`ai/agent` turn loop 组装最小上下文并决定工具调用，`tool/agent` 提供工具目录、契约、批处理、管线、执行、结果、保护和 hook，Gateway 负责模型流，Processor/Recorder 负责把文本、推理、工具、错误写成稳定的可恢复步骤，前端订阅事件渲染。

Navis Go 应该吸收 Claude Code 的交互韧性：streamingText / streamingToolUses / streamingThinking 分离，权限提示可解释，中断保留 partial assistant response，stream idle watchdog 防止挂死，工具结果和 tool_use 配对严格。

Navis Go 只吸收两者的宿主层契约，不吸收 opencode 的双事件路径、TUI 专用展示逻辑、不同前端扩展层的重复路径，也不吸收 Claude Code 的产品特性分叉、远控/bridge 多套路径和发布包历史负担。Navis Go 是桌面应用，应该把 AgentTimelinePart 作为第一等 UI/存储契约，一次设计清楚。这里的 `AgentTimelinePart` 是 Navis Go 自己的命名，不是 Claude Code 或 opencode 的源码术语。

### 1.1 取长补短原则

| 来源 | 借鉴 | Navis Go 优化 |
| --- | --- | --- |
| opencode | `message + part` 事实源，事件入库后视图订阅，工具/权限/LLM event 规范化 | 不照搬 HTTP SSE/TUI 结构；桌面主链路使用 Tauri Channel，`agent_timeline_parts` 是可恢复事实源 |
| opencode | 不先扫目录，不默认注入 worktree tree，按需通过工具读上下文 | 写成硬规则：PromptAssembler 只组装用户输入、系统约束、会话摘要、已选择附件和必要 instruction |
| opencode | `message.part.updated` / `message.part.delta` 分层 | Navis Go 使用 `AgentTimelinePart` 完整快照 + `AgentTimelinePartDelta` 高频字段追加，前端不接裸 provider delta |
| Claude Code | stream event 只驱动临时 UI，最终 assistant/user/system message 才进入 transcript | Navis Go 的 delta 只更新当前 `AgentTimelinePart`；最终以数据库 canonical `AgentTimelinePart` 覆盖 UI |
| Claude Code | `streamingText`、`streamingToolUses`、`streamingThinking` 分离，spinner mode 表示真实阶段 | Navis Go 用 `turn_prelude`、`text`、`tool`、`permission`、`gateway_retry`、`turn_finalizer` 分阶段展示 |
| Claude Code | 取消时保留已生成 partial assistant 文本，中断/恢复不丢历史 | Navis Go 在 cancel/error/timeout 时先写 aborted/error text AgentTimelinePart，再写终止 AgentTimelinePart |
| Claude Code | stream idle watchdog 和 stall 检测 | Navis Go 使用 `requestTimeoutSecs + 120s` 作为 idle timeout：无新 chunk 才终止，不限制长回答总时长 |
| Claude Code | 权限请求队列和可解释 prompt | Navis Go 权限落成 `PermissionStep`，支持 `allow_once`、`allow_session`、`allow_project`、`deny_always` 建议选项 |
| 两者共同点 | 工具调用必须有稳定 ID，tool_use/tool_result 严格配对 | Navis Go 后端以 `callId` 绑定工具生命周期，但 UI 流只按稳定 `partId` 更新 AgentTimelinePart；缺失 ID 或同一工具生命周期更换 `partId` 都是后端协议错误 |

## 2. opencode 怎么做

opencode 的主流程是：

```text
Client submit
-> HTTP session.prompt / promptAsync
-> createUserMessage
-> SessionPrompt.runLoop
-> resolve agent/model/tools
-> build environment + instructions + skills + message history
-> LLMRequestPrep.prepare
-> LLM.stream
-> SessionProcessor.process
-> text/reasoning/tool/error AgentTimelineParts
-> frontend sync store
-> frontend renders assistant AgentTimelineParts
-> if tool results exist, runLoop continues
```

关键设计点：

- 不自动扫描项目所有文档。
- 不自动注入 worktree tree。
- 文件上下文由附件、编辑器选区或工具调用产生。
- 工具定义统一注册、统一过滤、统一执行。
- LLM provider stream 被规范化成统一事件。
- 工具执行过程写入 assistant AgentTimelinePart。
- 前端展示由后端事件驱动。

## 2.1 Claude Code 怎么做

本地可读资料包括 `D:\myworkspace\claude-code-main` 的完整源码。关键入口为 `src/query.ts`、`src/QueryEngine.ts`、`src/services/api/claude.ts`、`src/utils/messages.ts`、`src/screens/REPL.tsx`、`src/components/messages/*`、`src/services/tools/StreamingToolExecutor.ts` 和权限相关 hook/组件。

Claude Code 的一轮任务不是“用户发出指令后界面沉默等待模型结束”，而是由多个可观察阶段组成：

```text
UserPromptSubmit hooks
-> session/user/system context assembly
-> assistant run starts, spinner/status line shows active state
-> model thinking / first token waiting state
-> tool_use emitted
-> permission decision / PreToolUse hooks
-> tool execution
-> PostToolUse hooks and tool result block
-> assistant text display, MessageDisplay hooks
-> Stop/SidechainStop hooks
-> finalizer records usage/duration/tool count/result status
```

关键设计点：

- `query()` 是 async generator，`queryLoop()` 负责模型流、工具调用、压缩、错误恢复和中断收尾；调用方通过 `for await` 消费事件。
- `services/api/claude.ts` 使用 raw stream，自己累积 text/tool/thinking block，避免 SDK partial JSON O(n²) 解析；`content_block_stop` 时才生成规范化 assistant message。
- `handleMessageFromStream()` 把 provider stream event 映射为 `streamingText`、`streamingToolUses`、`streamingThinking` 和 spinner mode；最终消息到达时清空 streaming 临时态并 append canonical message。
- `REPL.tsx` 维护 `messages`、`streamingText`、`streamingToolUses`、`streamingThinking`、`toolUseConfirmQueue`；权限、prompt、sandbox 等等待态都会进入队列，而不是变成普通 assistant 文本。
- 中断时如果 `streamingText` 非空，先追加 partial assistant message，再 abort 当前 controller，保证用户能看到中断前已经生成的内容。
- stream watchdog 是 idle 语义：长时间没有 chunk 才 abort stream；stall detection 只记录相邻 chunk 间隔，二者不等同于总时长 deadline。

Navis Go 的吸收方式：

- 保留 opencode 的“消息下挂可恢复执行节点”时间线思想，但不沿用 `part` 作为 Navis Go 公共命名；增加 Claude Code 式生命周期分层：`turn_prelude`、`tool_decision`、`permission`、`tool_step_completed`、`message_display`、`stop_check` 都应成为可记录 AgentTimelinePart。
- 每轮成功结束必须写入 `source = "turn_finalizer"` 的 summary AgentTimelinePart。它表示 assistant 文本、工具结果和 turn metadata 已经保存；后续 Stop hook / completion verifier / duration summary 都挂在这个阶段。
- UI 不再只有一个全局 loading spinner；每轮从后端第一条 `turn_prelude` AgentTimelinePart 开始，之后 thinking、tool running、permission waiting、streaming、stop checking 分阶段展示。`turn_prelude` 是运行态状态，不是 assistant 自然语言；当本轮尚无真实 assistant text 可显示时它可以与工具行同时展示，finalizer 写入前必须更新为 `completed`，历史视图不得长期保留 running prelude。
- 扩展扩展不直接改聊天 UI，而是贡献生命周期 hook 或工具 / 面板能力，由 AgentTimelinePartRecorder 统一落盘和发布。
- 远程和扩展工具只认 MCP `tools/list` 发现出的 `ToolDefinition`。Agent 运行时通过 Tool Catalog 将 provider-safe tool name 反查到 MCP canonical name，再由 MCP Executor 发送 `tools/call`。Server 未连接、`tools/list` 不合法或 `tools/call` 失败时，必须写 error AgentTimelinePart 并把错误作为 tool result 回注模型；不得创建 StubTool、空成功结果或静默本地替代。
- 当前真实 transport 先落 stdio：启动 MCP 子进程后用 `Content-Length` framing 完成 `initialize -> initialized -> tools/list/tools/call`。SSE / WebSocket / REST / gRPC 未完整实现前只返回真实 `not implemented` 错误，不能假装连接成功并返回 `null`。内置 clipboard/memory 也必须返回真实执行结果；`memory.recall/store` 读写 SQLite `memories` 表并声明 `scope = "persistent"`，不能使用进程缓存或伪造持久记忆。
- Memory 写入和上下文注入分离：`memory.store` 立即落库，同一 `scope_type + scope_id + category + key` 覆盖旧值；PromptAssembler 在每轮上下文组装这个安全刷新点读取 bounded memory snapshot，避免同一 turn 流式过程中系统上下文突变。`memory.recall` 作为普通 tool result 回注模型，必须有 query/result 上限和 category/key/confidence 元数据；显式传 scope 时只查指定层，默认在当前 session 下按 `global -> mode -> project -> session` 查完整 scope stack。
- Memory scope 采用四层合并：`global -> mode -> project -> session`。`mode` 层覆盖 `code / cowork / custom mode extension`；Custom 扩展不能自建私有 memory prompt 注入，只能通过同一 SQLite/MCP memory scope 写入和读取。
- 权限链路采用双层约束：Agent UI 根据权限矩阵和 `allow_once / allow_session / allow_project / deny_always` 决策，允许后只给当前 tool call 注入内部一次性 grant；MCP Executor 收到 Sandbox `require_confirm` 但没有 grant 时必须拒绝执行。grant 不进入模型上下文和 UI input，只作为运行时执行凭证。

## 3. Navis Go 要怎么做

Navis Go 的优化目标不是“照抄 opencode 文件结构”，而是保留它正确的边界，并按桌面产品和扩展化要求重新整理。

Navis Go 主流程：

```text
Composer submit
-> UiSessionMessageService
-> SessionMessageStore persists user message
-> AgentTurnRunner owns one turn loop
-> PromptAssembler builds minimal prompt
-> tool/agent catalog resolves available tools
-> GatewayRequestBuilder builds provider request
-> GatewayStreamRuntime reads stream
-> StreamEventNormalizer normalizes deltas/tool calls/errors
-> AgentTimelinePartRecorder persists and publishes steps
-> Frontend renders AgentTimelineParts
-> tool/agent runtime / MCP executes tool calls
-> Remote MCP tools execute through tools/call only
-> Tool results are appended and loop continues
```

## 4. 为什么这样比直接照搬更好

| 设计点 | opencode | Navis Go 优化 |
| --- | --- | --- |
| UI 形态 | TUI 为主，桌面 UI 不是核心约束 | 桌面 UI 以 AgentTimelineParts 为核心，可同时服务聊天区和右侧面板 |
| 事件系统 | 双事件路径 | 直接使用单一 `AgentTimelinePart` 协议，避免协议分叉 |
| 工具展示 | TUI 里按工具名映射组件 | 工具注册自带 `ui_hint`，前端可通用渲染，特殊工具再扩展 |
| 菜单行为 | 有扩展扩展，但 UI 区域仍有一些专用逻辑 | 菜单、命令面板、agent tools、右侧视图使用清晰的能力接口，共享底层工具/业务服务 |
| Gateway | provider runtime 和 agent loop 边界足够清楚 | 进一步把 Gateway 限定为协议适配，不解释 agent 行为 |
| 配置 | 有 provider 特殊路径 | 用户 UI 只暴露真实用户设置；`requestTimeoutSecs` 是 provider-level timeout，stream idle timeout = 配置 + 120 秒 |
| 模型默认值 | provider/model/variant 都在消息中流动 | Default model 只以 `model.id` 关联，展示字段变化不影响选择 |
| 上下文 | 不扫项目树 | 明确写成硬规则，移除 Navis Go 当前 worktree snapshot 注入设计 |

## 5. 后端模块设计

### 5.1 UiSessionMessageService

职责：

- 接收前端 `ui_stream_session_message`。
- 校验 session 是否存在。
- 解析当前工作模式 `Code`、`Cowork`、`Custom(mode_id)`。
- 校验 model id 是否来自已添加模型列表。
- 生成 `turnId` 和 `requestId`。
- 防止重复提交，重复 requestId 返回已有 turn stream。
- 不直接调用裸 Gateway chat。

输入契约：

```json
{
  "sessionId": "s_...",
  "turnId": "t_...",
  "mode": "code",
  "modelId": "gpt-4.1",
  "parts": [
    { "type": "text", "text": "..." },
    { "type": "file_ref", "path": "D:/repo/src/main.ts" },
    { "type": "editor_selection", "path": "D:/repo/src/main.ts", "ranges": [{ "start": 10, "end": 30 }] }
  ]
}
```

### 5.2 SessionMessageStore

职责：

- 先落盘 user message，再启动模型调用。
- 创建 assistant turn shell，便于所有 AgentTimelinePart 都绑定真实 `messageId`。
- 立即写入并推送第一条 `reasoning` AgentTimelinePart，`source = "turn_prelude"`，用于展示本轮已经开始和当前阶段。它不能复述用户请求，也不能伪装成模型自然语言回复；应接近 Claude Code 的 stream mode/status line：`requesting`、`thinking`、`tool-input`、`responding`。这不是前端 loading 文案，必须入库，刷新后仍可回放。
- 保存 message、AgentTimelineParts、token/cost、finish、error。
- 支持历史回放时直接从 AgentTimelineParts 恢复 UI。
- 保存 compaction 元数据，但不删除原始消息。

硬规则：

- 前端不维护另一份不可落盘的“临时工具过程”。
- 所有可见过程都必须来自 store 或 store 事件。
- 错误和取消也必须落盘，否则刷新后历史会消失。
- 原始消息、工具 AgentTimelinePart、错误 AgentTimelinePart 必须可审计；压缩只影响 PromptAssembler 如何取上下文，不影响用户查看历史。

软压缩元数据：

```ts
type CompactedRange = {
  id: string;
  sessionId: string;
  startMessageId: string;
  endMessageId: string;
  tailStartMessageId?: string;
  summaryMessageId: string;
  summarypartId?: string;
  tokenBefore: number;
  tokenAfter: number;
  createdAt: string;
  trigger: "auto_overflow" | "manual";
};
```

存储规则：

- 原始消息不删除，只标记被某个 `CompactedRange` 覆盖。
- 压缩摘要保存为 assistant/system summary message，也保存一个 `SummaryStep`，用于前端显示 `Compacted context` 分隔线。
- `PromptAssembler` 构建上下文时，用 summary 替换被覆盖 range 的原始消息。
- 若 summary message 或 range 边界缺失，后端必须记录 warning 并保留原始历史，不得硬删消息。
- 前端历史默认显示完整对话；当 range 很长时可折叠为 summary，但用户必须能展开原始消息。

### 5.3 AgentTurnRunner

职责：

- 持有单轮 agent loop。
- 处理 busy、streaming、toolCalling、waitingPermission、completed、error、aborted 状态。
- 控制 max steps，防止工具死循环。
- 调用 compaction。
- 负责取消和重试恢复。

伪流程：

```text
start turn
record turn prelude AgentTimelinePart
while true:
  load compacted history
  if completed and no pending tools: break
  resolve mode config
  assemble prompt
  resolve tools
  stream gateway response
  record AgentTimelineParts
  if tool calls: execute tools and continue
  if token overflow: compact and continue
  else finish
record turn finalizer AgentTimelinePart
```

熔断规则：

- `maxSteps` 默认 8，mode 或 agent 可降低，不能无限提高。
- 单轮 wall-clock 上限默认 10 分钟；超过后写 `ErrorStep` 并停止。
- 单个工具连续失败 2 次停止该工具，允许模型解释失败原因，不允许继续重试同一输入。
- 同一 `toolId + normalizedInput` 连续出现 3 次视为 doom loop，停止并写 `ErrorStep`。
- `read -> search -> read -> search` 且连续 3 轮没有新增文件、命中或文本输出，视为空跑循环，停止并提示用户补充目标。
- 每轮工具调用总数默认 30，超过后停止。
- 每轮累计工具输出超过本模型 context window 的 20% 时，必须先压缩或截断工具输出，再继续。
- 用户取消后，所有 running AgentTimelinePart 标记 `aborted`，不得继续执行未开始的工具。

熔断落盘：

- 熔断原因写入 `ErrorStep`，字段包括 `reason`、`toolId`、`callId`、`step`、`normalizedInputHash`。
- 熔断不删除已执行工具结果。
- 熔断后的 Retry 必须从新 assistant attempt 开始，不能在原 attempt 上继续追加。

### 5.3.1 CompactionManager

触发条件：

- 当前 prompt 近似 token 预算超过模型 context window 的安全水位，默认 70%，不是精确 80%。
- 用户手动点击 `Compact context`。
- 单轮工具输出过大，导致后续请求无法进入模型。

Token 预算规则：

- token 估算只能作为近似值，不要求跨模型精确。
- 按 provider/model 选择可用 tokenizer；没有精确 tokenizer 时使用保守估算。
- 估算结果必须乘安全系数，默认 1.2。
- 如果 provider 返回 context overflow，CompactionManager 记录真实失败并降低该模型本 session 的水位。
- 不允许因为估算误差在同一轮反复 compact；一轮最多 compact 一次。

压缩粒度：

- 粒度是 message range，不是单个 token 片段。
- 最近消息保留受硬预算约束，不使用固定“8 轮必保留”。
- 默认先保留最近 20% prompt budget 的原文，再从最近往前追加，直到达到预算上限。
- 如果最近单轮本身过大，必须截断或摘要该轮的大型工具输出，而不是强行保留整轮。
- 永远保留第一条用户目标消息、系统模式信息、未完成工具调用、权限决策、错误 AgentTimelinePart。
- 大型工具输出在摘要中只保留标题、参数、关键结果、错误和文件路径，不保留完整 stdout。

压缩流程：

```text
estimate tokens
select middle range to compact
build compaction prompt from selected range
call compaction model/agent
persist summary message + SummaryStep
persist CompactedRange metadata
PromptAssembler replaces selected range with summary
frontend can show full history or collapsed summary
```

压缩产物：

- `SummaryStep.kind = "summary"`。
- `SummaryStep.status = "completed"`。
- `SummaryStep.summary` 存短标题，例如 `Compacted 42 messages into 1.8k tokens`。
- `SummaryStep.detail` 存摘要正文。
- `CompactedRange` 存覆盖范围和 token 前后对比。
- 压缩摘要允许用户在右侧面板查看和手动编辑；手动编辑会生成新的 summary revision，不覆盖旧摘要。
- 用户可以对某个 `CompactedRange` 执行 `Restore original in prompt for next turn`，仅下一轮临时使用原文，不删除摘要。

Prompt 拼接：

- 未被压缩的消息照常进入上下文。
- 被压缩 range 只放摘要 message。
- 如果用户显式要求查看被压缩历史，前端展示原始历史；除非用户要求基于那段历史继续推理，否则不自动把原文重新塞进 prompt。

失败策略：

- 压缩失败不删除原文。
- 如果因为压缩失败导致请求仍超限，停止本轮并写 `ErrorStep`。
- 不允许反复压缩同一个 range 超过 2 次。
- 压缩摘要低质量或用户标记不可信时，该 range 下一次优先重新压缩或要求用户确认。

### 5.4 PromptAssembler

PromptAssembler 只能组装这些上下文：

- Provider/model 基础 system prompt。
- Mode role prompt。
- 当前工作目录、worktree root、git 状态、平台、日期。
- Session 历史消息。
- 用户显式输入片段：文本、附件、编辑器选区。
- 用户/项目/模式 instruction 文件。
- Skills 摘要。
- 本轮可用工具定义。

PromptAssembler 禁止做这些事：

- 禁止每轮扫描整个项目。
- 禁止每轮注入 worktree tree。
- 禁止每轮注入“关键文件摘录”。
- 禁止为了让模型“看起来知道项目”而预读大量文件。
- 禁止把右侧 editor 打开的文件自动当作全文上下文，除非用户显式引用或选择区域。

正确语义：

- 如果用户说“看看这个文件”，且输入里带 file_ref，则调用 read 读取该文件。
- 如果用户说“项目里哪里实现了 X”，模型应该用 search/list/read 工具。
- 如果用户只是普通聊天，不触发文件工具。
- 如果用户要求代码修改，先用 search/read 定位，再 edit。

### 5.5 InstructionResolver

职责：

- 加载全局 Navis Go instruction。
- 加载项目 `navis.md` 或 `.navis` 下明确配置文件。
- 加载当前 mode 的 instruction。
- 当工具读取某个文件后，再解析该文件附近的局部 instruction，并作为 tool result 的附加 reminder。

它不负责扫描所有文档。

优先级：

1. Host 安全规则最高，用户和扩展不能覆盖。
2. 当前 mode role 和 mode behavior rules。
3. 项目根 `navis.md` / `.navis` 明确配置。
4. 当前文件路径附近的局部 instruction，越靠近文件优先级越高。
5. 用户全局 instruction。
6. 扩展 skill instruction 只在 skill 被加载后生效。

冲突规则：

- 安全、权限、sandbox 规则只允许收紧，不允许放宽。
- 低优先级 instruction 不覆盖高优先级 instruction，只能补充。
- 多目录嵌套项目以 session `worktreeRoot` 为边界，不能向父级无限查找。
- 局部 instruction 只在读取相关文件后注入为 reminder，不提前扫描整棵目录。

嵌套子项目：

- `worktreeRoot` 是最高查找边界，不向外查找。
- 在 `worktreeRoot` 内，如果文件路径下存在更近的 `navis.md`、`.navis/modes/*`、`package.json`、`Cargo.toml`、`.git`，InstructionResolver 可以识别为 nested project boundary。
- 读取 nested project 内文件时，优先加载 nested project 的 instruction，再加载外层 project instruction。
- nested project instruction 只能作用于该子树，不影响 sibling 子项目。
- 用户可在项目配置中声明 `instructionRoots`，用于 monorepo 中显式指定多个子项目根。
- 如果多个 instruction 发生冲突，离文件最近的项目 instruction 优先，但仍不能覆盖 Host 安全规则和当前 mode 的硬约束。

### 5.6 Tool Catalog 与 Kernel Registry

Navis Go 不再设计一套跨 UI、菜单和 Agent 共享的应用层 ToolRegistry。可执行工具能力的权威来源是 MCP `ToolDefinition` / `MCPToolCapability`，注册事实进入 `tool/mcp/registry.rs` 的 Kernel-backed facade；Agent 只通过 Tool Catalog 获取 provider-safe 名称、Gateway function schema、风险、UI hint 和 displayKind 投影。

工具来源：

- 内建工具：以 MCP builtin server/tool 暴露，例如 filesystem、terminal、git、clipboard、memory。
- 远端 MCP 工具：server `tools/list` 发现成功后注册。
- 扩展贡献工具：必须通过扩展 MCP server 暴露真实 `ToolDefinition`，不能只在 manifest 中伪造工具。
- Agent control tool：例如 task/todo 由 `tool/agent/special.rs` 承接，仍输出同构 Gateway tool result 和 AgentTimelinePart。
- 菜单、命令面板、Host view 是 UI/extension host 能力，不要求伪装成 agent tool；需要执行工具时调用已经注册的 Tool Catalog / MCP 路径。

工具定义：

```ts
type ToolCatalogEntry = {
  mcpName: string;          // MCP canonical name
  providerName: string;     // model-visible stable name
  title: string;
  description: string;
  inputSchema: JsonSchema;
  risk: "read" | "write" | "command" | "network" | "dangerous";
  displayKind: string;
  uiHint?: ToolUiHint;
  source: "builtin" | "mcp" | "extension" | "agent_control";
};
```

这样做的好处：

- Agent tools 统一从 Kernel Registry 的工具事实投影出来，不维护第二张执行目录。
- 新扩展能增加菜单功能，也能增加 agent 可调用工具，但二者契约分开。
- UI 不需要猜工具如何展示，优先读 `uiHint`。
- 权限和 sandbox 不被绕过：执行必须进入 Agent Tool Pipeline，再进入 MCP Executor 的 Kernel Pipeline。

### 5.7 tool/agent Runtime / MCP 执行边界

职责：

- 执行工具。
- 处理权限确认。
- 把工具输入、运行中、结果、错误写成 AgentTimelineParts。
- 统一处理 MCP 工具名和模型 function name 的映射。
- 归属 `tool/agent` 的 catalog / pipeline / runtime / result / guardrail / hooks / special；`ai/agent` 只消费工具结果继续 turn loop。

模型可见工具名：

- `list`
- `read`
- `search`
- `inspect`
- `glob`
- `grep`
- `edit`
- `write`
- `bash`
- `git`（通过 `operation=status|diff|commit` 区分具体操作）
- `lsp`
- `todo`
- `task`
- `task_output`
- `task_stop`
- `webfetch`
- `websearch`
- `ask_user`

MCP 原始工具可以继续叫 `fs.read_file` / `terminal.run_command`，但模型 function name 对齐 Claude Code / opencode 的短名称，不使用点号。

工具名映射规则：

- 映射必须在注册时固定，不做运行时猜测转换。
- MCP 工具注册时必须同时提供 `modelName` 和 `originalName`。
- `modelName` 是模型可见 function name，例如 `read`。
- `originalName` 是执行时调用的 MCP 原名，例如 `fs.read_file`。
- AgentTimelinePart 同时记录 `tool = modelName` 和 `gatewayTool/originalTool = originalName`，便于调试。

注册示例：

```ts
registerMcpTool({
  modelName: "read",
  originalName: "fs.read_file",
  serverId: "builtin-fs",
  inputSchema,
  permission,
  execute,
});
```

禁止策略：

- 禁止把 `fs.read_file` 在运行时临时替换成 `read`。
- 禁止多个 MCP 工具注册同一个 `modelName`。
- 命名冲突必须在注册阶段失败，并显示具体 server/tool。

### 5.6.1 Builtin Filesystem Boundary

Navis Go 内置文件工具分三层，不能混在一起：

- 模型可见名：`read`、`list`、`search`、`inspect`、`write`、`edit`、`bash`。这些名字只为 provider function calling 服务，并对齐 Claude Code / opencode 的短工具名。
- MCP canonical name：`fs.read_file`、`fs.list_files`、`fs.search_files`、`fs.file_info`、`fs.write_file`、`fs.replace_in_file`。这是 Navis Go 内部工具注册、审计和执行的真名。
- OS filesystem：内置 MCP 工具执行时通过 Rust `std::fs` / `std::path` 访问当前操作系统文件系统。Windows、macOS、Linux 差异由 Rust 标准库和 Navis Go `PathManager` 处理，业务逻辑不写按系统分叉的执行分支。

`write` 不是一套独立 filesystem，也不是前端协议；它只是 `fs.write_file` 的模型可见名。`tool/agent` 必须在注册阶段固定 `modelName -> originalName` 映射，执行时只能按映射调用原始 MCP 工具。

对齐 Claude Code / opencode 的 `Write/Edit` 分工：`write` 可用于创建文件或完整重写已有 UTF-8 文件，普通局部修改优先使用 `edit` exact replacement。已有文件写入必须先有同路径 `read` 记录，执行前确认当前内容仍等于读取内容，再生成 diff 并作为高风险写入进入权限、审计和 AgentTimelinePart 展示。UI 层不因底层工具名派生 `Wrote` 分支，文件创建、完整覆盖和局部替换统一展示为 `Edit/Edited +n -m`。

文件变更事实写入 `SessionChange`，不是 `AgentTimelinePart` 私有 metadata，也不是 `SessionCheckpoint`。`AgentTimelinePart` 展示“Edit/Edited”这一过程，`SessionChange` 保存 `beforeContent / afterContent / diff / stats / operation`，供右侧 Review/Diff、API、Share 和 Revert 使用。`SessionCheckpoint` 只用于 Agent 会话恢复，避免 checkpoint 概念同时承担会话恢复和文件回滚两种职责。

`edit/write` 的完整 before/after 不进入模型 tool result，也不塞进聊天 UI metadata；`tool/agent` 与 `project/session` change recorder 在工具执行前后读取文件内容并写入 `session_changes`。这样既能支持真实 revert，也避免污染 prompt 和流式 UI。

文件安全边界：

- `worktreeRoot` 只能由 `tool/agent` runtime 从 `Session.worktree_root` 注入，模型、前端、扩展 UI 都不能自行指定。
- 读文件必须解析到已存在的 worktree 内路径。
- 写文件必须解析到 worktree 内目标路径；允许自动创建 worktree 内缺失父目录。
- `../`、绝对路径逃逸、canonicalize 后逃逸 worktree 的路径必须失败。
- `fs.write_file` 和 `fs.replace_in_file` 的结果必须包含 `insertions/deletions`，供 AgentTimelinePart summary 和 Diff 面板展示 `+n -m`。

### 5.7.1 PermissionManager

用户可选权限策略只有三个，和当前 Composer 菜单保持一致：

| UI 文案 | 存储值 | 语义 |
| --- | --- | --- |
| Ask for approval | `suggest` | 对写文件、编辑、删除、命令、网络、第三方 MCP 等非纯只读操作请求批准 |
| Review risk only | `auto-edit` | 低风险读写自动执行；删除、命令、网络、第三方 MCP 等高风险操作请求批准 |
| Full access | `full-auto` | 当前 session 内自动执行工具；仍记录审计轨迹和工具结果 |

内部风险分类只用于判断在上述三个策略下是否需要弹确认，不是新的 UI 权限模式：

| 级别 | 示例 | 默认策略 |
| --- | --- | --- |
| low | 读取 worktree 内文件、列目录、只读 git status | 三种策略均允许并记录 |
| medium | 写文件、编辑文件、创建目录 | `suggest` 询问；`auto-edit` / `full-auto` 自动执行 |
| high | 删除文件、执行命令、联网调用第三方 MCP、修改 git 状态 | `suggest` / `auto-edit` 询问；`full-auto` 自动执行并记录 |
| critical | 明显破坏性命令、系统目录写入、疑似密钥泄露 | 作为安全拦截或强警告处理，不新增第四种权限策略 |

权限维度：

- path allowlist：默认只允许 session `worktreeRoot`。
- path denylist：`.git` 敏感内部文件、密钥文件、系统目录。
- command denylist：删除/格式化/权限提升/后台持久进程等危险命令。
- network scope：第三方 MCP、外部 URL、provider 外请求单独授权。
- extension scope：扩展工具权限和内建工具权限隔离。

存储规则：

- 只存当前 session 的 `metadata.ui.permissionPolicy`，值只能是 `suggest`、`auto-edit`、`full-auto`。
- Tool approval 的决策选项是审批面板里的本次处理方式，不是新的 Permission policy。
- `Ask for approval` / `suggest` 触发审批时，确认面板可以提供 `Allow once`、`Allow this session`、`Allow this project`、`Deny always`；这些是本次审批建议项，不是新的三档 Permission policy。
- `Review risk only` / `auto-edit` 只有命令、删除、联网、第三方 MCP 等高风险操作才触发同一确认面板。
- `Full access` / `full-auto` 默认不触发确认面板，但仍写入 AgentTimelinePart 审计记录。
- `allow_project` 必须限定 `projectId + permission pattern`，只写入项目级信任配置，不得扩大到全局用户配置。
- `deny_always` 必须限定 `projectId + permission pattern` 或明确的全局安全规则；默认优先写入项目级拒绝规则，避免一次弹窗误伤其他项目。
- MCP 第三方工具不能绕过当前 session 的三档审批策略。
- 所有工具执行和用户决策都写入 AgentTimelinePart，用于审计和历史恢复。
- 权限等待必须先入库再通知前端：后端创建 `kind = permission`、`status = waiting_permission`、`source = permission_runtime` 的 AgentTimelinePart，`id = permission:{requestId}`，`message_id` 指向本轮 assistant shell，`call_id` 指向被阻塞的 tool call。
- 权限决策更新同一个 permission AgentTimelinePart，不创建第二条历史：`allow_once` / `allow_session` / `allow_project` 更新为 `status = completed`，`deny_always` 更新为 `status = denied`，data 中保留 `decision`、`pattern`、`riskLevel`、`args`、`reason`。
- `allow_session` 的缓存键必须是 `sessionId + permission pattern`，只存在后端 `ToolApprovalStore` 内存中；不写入 project config、用户 config 或扩展 manifest，重启应用后自然失效。
- `allow_project` / `deny_always` 的缓存键必须是 `projectId + permission pattern`，持久化到项目级审批规则 store；当前实现将规则存入 Navis Go app data 的 `approval-rules.json`，但每条规则都以规范化 `projectId` 为 scope，不写入 `navis.md`，避免污染项目指令文件。前端只展示后端返回的建议项，不能自行扩权生成 pattern。

Permission 状态机：

```text
ToolStep.running
-> ToolStep.waiting_permission
-> PermissionStep.waiting_permission
-> user allow_once / allow_session / allow_project / deny_always
-> PermissionStep.completed | PermissionStep.denied
-> ToolStep.running | ToolStep.error
-> ToolStep.completed | ToolStep.error
```

多权限工具：

- 同一 tool call 需要多个权限时，PermissionManager 合并为一个 `PermissionStep`，列出所有 permission items。
- 用户可整体允许或整体拒绝；不做部分允许，避免工具进入半执行状态。
- 工具执行中途发现新增权限时，原 `ToolStep` 进入 `waiting_permission`，不创建新的 tool call。

恢复规则：

- 用户点击 `Allow once` 后，前端发送 `ui_respond_tool_approval({ requestId, decision: "allow_once" })`。
- 用户点击 `Allow this session` 后，前端发送 `ui_respond_tool_approval({ requestId, decision: "allow_session" })`，后端只在当前 session 内缓存同一 permission pattern。
- 用户点击 `Allow this project` 后，前端发送 `ui_respond_tool_approval({ requestId, decision: "allow_project" })`，后端写入项目级 trust store。
- 用户点击 `Deny always` 后，前端发送 `ui_respond_tool_approval({ requestId, decision: "deny_always" })`，后端写入项目级 deny rule。
- 后端继续同一个 tool call，并更新原 `ToolStep`。
- 不创建新的异步流，不复制 tool call。
- 被拒绝的破坏性工具结果必须写入模型上下文，模型不能假装执行成功。

### 5.8 GatewayRequestBuilder

职责：

- 根据调用方已解析出的 `providerId + modelId` 派生 `provider/model` 路由键；Gateway 只接受该明确路由键，不根据短 `modelId` 猜 Provider。
- 合并 system/messages/tools。
- 应用 provider 协议差异。
- 设置 provider-level timeout、retry、headers。
- 输出统一请求对象。

Settings UI 保留 `requestTimeoutSecs`，语义是 provider-level 请求超时；默认 300 秒。流式读取额外使用 `requestTimeoutSecs + 120s` 的后端 idle 超时，即超过该时长没有新 chunk 才终止，避免 provider stream 静默挂起但不限制长回答总时长。

### 5.9 GatewayStreamRuntime

职责：

- 读取 SSE / OpenAI compatible stream / 非流式响应。
- 处理取消。
- 处理网络错误和 JSON 解码错误。
- 只输出原始 provider event，不写 UI。
- Gateway 只在 provider 主动报错且尚未产生任何 provider 内容时自动重试：请求发送失败、HTTP retryable 状态、流读取失败、流式 JSON 解码失败都可以触发；一旦已经向 Agent 输出过 provider chunk，就不重放请求，避免重复文本或重复 tool_call。
- 每次自动重试输出内部 `gatewayRetry` stream event，由 `ui_stream_session_message` 转成 `kind = error`、`status = retrying`、`source = gateway_retry` 的 AgentTimelinePart，标题为 `Retrying n/max`，完整错误写入 `detail`。

如果出现 `error decoding response body`：

- GatewayStreamRuntime 要把错误转换成 `stream_error`。
- AgentTimelinePartRecorder 要落盘错误 AgentTimelinePart。
- 前端历史不能因为流失败而丢失已有 user message 或已生成 AgentTimelinePart。

### 5.10 StreamEventNormalizer

职责：

- 把 provider 事件转成统一事件：

```ts
type NormalizedStreamEvent =
  | { type: "text_delta"; partId: string; delta: string }
  | { type: "reasoning_start"; partId: string; title?: string }
  | { type: "reasoning_delta"; partId: string; delta: string }
  | { type: "tool_call_start"; callId: string; toolId: string; inputPreview?: string }
  | { type: "tool_call_ready"; callId: string; toolId: string; input: unknown }
  | { type: "tool_step_completed"; callId: string; output: ToolResult }
  | { type: "finish"; reason: string; usage?: TokenUsage }
  | { type: "error"; error: AgentError };
```

Normalizer 不能知道 UI 布局，也不能执行工具。

### 5.11 AgentTimelinePartRecorder

职责：

- 参考 opencode 的 `message.part.updated` / `message.part.delta` 分层：状态变化发布完整 AgentTimelinePart，文本高频输出发布字段 delta。参考 Claude Code 的 `handleMessageFromStream()` 分层：provider stream event 只驱动临时 UI 状态，最终消息到达后再进入 canonical transcript。
- 将 normalized events 转成 `AgentTimelinePart`。
- 负责 append/update/delta。
- 工具状态、权限、重试、错误和 finalizer 使用完整快照：先写入 `agent_timeline_parts`，再读取数据库确认后的 canonical `AgentTimelinePart`，最后通过 Tauri Channel 发布 `type = AgentTimelinePart`。写入 `turn_finalizer` 前必须先更新同轮 `turn_prelude` 为 `completed`，让运行态有明确生命周期收口，而不是依赖前端隐藏或刷新后 stale-abort。
- 高频文本追加使用 `type = AgentTimelinePartDelta`：text AgentTimelinePart 先写入 running 快照，provider token 只通过 delta 追加到同一 `partId.text`，流结束后再写入 completed 完整快照。
- 取消、超时或错误终止时，如果本轮已经产生 assistant 文本，必须先把当前累计文本写入 aborted/error text AgentTimelinePart 完整快照，再写终止 AgentTimelinePart；这吸收 Claude Code “interrupted mid-stream 不丢 partial assistant response”的行为要求。
- Tool Catalog 必须为内建工具写入稳定 `metadata.displayKind`：当前内建集合为 `read/list/glob/grep/search/inspect/edit/write-as-edit/bash/git/lsp/todo/task/task_output/task_stop/webfetch/websearch/permission/error`。`skill`、`mcp_resource`、`browser` 只能在真实 Skill/MCP 扩展能力被注册和发现后进入同一 renderer 体系，不能作为宿主假内建工具占位。前端对话区优先按原始 `metadata.displayKind` 选择 renderer 和 glyph，再按 Timeline 归一化 kind、MCP `tool`、provider-safe `gatewayTool`、`rendererHint.renderer/detailView` 匹配；不能靠 title、summary 或 provider-safe tool name 猜测内建工具语义。`write` / `fs.write_file` 只能进入 `write-as-edit` 或 `edit` 语义，展示为 `Edit/Edited`，不能出现 `Wrote` UI 分支；扩展工具没有专属 display kind 时使用 `other` 并通过 `rendererHint` 接入专属 renderer。
- `glob` / `grep` / `webfetch` / `websearch` 是真实内建 MCP tool，不是 `search` 的 UI 别名：`glob -> fs.glob`，`grep -> fs.grep`，`webfetch -> web.fetch`，`websearch -> web.search`。Web 工具只返回真实 HTTP 请求结果或真实网络错误；浏览器控制、电脑控制等需要可交互外部系统的能力必须由真实 MCP 扩展通过 `tools/list` 发现后接入，不能在宿主里注册一个假 `browser` 成功工具占位。
- 同一 `callId` 的 running/completed/error 更新必须保持同一数据库行；`createdAt`、`partId`、`sequence` 以首次写入的事实行为准，后续更新只改变状态、详情、`updatedAt` 和扩展数据。进入 UI 后，前端只按 `partId` 合并完整快照和 delta，不允许用 `callId` 兜底合并不同 `partId` 的步骤。
- 保证刷新后历史可恢复。
- Navis Go 桌面端不使用 HTTP SSE 作为主传输；opencode 的 SSE 订阅思想只作为“事实入库、视图订阅”的参考，具体传输由 Tauri Channel 承担。

核心数据结构：

```ts
type AgentTimelinePart =
  | TextStep
  | ReasoningStep
  | ToolStep
  | PermissionStep
  | DiffStep
  | TerminalStep
  | ErrorStep
  | SummaryStep;
```

必须保留：

- `partId`
- `turnId`
- `messageId`
- `status`
- `createdAt`
- `updatedAt`
- `startedAt` / `completedAt` / `durationMs`（工具类 AgentTimelinePart 必填；running 时至少有 `startedAt`，完成或失败后必须有 `completedAt` 与 `durationMs`）
- `input` / `progress` / `output` / `metadata`（工具类 AgentTimelinePart 使用 Claude Code 风格三段展示契约；扩展工具也必须走该字段集）
- `title`
- `summary`
- `detail`
- `source`

协议版本：

```ts
type AgentTimelinePartEnvelope = {
  schemaVersion: 1;
  minClientVersion?: string;
  step: AgentTimelinePart | UnknownAgentTimelinePart;
};
```

版本演进规则：

- 新 AgentTimelinePart 类型必须增加 fallback 展示字段：`title`、`summary`、`detail`。
- 旧客户端遇到未知 `kind` 时显示 `Unsupported workflow step`，并展示 fallback 文本。
- 数据库 `agent_timeline_parts.data` 必须保留 JSON 扩展字段，不把每种 AgentTimelinePart 的字段全部摊平成固定列。
- `AgentTimelinePartRecorder` 必须能忽略未知字段，不能因为新字段导致历史加载失败。
- 破坏性协议变更必须提高根协议 / 根 Schema 版本；Navis Go 未发布阶段启动时发现不匹配必须提示重建或手动更新。

## 6. 前端交互流程

### 6.1 发送前

Composer 显示：

- 当前 mode。
- 当前 default model 或用户本轮选择的 model。
- 输入文本。
- 附件 chips。
- 编辑器选区 chips。
- 可用快捷动作。

模型选择规则：

- 默认模型只和 `model.id` 关联。
- display name、context window、protocol、compression 配置变更，不得改变 default model。
- 如果 default model 不存在，显示 `Choose`，并在发送时阻止提交。
- 新建任务、新建会话、多功能面板都从同一个 `DefaultModelStore` 读取默认模型。

### 6.2 点击发送

前端立即做这些事：

- 生成 `turnId`。
- 输入框进入 submitting。
- 禁用重复发送。
- 显示停止按钮。
- 乐观追加 user bubble。
- 在 user bubble 下创建空 assistant turn 容器。
- 等待后端第一条 AgentTimelinePart：`reasoning` + `source = "turn_prelude"`，展示为轻量 thinking 状态，例如 `Thinking`。该文本不得拼接用户原始指令，不得写成“准备本轮任务”这类模板化工作项；前端只能在本轮尚无真实 assistant text 可显示且 prelude 仍处于 active 状态时显示，不能把它长期保留成历史步骤。
- 发起 `ui_stream_session_message`。

如果后端在入队前失败：

- user bubble 保留。
- assistant turn 显示 error AgentTimelinePart。
- 输入框恢复可编辑。
- 显示 Retry。

### 6.3 流式回复展示

回复区域采用单条纵向时间线：

```text
User bubble

Assistant turn
  Thinking
  Read src/main.ts
  Search "Default model"
  Edited src/components/Settings.tsx +12 -4
  Ran cargo check
  Assistant text streaming...
  Footer: Code · gpt-4.1 · 18s · 12.4k tokens
```

展示规则：

- `Thinking` 是 reasoning AgentTimelinePart 或模型未输出前的占位状态。
- `source = "turn_prelude"` 的 reasoning AgentTimelinePart 单独渲染为开工说明，不计入工具统计，也不显示 `Done` 徽标；后端写入 `turn_finalizer` 前必须把它更新为 `completed`，前端历史中 inactive prelude 不作为普通步骤展示。
- `source = "turn_finalizer"` 的 summary AgentTimelinePart 单独渲染为收尾状态，例如 `Finished response · 2 tool calls · 4096 tokens`，不计入工具统计，也不显示为 `Compacted context`。
- 文本 delta 进入 assistant text AgentTimelinePart，使用 markdown streaming 渲染。
- 同一 turn 内只要存在工具 / 权限 / 错误等 action AgentTimelinePart，普通最终 assistant text 必须等 `source = "turn_finalizer"` 的 summary AgentTimelinePart 到达后再展示；若仍存在 running / retrying / waiting_permission，则最终 assistant text 和 `turn_finalizer` 都暂不展示。工具调用前的 assistant text（`source = "gateway_tool_prelude"`）仍按 sequence 展示；普通最终 assistant text（`source = "gateway"`）在存在 action step 时进入 final phase，统一显示在所有 action steps 后、`turn_finalizer` 前。这样保证命令、编辑、审批等真实动作完成前，UI 不会先显示“成功/完成”的最终回复，也避免后端 final messages 覆盖时出现顺序跳动。
- 工具调用开始时插入 tool AgentTimelinePart。
- 工具 running 时显示 spinner 和简短 title。
- 工具 completed 时在同一行显示紧凑 meta，默认折叠 detail；工具行固定为 `icon / title / expand chevron / result meta / duration / status`，read/list/glob/grep/search 的 `Read n lines`、`Listed n entries`、`Matched n files`、`Found n results` 等 meta 必须位于 expand chevron 后、duration 前，不另起一行撑高 timeline。写入/编辑类工具如果结果包含 `insertions/deletions`，标题追加 `+n -m`。
- 同一 turn 仍在运行时，已完成的 action AgentTimelinePart 默认聚合成一行 `Completed read/list/search/edit/command...` 摘要，用户展开后才显示历史完成步骤；当前 `running` / `retrying` / `waiting_permission` / `error` / `denied` / `aborted` 步骤必须直接显示，避免界面闷头干或被历史工具流水账占满。该规则参考 Claude Code 的 collapsed read/search group、opencode 的 tool count summary 和 Hermes 的 event-driven ToolTrail，但 Navis Go 统一由 `AgentTimelinePartsView` 编排，不要求每个工具 renderer 自己做聚合，也不能用本地 fake 状态补齐后端没有发送的进度。
- 工具 error 时红色状态，不吞掉最终文本。
- 权限等待时插入 permission AgentTimelinePart，提供 Allow once、Allow this session、Allow this project、Deny always。
- 用户拒绝权限后，tool AgentTimelinePart 进入 denied/error，agent loop 可继续解释或停止。
- 取消时所有 running AgentTimelinePart 标记 aborted，assistant footer 显示 interrupted。
- `turn_finalizer` summary 必须包含整轮耗时，例如 `Finished response · 5 tool calls · 4735 tokens · 12s`。
- 运行中前端收到 AgentTimelinePart 但还没收到最终 messages 列表时，必须先用 `turnId` 插入 pending user message，再插入 assistant shell，保证用户指令永远显示在该轮处理过程上方。

### 6.4 Tool Part 展示规范

| 工具类型 | 紧凑标题 | 默认 detail | 右侧面板行为 |
| --- | --- | --- | --- |
| read | `Read {path}`，`>` 后、耗时前显示 `Read n lines` | 默认折叠；展开后在公共灰色详情面板预览读取内容、范围和截断说明，面板内部滚动 | 打开文件到右侧 editor |
| list | `List {path}`，`>` 后、耗时前显示 `Listed n entries` | 默认折叠；展开后在公共灰色详情面板预览文件/目录列表，面板内部滚动 | 可跳转 worktree tree |
| glob/search/grep/websearch | `Search "{query}"` 或 `Matched n files`，`>` 后、耗时前显示结果数 | 默认折叠；展开后在公共灰色详情面板预览命中列表，面板内部滚动 | 点击命中打开右侧 editor / resource |
| edit | `Edited {path} +x -y` | diff | 右侧打开 diff/editor |
| write | 不单独显示 Wrote，归并为 `Edited {path}` | diff 或 created file | 右侧打开文件 |
| bash/git | `Ran {command}` | 默认折叠；展开后在公共灰色详情面板显示 stdout/stderr/exit/status，terminal 状态来自真实 AgentTimelinePart.status | 右侧 terminal / git detail 只能由 renderer 明确声明 |
| lsp/todo/skill/webfetch/mcp_resource/browser | 读 `uiHint.title` 或内建 compact label | 默认折叠；展开后在公共灰色详情面板显示 structured output | 根据 `rendererHint.detailView` 或内建 renderer |
| task/task_output/task_stop | `Task {task}` | 子任务摘要、活动、工具次数、token、sidechainSessionId | 打开 sidechain session 由 renderer 明确声明 |
| question | `Needs input` | 选项和说明 | 原地展示确认 UI |
| mcp | 读 `uiHint.compactTitle` | structured output | 根据 `uiHint.detailView` |

这里明确：Navis Go UI 不使用 `Wrote` 作为独立语义。创建、覆盖、局部修改都归入文件变更，展示为 `Edit/Edited`。

### 6.5 右侧面板联动

用户之前明确要求 editor 打开的文件在右侧面板。这里作为交互契约：

- 点击 read/inspect/edit 相关 AgentTimelinePart 的文件路径，右侧面板打开 `File` 并定位到该 worktree 内文件。
- 点击 edit AgentTimelinePart 的展开箭头只展开当前 timeline diff；如需右侧完整文件视图，点击路径打开 `File`。后续若增加独立 diff 右侧面板，必须由工具 renderer 明确声明，不再把 edit 泛化成自动打开 diff。
- 点击 command AgentTimelinePart，右侧面板打开 terminal output detail，不抢占聊天滚动。
- 点击 mcp structured AgentTimelinePart，右侧面板按 `detailView` 打开表格、JSON、文本或自定义扩展 view。
- 右侧面板不能覆盖 Composer，也不能把多功能面板挤出窗口。

### 6.6 滚动和自适应

聊天区：

- 主消息列表独立滚动。
- Composer 固定在底部。
- 多功能面板在窗口高度不足时向上压缩，内部滚动，不允许溢出窗口。
- 当用户在底部附近时自动跟随最新 token。
- 当用户手动向上滚动时停止自动贴底，并显示 Jump to latest。

Settings：

- Dialog 外层高度为 viewport bounded。
- 左侧菜单固定。
- 右侧内容区域必须 `overflow-y: auto`。
- 模型列表和 Default model 位于同一个右侧滚动上下文。
- Add model 后按钮不能被窗口裁掉。
- 下拉框 popup 必须受 viewport 约束，不能把页面撑出窗口。

### 6.7 错误恢复

网络流失败：

- 保留 user bubble。
- 保留已收到的 assistant text/tool AgentTimelinePart。
- 添加 error AgentTimelinePart：`Stream failed: ...`。
- footer 显示 failed。
- 提供 Retry from last user message。
- Retry 复用同一 user message，创建新的 assistant attempt，旧 attempt 保留或折叠。
- 默认最多自动重试 0 次，用户手动 Retry 最多 3 次。
- Retry 不重复执行已成功完成的破坏性工具。
- Retry 默认不复用只读工具结果；用户或系统策略启用后才允许复用。
- 只读工具结果复用必须校验 worktree revision、文件 mtime/size/hash、tool input hash、model-visible path。
- 复用有效期默认只在当前 assistant attempt retry 链内生效；切换 session、切换 worktreeRoot、检测到文件变更后失效。
- 被复用的结果必须写成 `ToolStep.status = "reused"`，并显示来源 attempt。
- 用户可在 Retry 面板关闭 `Reuse safe read results`。
- 如果上一次失败发生在写文件或命令执行之后，Retry 前必须要求用户确认。
- 每个 assistant attempt 都有独立 `attemptId`，不能把新输出追加到旧失败 attempt。

工具失败：

- tool AgentTimelinePart 显示 error。
- 如果错误可恢复，agent 可以继续解释或选择其他工具。
- 如果错误不可恢复，assistant message 结束为 error。

历史恢复：

- 页面刷新后从 SessionMessageStore 加载 messages + AgentTimelineParts。
- 不依赖前端内存里的临时 trace。
- 不允许出现“界面之前的记录没了”。

## 7. 菜单和扩展设计

当前结构问题是：菜单契约统一成 `MenuRegistration` 后，行为解释器仍然按区域拆分。Navis Go 应该把行为解释器也统一。

不要把菜单、Agent 工具、右侧面板强行塞进同一个 `execute(ctx: ActionContext)`。Navis Go 的灵活扩展性来自“扩展可以实现多个清晰能力接口”，不是来自一个巨大泛型 Action。

当前落地约束：所有已落地 surface 的内建菜单 command 必须进入 `menu-command-coverage` 覆盖表，并由 `npm run test:menus` 对照后端 `builtin_menus()` 自动校验。这个约束保证可见菜单都有真实动作；后续抽统一解释器时也必须保留同等或更强的覆盖校验。

建议结构：

```ts
type MenuContribution = {
  id: string;
  title: string;
  locations: MenuLocation[];
  action: HostAction;
};

type ViewContribution = {
  viewId: string;
  title: string;
  renderer: "host:panel";
  placement: "rightWorkspace" | "chatAside" | "bottomDrawer" | "settingsSection";
  config?: Record<string, unknown>;
};

type ExtensionContribution =
  | MenuContribution
  | ViewContribution
  | CommandContribution
  | InstructionProvider
  | McpServerContribution;
```

基础原则：

- 菜单入口是声明式 `MenuContribution`，执行 `HostAction`，不直接持有业务执行器。
- Agent 可调用工具必须通过 MCP server 贡献真实 `ToolDefinition`，进入 Kernel-backed MCP Tool Registry 后再由 Tool Catalog 暴露给模型。
- Host panel/view 是 UI 域 renderer，不通过工具执行器渲染，也不进入 Kernel。
- Command Palette 可以触发 `HostAction` 或打开 Host view；需要执行工具时走 Tool Catalog / Agent Tool Pipeline / MCP Executor。
- 内建 read/search/edit/bash 等 agent tools 必须优先作为 MCP builtin tool 注册，不需要先注册成菜单 action。
- 菜单如果要读文件或编辑文件，应调用对应后端 IPC/host action；若进入 Agent 工具语义，必须走 Tool Catalog 和权限检查，不能创建 adapter 旁路。
- 扩展可以同时贡献 view、menu、command、MCP server，但三者契约分开；MCP server 才是 agent tool 的进入方式。
- 不再有 composer-menu、session-menu、gateway-menu、right-workspace-menu 四套行为解释器；各入口只负责上下文和 HostAction 分发。

这样保留扩展扩展能力，同时避免 `ActionContext` 变成丧失类型安全的巨大联合类型。

## 8. 配置设计

Gateway Settings 只保留用户真正需要配置的项：

- Provider name。
- Base URL。
- API key。
- Protocol。
- Models。
- Default model。
- Request timeout。
- Max retries。

不在 UI 暴露：

- provider 内部解码策略。
- SSE 总时长上限。

Context window 默认：

- 新增模型默认上下文为 `256k`。
- 展示值必须来自模型配置，不写死。
- 如果用户把 128k 改成 256k，所有展示位置从同一份 model config 读取。
- Default model 下拉只使用 model id，不依赖 context window。

Default model 降级：

- 如果 default model 被删除，Settings 显示 `Choose`，但不静默选择其他模型。
- 如果当前会话已有 `session.model`，继续使用会话模型，不受全局 default 删除影响。
- 如果新建会话时没有 default model，阻止发送并打开模型选择。
- 如果 provider 返回模型不可用，Gateway 返回明确错误，前端提供 `Choose another model`。
- 不允许根据 display name、context window 或列表顺序自动替换 default model。

## 9. 前端视觉效果建议

Navis Go 的回复展示应参考 Codex App 的紧凑工程感，而不是做大型卡片堆叠。

视觉原则：

- 用户消息清晰，assistant timeline 紧凑。
- 聊天主轨道固定宽度并与用户灰色气泡右边界对齐；工具行、detail、diff、terminal 输出不得因为展开、滚动条或长命令改变主轨道宽度。
- 工具节点一行优先：icon、title、expand chevron、result meta、duration、status。Duration、result meta 和 status 即使为空也必须保留占位列，expand chevron 固定在 result meta 前、duration 前两列，不能因 status / duration 缺失、展开 detail 或内部滚动条出现而横向漂移。read/list/glob/grep/search 的结果统计显示为行内 meta，不在标题下方另起一行；无 meta 时保留稳定列。宿主内建 `read/list/glob/grep/search/inspect/edit/write-as-edit/bash/git/lsp/todo/task/task_output/task_stop/webfetch/websearch/permission/error` 必须使用可区分 glyph 或明确复用同族 glyph；扩展注册的 `skill/mcp_resource/browser` 等 renderer 也必须提供明确 glyph 或复用同族 glyph。shell/terminal 使用终端窗口 glyph，read 使用四点资源 glyph，edit/write-as-edit 使用铅笔 glyph，多命令/收尾 summary 使用多行命令列表 glyph。edit 默认折叠，只在用户展开后显示 diff 与行号，diff 的横向溢出只在详情块内部滚动。Shell / terminal 展开详情块右下角显示由真实 `AgentTimelinePart.status` 映射出的 Running / Success / Failed 状态。
- 运行中如果已完成工具超过当前可读性阈值，不继续铺开历史步骤；显示 `Completed ...` 紧凑摘要并保留展开入口。当前正在执行和失败/中断的步骤永远可见，完成后的整轮 summary 再统一折叠为末尾摘要。
- 工具展示入口使用 `ToolRendererCatalog`；内建工具和扩展工具都注册 renderer，未知工具走 generic renderer，不允许主 Timeline 巨型 switch 解释所有工具。Catalog 匹配必须支持原始 `metadata.displayKind`、Timeline 归一化 kind、MCP `tool`、provider-safe `gatewayTool`、`rendererHint.renderer` 和 `rendererHint.detailView`，让浏览器控制、电脑控制、数据库查询等扩展工具可以在同一 renderer 下选择不同 detail view。
- 详情折叠，点击后在原地或右侧面板展示。
- Diff、terminal、large output 默认不挤占聊天区；read/list/search/inspect/terminal 的原地展开必须复用同一个公共灰色详情面板，并在面板内部滚动，不能撑高整个对话流。Diff 保持独立浅米色 diff 面板和行号语义。
- 错误节点明显但不破坏整轮阅读。
- 最终 assistant 文本是主内容，工具过程是可审计过程；`turn_finalizer` 只作为按 sequence 渲染的末尾 summary AgentTimelinePart，不能被前端抽离到正文之前。

状态样式：

| 状态 | 展示 |
| --- | --- |
| pending | 低对比文字 |
| running | spinner + running title |
| completed | check/neutral icon + compact title |
| error | error color + short reason |
| denied | permission denied 标识 |
| aborted | muted interrupted |
| compacted | `Compacted context` separator |

## 10. 需要修改的现有设计

这份设计确认后，Navis Go 相关设计文档应同步修改：

- `design/16-agent.md`：删除“worktree tree 和关键文件摘录每轮注入”的描述。
- `design/22-ui-framework.md`：删除主对话通过 Agent Turn Context 注入 worktree tree 的描述，改为 AgentTimelineParts + tool-driven context。
- `design/12-gateway.md`：Settings 保留 provider-level requestTimeoutSecs，默认 300 秒；流式读取 idle 超时为配置 + 120 秒。
- `design/03-config.md`：用户级配置保留 requestTimeoutSecs，语义同 Gateway provider-level timeout。
- `design/22-ui-framework.md` / `design/07-extension.md`：补充菜单入口、扩展菜单和 action 执行边界，避免形成多套菜单解释器。

## 11. 后续实现顺序

建议不要零散修 UI bug，而是按边界重构：

1. 先改设计文档，移除 worktree snapshot 注入。
2. 建立 `AgentTimelinePart` 单一前后端协议。
3. 重构 AgentTurnRunner，禁止裸 Gateway chat 作为主对话路径。
4. 重构 PromptAssembler，只保留最小上下文。
5. 重构 Tool Catalog / HostAction 执行器，消除多套菜单解释器。
6. 重构 Gateway Settings，把 default model 绑定到 model id。
7. 修复 Settings 和 Composer 面板自适应滚动。
8. 按 AgentTimelineParts 重做前端回复展示。
9. 最后接入 edit/bash/git 等高风险工具，并用 permission/sandbox 闭环。

这个顺序的原因是：Settings 下拉、Default model、上下文显示、回复记录丢失、工具标题 Wrote/Edit、菜单扩展扩展，本质都依赖统一数据契约。只在组件局部打补丁，会继续制造互相影响的 bug。

## 12. 当前落地状态

截至 2026-06-04，本轮已落地：

- Agent Turn Context 已移除每轮 worktree snapshot/tree/important file excerpts 注入，改为 tool-driven context。
- 后端主对话会先落盘 user message，再创建 assistant shell；`AgentTimelinePart.messageId` 是必填协议字段，来自 assistant message id。
- `agent_timeline_parts.message_id` 已收紧为非空存储契约；旧的“AgentTimelinePart 先无 message_id、最后 attach”的路径已移除。
- text/tool/error/abort AgentTimelineParts 会携带 `messageId` 和 `turnId` 入库并通过 stream 推送。
- 前端聊天区不再使用 `executionParts`、`local-assistant-*`、delta fallback；收到缺少 `messageId` 的 AgentTimelinePart 会作为协议错误暴露。
- `StreamHandler` 不再用空字符串兜底 message id；未先调用 `start_stream(message_id)` 就发 chunk 会作为后端协议错误返回。
- Settings / Composer 的 Default model 只按 `model.id` 绑定；权限策略只保留三档 `suggest`、`auto-edit`、`full-auto`。
- Settings 右侧内容和 Composer 菜单增加 viewport 内滚动约束；`requestTimeoutSecs` 作为 provider-level timeout 保留，默认 300 秒，流式读取 idle 超时为配置 + 120 秒。

仍未落地：

- 完整 `CompactionManager` 自动/手动触发、summary message/AgentTimelinePart 生成和 summary revision。
- `CompactedRange` 已有持久化表和 Context Assembler prompt 替换；尚未接入主对话 AgentTurnRunner 的自动生成流程。
- 完整 `InstructionResolver`、nested project instruction 和 `instructionRoots`。
- turn-level retry ledger、工具结果复用校验和 mutating tool 幂等保护。
- 菜单/扩展系统归一为 `MenuContribution` / `ViewContribution` / `CommandContribution` / `McpServerContribution`，Agent 工具只通过 MCP ToolDefinition 进入 Tool Catalog。

