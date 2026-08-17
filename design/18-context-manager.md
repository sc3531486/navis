# 18 - Context Manager 上下文管理 详细设计

> 模块编号：18 | 层级：AI 核心层
> 依赖：01-Logger, 03-Config, 04-Storage, 08-Session, 19-Skills
> 被依赖：16-Agent

---

## 一、模块概述

### 1.1 定位

Context Manager 负责组装发送给 LLM 的完整上下文，包括系统提示词、角色设定、项目配置、历史消息、RAG 检索结果、可用工具列表，并管理 Token 窗口裁剪和上下文压缩。

### 1.2 职责边界

```
负责：
├── 上下文组装（拼装所有上下文来源）
├── Token 计数（估算上下文 Token 数）
├── 上下文裁剪（超出窗口时裁剪历史消息）
├── 上下文压缩（自动阈值触发 + 用户手动触发，LLM 摘要压缩）
├── 工具列表注入（根据 Skills 白名单过滤）
└── 项目配置注入（navis.md）

不负责：
├── Agent 决策 → Agent
├── 模型调用 → Gateway
├── RAG 检索执行 → RAG（Context Manager 只消费结果）
├── 消息存储 → Storage
├── 跨会话上下文拉取 → 禁止（见 §5 约束说明）
└── Task Sidechain 上下文组装 → Task Sidechain（见 17-task-sidechain.md §6）
```

---

## 二、架构设计

```
context/
├── mod.rs              # 模块入口
├── assembler.rs        # 上下文组装器入口：公开 API、assemble 编排、压缩流程外壳
├── assembler/
│   ├── runtime.rs              # 同步 API 的 async future 驱动边界
│   ├── compression_boundary.rs # 压缩安全边界选择，保护 system/head/tail 和 tool call/result 原子段
│   ├── compression_render.rs   # Storage/Gateway 消息渲染为摘要输入文本
│   ├── compression_template.rs # 项目级/用户级/内建压缩模板加载
│   └── summary.rs              # Gateway-backed LLM 摘要生成
├── token_counter.rs    # Token 计数器
├── trimmer.rs          # 上下文裁剪
└── model_adapter.rs    # 模型上下文适配（格式转换、tokenizer 切换）
```

`assembler.rs` 保持 `Assembler` 对外路径稳定，子模块只承接可独立测试的纯逻辑和 Gateway 摘要调用。后续继续拆分时，历史加载、memory snapshot、Storage Message → Gateway Message 投影和 token budget 应继续进入 `assembler/` 子模块，不把新职责塞回入口文件。

---

## 三、数据模型

```rust
struct AssembledContext {
    system_prompt: String,           // 系统提示词
    role_prompt: Option<String>,     // 角色设定（Skills/Role）
    project_context: Option<String>, // 项目配置（navis.md）
    rag_context: Option<String>,     // RAG 检索结果
    messages: Vec<Message>,           // Session 历史消息事实
    tools: Vec<ToolDefinition>,      // 可用工具列表
    total_tokens: usize,             // 估算总 Token 数
}

struct ContextConfig {
    max_context_tokens: usize,       // 最大上下文 Token（默认 128000）
    max_history_messages: usize,     // 最大历史消息数（默认 50）
    auto_compress_threshold: f32,    // 自动压缩触发阈值（默认 0.8，对标 Claude Code autoCompact）
    compress_keep_recent: usize,     // 压缩时保留最近 N 条消息（默认 10）
    rag_enabled: bool,
    rag_top_k: usize,
    project_config_enabled: bool,
}

// 上下文格式类型（不同模型对上下文结构有不同要求）
enum ContextFormat {
    SystemMessage,       // system prompt 作为 role:"system" 消息（OpenAI/DeepSeek/MiMo）
    SystemField,         // system prompt 作为顶层 system 字段（Anthropic Claude）
    NoSystemPrompt,      // 不支持 system prompt，需合并到 user 消息（部分开源模型）
}

// 模型上下文配置（每个模型的上下文相关参数）
struct ModelContextProfile {
    model_id: String,
    context_format: ContextFormat,           // 上下文格式
    context_window: usize,                   // 上下文窗口大小（Token）
    max_output_tokens: usize,                // 最大输出 Token
    tokenizer: TokenizerType,                // Tokenizer 类型
    supports_multimodal: bool,               // 是否支持多模态内容
    system_prompt_max_tokens: Option<usize>, // system prompt 最大 Token 限制
}

enum TokenizerType {
    CL100K,         // OpenAI cl100k_base（GPT-3.5/4）
    O200K,          // OpenAI o200k_base（GPT-4o）
    Claude,         // Anthropic Claude tokenizer
    Llama,          // Llama 系列（开源模型通用近似）
    Custom(String), // 自定义 tokenizer 标识
}
```

