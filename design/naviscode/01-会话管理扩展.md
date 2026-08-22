# 08 - Session 会话管理 详细设计

> 模块编号：08 | 层级：基础能力层
> 依赖：01-Logger, 02-Event+IPC, 03-Config, 04-Storage
> 被依赖：16-Agent, 18-Context-Manager

---

## 一、模块概述

### 1.1 定位

Session 管理对话会话的完整生命周期，包括创建、切换、归档、删除、历史管理、上下文快照、导入导出、会话恢复。

Session 是 transcript 和 Turn Timeline 的唯一业务事实源。Gateway 的 provider request/message 类型只用于模型协议发送边界，不能反向成为 Session 的核心模型。

### 1.2 职责边界

```
负责：
├── 会话 CRUD（创建/读取/更新/删除）
├── 会话切换（当前活跃会话管理）
├── 会话归档（已完成会话归档）
├── 会话绑定项目和工作树（1 Project : N Session，1 Session : 1 Project，1 Session : 1 active Worktree）
├── 历史消息管理（分页/搜索/清理）
├── Turn Timeline 管理（AgentTimelinePart 创建、更新、恢复）
├── 上下文快照（保存/加载会话上下文）
├── 会话导入导出（Markdown/JSON/文件路径）
├── 会话恢复（--continue / --resume）
├── 会话断点检查点（Checkpoint 保存/恢复）
├── 会话状态管理（活跃/归档/删除）
└── 项目切换响应（监听 project.switched 事件）

不负责：
├── 消息内容存储 → Storage
├── 上下文组装/压缩 → Context Manager
├── Agent 决策 → Agent
├── 会话级配置 → Config
└── 跨会话数据自动互通 → 禁止（见 §5.2 会话隔离约束）
```

---

## 二、架构设计

```
session/
├── mod.rs              # 模块入口
├── manager.rs          # 会话管理器
├── history.rs          # 历史消息管理
├── snapshot.rs         # 上下文快照
├── export.rs           # 导入导出
├── state.rs            # 会话状态机
├── checkpoint.rs       # 会话断点检查点
├── worktree_binding.rs # Session / Worktree 绑定
├── transcript_timeline.rs # Turn Timeline / AgentTimelinePart 事实管理
└── composer_runtime.rs # Session prompt queue / running task 投影运行态
```

`composer_runtime` 是所有 Composer 场景共用的 Session 运行态：普通 prompt、Plan draft、Plan execution、Goal guidance 和 Goal continuation 都先提交到这里，由后端根据同一 Session 是否存在 active turn / waiting approval / running task 决定 `runNow` 或 FIFO `queued`。当前运行任务与队列不写入 `Session.metadata`；metadata 只保存可恢复 UI 偏好，例如 plan mode、goal text、pending plan review。前端只能消费 `UiComposerRunState` 投影，不能自行推断 running / queue。

---

## 三、数据模型