---

## 四、接口定义

### 4.1 Rust API

```rust
ContextManager::init(config: ContextConfig) -> Result<ContextManager>

// 组装上下文：异步运行链优先使用 async 入口；同步入口只作为阻塞边界外壳
ContextManager::assemble_async(&self, session_id: &str, user_message: &str) -> Future<Result<AssembledContext>>
ContextManager::assemble(&self, session_id: &str, user_message: &str) -> Result<AssembledContext>

// Token 计数
ContextManager::count_tokens(&self, text: &str) -> usize
ContextManager::estimate_request_tokens(&self, context: &AssembledContext) -> usize

// 手动压缩（对标 Claude Code /compact 命令）
ContextManager::compress_async(
    &self,
    session_id: &str,
    messages: &[Message],
    options: Option<CompressOptions>,
) -> Future<Result<CompressResult>>

ContextManager::compress(
    &self,
    session_id: &str,
    messages: &[Message],
    options: Option<CompressOptions>,
) -> Result<CompressResult>

struct CompressOptions {
    from_message_id: Option<String>,    // 从指定消息 ID 开始压缩（None = 保留最近 N 条，压缩更早的）
    keep_recent_count: Option<usize>,   // 保留最近 N 条原始消息（默认 10）
    focus_instruction: Option<String>,  // 用户指定保留重点（对标 Claude Code /compact focus on xxx）
}

struct CompressResult {
    before_token_count: usize,   // 压缩前 Token 数
    after_token_count: usize,    // 压缩后 Token 数
    compressed_range: (String, String),  // 被压缩的消息 ID 范围（start_id, end_id）
    summary_message_id: String,  // 压缩生成的摘要消息 ID
    id_mapping: HashMap<String, String>,  // 原始 message_id → summary_message_id（供 Checkpoint 恢复）
    summary: String,             // 压缩生成的摘要内容
}

// 模型适配
ContextManager::get_model_profile(&self, model_id: &str) -> ModelContextProfile
ContextManager::format_for_model(&self, context: &AssembledContext, model_id: &str) -> Result<FormattedContext>

// 格式化后的上下文（按目标模型要求格式化完毕，可直接发给 Gateway）
struct FormattedContext {
    system: Option<String>,           // system prompt（按模型要求放置）
    messages: Vec<ProviderChatMessage>,       // Provider 边界消息列表，由 Session Message 转换而来
    estimated_tokens: usize,          // 按目标模型 tokenizer 估算的 Token 数
    format_used: ContextFormat,       // 使用的格式
}
```

同步/异步边界规则：

- `assemble_async()` / `compress_async()` 是 Agent/Gateway 运行链默认入口，LLM 摘要压缩通过 Gateway 原生 `await`。
- `assemble()` / `compress()` 只用于同步 IPC 或测试边界，内部集中驱动 async future，不允许业务逻辑中散落 `Handle::current().block_on(...)`。
- Context Manager 不是 Kernel 原语；它消费 Storage/Gateway/EventBus。执行链如果需要审批、重试、审计，应由上层 Agent/Tool Pipeline 承接。

### 4.2 IPC 命令

```typescript
context.assemble(sessionId: string, userMessage: string): Promise<AssembledContext>
context.countTokens(text: string): Promise<number>
context.getConfig(): Promise<ContextConfig>
context.setConfig(config: Partial<ContextConfig>): Promise<void>

// 手动上下文压缩（对标 Claude Code /compact 命令）
context.compress(sessionId: string, options?: {
  fromMessageId?: string;     // 从指定消息 ID 开始压缩（不传则保留最近 N 条）
  keepRecentCount?: number;   // 保留最近 N 条原始消息（默认 10）
  focusInstruction?: string;  // 用户指定保留重点（对标 Claude Code /compact focus on xxx）
}): Promise<{
  beforeTokenCount: number;
  afterTokenCount: number;
  compressedRange: [string, string];  // 被压缩的消息 ID 范围
  summaryMessageId: string;           // 压缩生成的摘要消息 ID
  idMapping: Record<string, string>;  // 原始 message_id → summary_message_id
  summary: string;
}>

// 模型适配
context.setModelProfile(modelId: string, profile: Partial<ModelContextProfile>): Promise<void>
context.getModelProfile(modelId: string): Promise<ModelContextProfile>
context.formatForModel(sessionId: string, userMessage: string, modelId: string): Promise<FormattedContext>
```

---

## 五、上下文组装流程

```
assemble(session_id, user_message)
     │
     ▼
1. 系统提示词（固定 + 可配置）
     │
     ▼
2. 角色设定（当前激活的 Skill/Role 的 system_prompt）
     │
     ▼
3. 项目配置（navis.md 内容）
     │
     ▼
4. 历史消息（从 Session 获取，必要时裁剪/压缩）
     │
     ▼
5. RAG 检索结果（如果启用，调用 RAG.search）← 移到历史消息之后、用户消息之前，因为 RAG 检索回应的是当次用户问题
     │
     ▼
6. 用户消息（当前输入）
     │
     ▼
7. 工具列表（根据 Skills 白名单过滤可用工具）
     │
     ▼
8. Token 计数（总 Token 超限则裁剪历史消息）
     │
     ▼
9. 模型格式化（format_for_model）
     ├── 根据目标模型的 ContextFormat 放置 system prompt
     │   ├── SystemMessage → 作为 role:"system" 消息插入消息列表头部
     │   ├── SystemField → 保留在 FormattedContext.system 字段
     │   └── NoSystemPrompt → 合并到第一条 user 消息
     ├── 使用目标模型的 Tokenizer 重新计算 Token 数
     └── 返回 FormattedContext（可直接发给 Gateway）
     │
     ▼
返回 FormattedContext
```

**会话隔离约束：**

`assemble()` 仅接受单个 `session_id`，所有上下文来源（历史消息、RAG 结果）均限定在该会话范围内。系统**不允许**自动从其他会话拉取数据注入当前上下文。

- 若用户需要引用其他会话的内容，必须通过以下**用户主动操作**之一：
  - 新建会话时选择"引用历史会话"
  - 在当前会话中显式执行"引用会话 xxx"
- Agent 不得自行决定访问其他会话的历史或结论
- 跨会话引用的上下文以**只读摘要**形式注入，不携带原始会话的完整消息链

> 详见 [08-session.md §5.2 会话隔离约束](08-session.md)

---

## 六、裁剪策略

```
Token 超限时裁剪顺序：
1. 移除 RAG 上下文（如果还有更早的消息可裁）
2. 移除项目配置（如果还有更早的消息可裁）
3. 压缩历史消息（Summary 策略）
4. 移除最早的历史消息（Sliding Window）
5. 如果仍然超限 → 报错

注：用户可通过手动压缩（context.compress）在 Token 超限前主动清理上下文，避免自动裁剪丢失重要信息。
```

---

## 七、压缩策略

### 7.1 设计对标

Navis Go 的上下文压缩对标 **Claude Code 的 /compact + autoCompact 方案**：

| 竞品 | 方案 | Navis Go 对齐 |
|------|------|-------------|
| Claude Code | LLM 自压缩：对话历史 → LLM 生成结构化摘要 → 替换原始消息 | ✅ 完全对齐 |
| Claude Code | 自动触发（80-95% 阈值）+ 手动 /compact | ✅ 完全对齐 |
| Codex CLI | 纯截断/滑动窗口 | ❌ 不采用（丢失重要上下文） |
| Hermes | 反思循环/经验提炼 | ❌ 不采用（适合工作流编排，不适合对话压缩） |

**核心原则**：使用当前用户选择的模型进行压缩（不引入额外模型），用 LLM 自身的智能判断什么信息重要。

### 7.2 压缩流程