```rust
struct Session {
    id: String,
    name: String,
    project_id: String,                 // 绑定的 Project ID（1 Session : 1 Project）
    worktree_root: PathBuf,             // 当前 Session 绑定的真实工作目录 / Git worktree 根路径
    status: SessionStatus,
    model: Option<String>,           // 会话级模型偏好；发送请求时由 Agent 写入 ProviderChatRequest.model
    system_prompt: Option<String>,   // 会话级系统提示词
    message_count: usize,
    total_tokens: usize,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
    metadata: serde_json::Value,
}

enum SessionStatus {
    Active,     // 活跃
    Archived,   // 归档
    Deleted,    // 已删除（软删除）
}

struct Message {
    id: String,
    session_id: String,
    role: MessageRole,
    content: SessionMessageContent, // Session 自有消息内容；发送前由 Context/Gateway adapter 转为 provider DTO
    token_count: Option<usize>,
    tool_calls: Option<Vec<ToolCall>>,
    tool_result: Option<ToolResult>, // Session message 字段；工具契约、批处理和结果归一仍归属 tool/agent
    created_at: DateTime<Utc>,
}

enum SessionMessageContent {
    Text(String),
    Parts(Vec<SessionContentPart>),
}

enum SessionContentPart {
    Text { text: String },
    Image { media_type: String, data: String },
    File { name: String, content: String, mime_type: Option<String> },
}

struct AgentTimelinePart {
    id: String,
    session_id: String,
    turn_id: String,
    message_id: String,
    sequence: i64,
    kind: AgentTimelinePartKind,
    status: AgentTimelinePartStatus,
    summary: String,
    detail: Option<serde_json::Value>,
    input: Option<serde_json::Value>,
    progress: Option<serde_json::Value>,
    output: Option<serde_json::Value>,
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    duration_ms: Option<i64>,
}

enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

struct SessionSnapshot {
    id: String,
    session_id: String,
    name: String,
    context: SnapshotContext,     // 保存的上下文数据
    created_at: DateTime<Utc>,
}

struct SnapshotContext {
    messages: Vec<Message>,        // 历史消息
    system_prompt: String,
    model: String,
    skills: Vec<String>,           // 激活的 Skills
    role: Option<String>,          // 当前角色
}

// 会话断点检查点（用于 --continue / --resume 恢复）
// 注意：使用 message_id（UUID）而非 message_index 作为恢复点标识，
// 避免上下文压缩后 message_index 语义变化导致的定位错误。
struct SessionCheckpoint {
    id: String,
    session_id: String,
    checkpoint_type: CheckpointType,
    agent_state: Option<AgentState>,       // Agent 任务执行状态
    execution_context: serde_json::Value,   // 执行上下文（工具调用栈、变量等）
    anchor_message_id: String,              // 断点处消息的 UUID（稳定标识，不受压缩影响）
    compression_snapshot: Option<CompressionSnapshot>,  // 压缩快照（若有）
    created_at: DateTime<Utc>,
}

enum CheckpointType {
    AutoSave,       // 应用关闭时自动保存
    TaskBreak, // Task 中断时保存
    Manual,         // 手动保存
}

// checkpoint 存储表（新建到 Storage 数据库）
// CREATE TABLE checkpoints (
//     id TEXT PRIMARY KEY,
//     session_id TEXT NOT NULL,
//     checkpoint_type TEXT NOT NULL,  -- 'auto_save' / 'task_break' / 'manual'
//     agent_state TEXT,               -- JSON，Agent 任务执行状态
//     execution_context TEXT NOT NULL, -- JSON，执行上下文
//     anchor_message_id TEXT NOT NULL, -- 断点处消息 UUID（稳定标识）
//     compression_snapshot TEXT,       -- JSON，压缩快照（可选）
//     created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
//     FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
// );
// CREATE INDEX idx_checkpoints_session ON checkpoints(session_id, created_at DESC);

// 当前软压缩主契约使用 Storage.compacted_ranges：
// - 原始 messages 不删除；
// - summary 作为普通 message 保存；
// - compacted_ranges 记录 start_message_id / end_message_id / tail_start_message_id / summary_message_id；
// - Prompt 组装时用 summary 替换被覆盖 range。
// compression_snapshot 只作为 checkpoint 恢复辅助，不再是 Prompt 压缩的唯一来源。

// ====== Checkpoint 与上下文压缩的兼容机制 ======
// 
// 核心问题：上下文压缩会删除和合并消息，如果使用 message_index 定位恢复点，
// 压缩后 index 会失效，导致恢复到错误的消息位置。
//
// 解决方案：使用 message_id（UUID）作为恢复点标识。压缩时将被合并的消息 ID
// 记录到 CompressionSnapshot 的映射表中，恢复时通过映射表查找对齐位置。
//
// 恢复流程：
//   checkpoint.anchor_message_id
//       │
//       ├── 在当前消息列表中查找 → 找到 → 恢复到该位置
//       │
//       └── 未找到（已被压缩） → 查看 compression_snapshot
//           │
//           └── 在 id_mapping 中查找该 message_id
//               │
//               ├── 找到 → 映射到压缩后的摘要消息 → 恢复到摘要之后的位置
//               │
//               └── 未找到（被彻底裁剪） → 恢复到最早可用消息 + 通知用户"部分历史已被压缩"

struct CompressionSnapshot {
    compressed_at: DateTime<Utc>,                    // 压缩发生时间
    trigger: CompressionTrigger,                     // 触发类型
    original_message_ids: Vec<String>,               // 被压缩的原始消息 ID 列表（有序）
    summary_message_id: String,                      // 压缩生成的摘要消息 ID
    id_mapping: HashMap<String, String>,             // 原始 message_id → summary_message_id
    preserved_message_ids: Vec<String>,              // 保留的原始消息 ID（最近 N 条）
}

enum CompressionTrigger {
    Auto,    // Token 达到阈值自动触发
    Manual,  // 用户通过 /compact 手动触发
}

struct AgentState {
    task_id: String,
    task_description: String,
    progress: f32,                   // 任务进度 0.0-1.0
    pending_tool_calls: Vec<String>, // 待执行的工具调用
    variables: serde_json::Value,    // 任务执行中的临时变量
}
```