```
触发压缩（自动 or 手动）
         │
         ▼
┌─ 分割消息历史 ─────────────────────────────────────────┐
│                                                         │
│  recent_messages（最近 N 条，不压缩）                    │
│  old_messages（更早的消息，送入压缩）                     │
│                                                         │
└─────────────────────────────────────────────────────────┘
         │
         ▼
┌─ 构造压缩请求 ─────────────────────────────────────────┐
│                                                         │
│  system: "你是一个对话压缩专家..."（压缩 Prompt）        │
│  user: old_messages（需要被压缩的原始消息）              │
│                                                         │
│  可选：focus_instruction（用户指定保留重点）             │
│                                                         │
└─────────────────────────────────────────────────────────┘
         │
         ▼
调用 Gateway.router()（使用调用方传入的 session/composer 模型）
         │
         ▼
┌─ 处理压缩结果 ─────────────────────────────────────────┐
│                                                         │
│  1. 在 Session 中插入一条系统消息标记压缩点              │
│     内容："[上下文已压缩] {summary}"                    │
│                                                         │
│  2. 记录压缩范围到 Storage.compacted_ranges             │
│     （start_message_id / end_message_id /              │
│       tail_start_message_id / summary_message_id）      │
│     原始 messages 不删除（软压缩主契约，见 08-session） │
│                                                         │
│  3. 保留 recent_messages 不动                           │
│                                                         │
│  4. 发出 context.compressed 事件                        │
│                                                         │
└─────────────────────────────────────────────────────────┘
         │
         ▼
会话继续（Prompt 组装时用 summary 替换被覆盖 range，释放 token 空间）
```

### 7.3 自动压缩

自动压缩在 `assemble()` 流程中被动触发：

- **触发条件**：`total_tokens / max_context_tokens >= auto_compress_threshold`
- **阈值**：默认 0.8（对标 Claude Code），可通过 `context.setConfig()` 调整
- **压缩方式**：调用方传入当前 Session / Composer 选定模型，Gateway 按该模型路由生成结构化摘要
- **保留策略**：保留最近 `compress_keep_recent` 条原始消息（默认 10）
- **用户感知**：通过 `context.compressed` 事件通知前端，前端可展示提示

```rust
// assemble() 中的压缩检查
let usage_ratio = total_tokens as f32 / max_context_tokens as f32;
if usage_ratio >= config.auto_compress_threshold {
    // 使用调用方传入的模型压缩早期消息（原始消息不删除）
    let old_messages = &messages[..messages.len() - config.compress_keep_recent];
    let summary = compress_with_llm(&old_messages, None).await?;
    mark_compacted_range(&mut messages, summary, config.compress_keep_recent);
    // compacted_ranges 记录覆盖范围；Prompt 组装时用 summary 替换被覆盖 range
    // （软压缩主契约，见 08-session.md；不再删除原始 messages）
    event_bus.emit("context.compressed", json!({ "trigger": "auto", ... }));
}
```

### 7.4 手动压缩

手动压缩由用户主动触发，提供更精细的控制：

- **触发方式**：用户通过 `/compact [focus_instruction]` 命令调用
- **保留策略**：默认保留最近 10 条原始消息，用户可通过 `keepRecentCount` 自定义
- **范围指定**：用户可通过 `fromMessageIndex` 精确指定从哪条消息开始压缩
- **重点保留**：用户可通过 `focusInstruction` 指定压缩时应特别保留的内容
- **Token 效果**：压缩后 Token 数应显著减少（目标压缩率 >= 50%）

### 7.5 压缩摘要 Prompt 模板

模板支持文件覆盖，优先级：**项目级 > 用户级 > 内置默认值**。

| 优先级 | 位置 | 说明 |
|--------|------|------|
| 1（最高） | `{project}/.navis/prompts/compress.md` | 项目级，针对特定项目定制压缩策略 |
| 2 | `~/.navis/prompts/compress.md` | 用户级，所有项目通用 |
| 3（默认） | 编译时嵌入 `DEFAULT_COMPRESS_TEMPLATE` | 内置默认，无需配置 |

用户创建自定义 `compress.md` 文件即可覆盖压缩策略，无需重新编译。模板中使用 `{focus}` 占位符，运行时替换为用户指定的保留重点。