---

## 四、接口定义

### 4.1 IPC 命令

```typescript
// 会话 CRUD
session.create(options: { projectId: string; worktreeRoot: string; name?: string; model?: string }): Promise<Session>
session.get(id: string): Promise<Session | null>
session.list(filter?: { status?: string; projectId?: string }): Promise<Session[]>
session.update(id: string, updates: Partial<Session>): Promise<void>
session.delete(id: string): Promise<void>
session.archive(id: string): Promise<void>
session.restore(id: string): Promise<void>

// 会话切换
session.getActive(): Promise<string | null>
session.setActive(id: string): Promise<void>

// 历史消息
session.getMessages(sessionId: string, options?: { limit?: number; offset?: number }): Promise<Message[]>
session.getMessageCount(sessionId: string): Promise<number>
session.searchMessages(sessionId: string, query: string): Promise<Message[]>
session.clearMessages(sessionId: string): Promise<void>
session.getTimelineParts(sessionId: string, turnId?: string): Promise<AgentTimelinePart[]>

// 上下文快照
session.createSnapshot(sessionId: string, name: string): Promise<SessionSnapshot>
session.listSnapshots(sessionId: string): Promise<SessionSnapshot[]>
session.loadSnapshot(snapshotId: string, confirm?: boolean): Promise<void>  // 替换当前上下文，confirm=false 时需二次确认提示
session.deleteSnapshot(snapshotId: string): Promise<void>

// 导入导出
session.export(sessionId: string, format?: 'markdown' | 'json'): Promise<string>
session.import(data: string, format?: 'markdown' | 'json'): Promise<Session>

// 统计
session.stats(): Promise<{ totalSessions: number; activeSessions: number; totalMessages: number }>

// 会话恢复（对标 Claude Code --continue / --resume）
session.continue_(): Promise<Session>                           // 继续最近一次会话，自动加载最新 AutoSave 类型的检查点
session.resume(sessionId: string): Promise<Session>             // 恢复指定历史会话，从断点继续
session.saveCheckpoint(sessionId: string, type?: 'auto' | 'manual' | 'task_break'): Promise<SessionCheckpoint>
session.loadCheckpoint(checkpointId: string): Promise<SessionCheckpoint>                         // 加载检查点
session.listCheckpoints(sessionId: string): Promise<SessionCheckpoint[]>                        // 列出会话的所有检查点
session.resolveCheckpoint(checkpointId: string): Promise<{ messageId: string; position: number }>  // 解析检查点锚点到当前消息列表中的实际位置（处理压缩映射）

// 导入导出（文件路径接口）
session.exportToFile(sessionId: string, filePath: string, format?: 'markdown' | 'json'): Promise<void>
session.importFromFile(filePath: string, format?: 'markdown' | 'json'): Promise<Session>
```

---

## 五、会话状态机

```
创建 → Active
         │
         ├── archive → Archived
         │                │
         │                └── restore → Active
         │
         ├── delete → Deleted（软删除，30天后由 Storage 定期物理清理）
         │
         └── setActive → 当前活跃会话

说明：
- Session 负责软删除（标记 status=Deleted），Storage 负责定期物理清理（30天后彻底删除）
- 物理清理由 Storage 后台定时任务执行，扫描 status=Deleted 且 updated_at 超过 30 天的记录
```

### 5.2 会话隔离约束

**核心原则：跨会话引用必须由用户主动发起，系统不允许自动跨会话拉取数据。**

```
允许（用户主动）：
├── 新建会话时，用户选择"引用某历史会话的上下文"
├── 在已有会话中，用户显式执行"引用会话 xxx"的操作
├── 用户手动加载另一个会话的快照（session.loadSnapshot）
└── 用户通过 /resume 或 --continue 恢复指定会话

禁止（系统自动）：
├── Agent 自动访问其他会话的历史消息
├── Context Manager 自动从其他会话拉取上下文
├── 系统自动将会话 A 的结论/经验注入会话 B
├── 跨会话的 RAG 检索结果自动共享
└── 新建会话时自动继承上一个会话的上下文
```

**设计理由：**
- 会话是用户的独立工作单元，自动跨会话会导致上下文污染
- 用户对"哪些信息跨会话共享"拥有完全控制权
- 避免 Agent 在不同会话间产生不一致的记忆幻觉

**实现约束：**
- Session 模块的所有查询接口默认仅返回当前会话数据，不支持跨会话查询（除非用户显式指定 sessionId）
- Context Manager 的 `assemble()` 仅接受单个 session_id，不接受多会话参数
- Agent 的 AgentContext 仅包含当前会话信息，不自动注入其他会话数据

---

## 六、事件定义

```typescript
type SessionEvents = {
  'session.created':       { sessionId: string; projectId: string; worktreeRoot: string }
  'session.deleted':       { sessionId: string }
  'session.archived':      { sessionId: string }
  'session.restored':      { sessionId: string }
  'session.switched':      { from: string | null; to: string }
  'session.updated':       { sessionId: string; changes: Array<{ field: string; oldValue?: any; newValue?: any }> }
  'session.snapshot.created': { sessionId: string; snapshotId: string }
  'session.snapshot.loaded':  { sessionId: string; snapshotId: string }
  'session.snapshot.deleted': { sessionId: string; snapshotId: string }
  'session.message.added':   { sessionId: string; messageId: string; role: string }
  'session.message.deleted': { sessionId: string; messageId: string }
  'session.timeline.part.upserted': { sessionId: string; turnId: string; partId: string }
  'session.exported':      { sessionId: string; format: string }
  'session.imported':      { sessionId: string }
  'session.continued':     { sessionId: string; checkpointId?: string }       // --continue 恢复
  'session.resumed':       { sessionId: string; checkpointId?: string }       // --resume 恢复
  'session.checkpoint.saved':   { sessionId: string; checkpointId: string; type: string }
  'session.checkpoint.loaded':  { sessionId: string; checkpointId: string }
  'session.checkpoint.deleted': { sessionId: string; checkpointId: string }
  'session.auto_save':     { sessionId: string; checkpointId: string }        // 应用关闭时自动保存
}
```

---

## 七、性能指标

| 指标 | 要求 | 说明 |
|------|------|------|
| 会话创建 | < 50ms | 含 Storage 写入 |
| 会话切换 | < 10ms | 内存操作 |
| 消息查询（100条） | < 20ms | 带分页 |
| 快照创建 | < 100ms | 序列化上下文 |
| 导出（1000条消息） | < 500ms | Markdown 格式化 |
| 检查点保存 | < 200ms | 含 Agent 状态序列化 |
| 会话恢复（continue/resume） | < 300ms | 含检查点加载 |
| 自动保存（关闭时） | < 500ms | 含完整状态持久化 |

---

## 八、项目切换响应

Session 与 Project / Worktree 的关系：**1 Project : N Session，1 Session : 1 Project，1 Session : 1 active Worktree**。每个 Session 必须绑定到一个 Project 和一个当前 Worktree；一个 Project 下可以有多个 Session，也可以对应多个 Worktree。

当收到 `project.switched` 事件时，Session 模块自行响应（Project 模块不主动协调）：

1. **保存当前会话**：对当前 Project 的活跃 Session 执行 AutoSave 检查点保存
2. **切换活跃会话**：查找目标 Project 下最近活跃的 Session，设为当前活跃会话
3. **无历史会话**：若目标 Project 下无 Session，自动创建新 Session 并绑定
4. **事件通知**：发出 `session.switched` 事件，通知 Agent、Context Manager 等模块

```
project.switched 事件到达（Session 自行监听，非 Project 主动调用）
         │
         ▼
保存当前 Project 活跃 Session 的检查点
         │
         ▼
查找目标 Project 的最近活跃 Session
         │
         ├── 存在 → setActive → 发出 session.switched
         │
         └── 不存在 → createSession → 绑定 Project → 发出 session.switched
```

---

## 九、测试策略

```
单元测试：会话 CRUD、状态转换、快照序列化、检查点保存/加载
集成测试：会话切换、历史消息分页、导入导出完整性、--continue/--resume 恢复流程
场景测试：应用关闭后自动保存与重启恢复、Agent 任务中断后断点恢复
```