**内置默认模板**（对标 Claude Code）：

```text
你是一个对话压缩专家。请将以下对话历史压缩为结构化摘要。

## 保留要求

请保留以下关键信息，确保压缩后的摘要足以让对话无缝继续：

1. **项目上下文**：项目名称、技术栈、当前开发阶段
2. **用户目标**：原始需求、关键指令、期望结果
3. **文件变更**：读写过的文件路径和变更摘要（不需要完整代码，只要改了什么）
4. **设计决策**：关键技术选择及原因（如"选择了 xterm.js 而非自研终端"）
5. **任务状态**：已完成的工作、正在进行的任务、下一步计划
6. **遗留问题**：未完成的任务、待修复的 bug、待确认的决策
7. **用户偏好**：编码风格、工具选择、约束条件（如"不要过度设计"）
8. **工具调用结果**：命令执行的关键输出（成功/失败、错误信息），不需要完整 stdout

## 不要保留

- 完整的代码内容（只保留文件路径和变更摘要）
- 详细的工具输出（只保留关键结论）
- 重复的确认对话（如"好的"、"收到"）

{focus_instruction}

## 输出格式

### 项目上下文
[项目背景、技术栈、当前阶段]

### 已完成的工作
[按时间顺序列出关键变更]

### 当前状态
[正在进行的任务、下一步计划]

### 遗留问题
[未完成项、待确认项]

### 用户偏好
[编码风格、工具选择、约束条件]

---

以下是需要压缩的对话历史：
```

### 7.6 自动压缩 vs 手动压缩对比

| 维度 | 自动压缩 | 手动压缩 |
|------|---------|---------|
| 触发方式 | Token 使用率达阈值时自动触发（默认 0.8） | 用户显式调用 `/compact` |
| 压缩范围 | 固定压缩最早消息 | 可指定起始位置 |
| 保留消息数 | 由 `compress_keep_recent` 配置决定 | 用户可自定义 |
| 重点保留 | 无 | 支持 `focusInstruction` 指定保留重点 |
| 使用模型 | 调用方传入 Session / Composer 选定模型 | 调用方传入 Session / Composer 选定模型 |
| 用户感知 | 通过事件通知前端 | 用户主动发起，返回压缩报告 |
| 适用场景 | 长会话持续对话的自动维护 | 会话中期清理、聚焦当前任务 |

### 7.7 压缩效果保障

| 保障措施 | 说明 |
|---------|------|
| 压缩点标记 | 压缩后插入系统消息 `[上下文已压缩]` 标记分界，前端可展示 |
| 消息映射 | `CompressResult.id_mapping` 记录原始消息 ID → 摘要消息 ID 的映射，供 Checkpoint 恢复 |
| 可逆性 | 压缩前的消息已存入 Storage，用户可通过 `/resume` 恢复到压缩前状态 |
| 压缩率监控 | 通过 `context.compressed` 事件报告压缩前后的 Token 数，前端可展示效果 |
| 失败降级 | LLM 调用失败时，回退到本地截取策略（前 500 + 后 500 字符） |

---

## 八、事件定义

```typescript
type ContextEvents = {
  'context.assembled':      { sessionId: string; tokenCount: number; sources: string[] }
  'context.compressed':     { sessionId: string; beforeTokens: number; afterTokens: number; compressedRange: { from: number; to: number }; summary: string; trigger: 'auto' | 'manual' }
  'context.truncated':      { sessionId: string; droppedMessages: number }
  'context.model_switched':  { sessionId: string; fromModel: string; toModel: string; tokenDelta: number }
  'context.provider.injected': { sessionId: string; providerId: string; tokensInjected: number; source: string }
  'context.assembly.failed': { sessionId: string; error: string }
}
```

---

## 九、测试策略

```
单元测试：Token 计数、裁剪策略、上下文组装顺序
集成测试：与 Session/RAG/Skills 联动、压缩效果
自动压缩测试：阈值触发正确性、压缩后 Token 数减少、关键信息保全
手动压缩测试：压缩后 Token 数减少 >= 50%、关键信息保全验证、指定范围压缩正确性、focusInstruction 生效验证
```
