# Navis Go Kernel 设计

> 本文档定义 Navis Go 的底层内核。它不是功能清单，而是所有功能共同依赖的地基。
> 内核目标是：底座稳定、核心收紧、扩展留好。

---

## 1. 结论

Navis Go Kernel 只保留四个原语：

```text
Registry   -> 有什么能力可用
Pipeline   -> 能力怎样执行
Event Bus  -> 状态怎样通知
Policy     -> 谁能做什么
```

这四个原语可以支撑整栋 Navis Go：

- Agent tool、MCP tool、Provider、Transport adapter、Policy constraint 等可执行或可生命周期管理的能力通过 `Registry` 发现。
- UI Host view renderer 是 UI 域内置的视图渲染策略，不进入 Kernel；Kernel 不感知 DOM、菜单、布局、placement 或 renderer 字符串。
- Agent loop、Tool run、Gateway request、MCP transport、Review/Diff 都通过 `Pipeline` 执行。
- AgentTimelinePart、Task、SessionChange、Settings、Extension 都通过 `Event Bus` 通知。
- Sandbox、工具审批、扩展权限、模式限制、quota、rate limit、trust 都通过 `Policy` 判断。

不增加第五个原语。缺能力时优先补四个原语的字段、trait、adapter 或 stage，而不是新建平行系统。

以下能力不是新原语，而是挂在四个原语下面的 Linux-style 横切底座规范：

```text
Kernel
├─ Registry                  -> 有什么能力可用
│  ├─ KernelObjectInfo        统一描述 registry.entry
│  ├─ ResourceLease           描述 capability lifecycle / remove 的资源持有
│  ├─ PolicyCheckpoint        registry.lifecycle
│  └─ Snapshot                RegistryStats
│
├─ Pipeline                  -> 能力怎样执行
│  ├─ KernelObjectInfo        统一描述 pipeline.run / stage
│  ├─ ResourceLease           描述 active pipeline run / cancellation / shutdown
│  ├─ PolicyCheckpoint        pipeline.run
│  ├─ ExecutionObservation    Pipeline 执行生命周期事件的程序化消费入口
│  └─ Snapshot                PipelineStats
│
├─ Event Bus                 -> 状态怎样通知
│  ├─ KernelObjectInfo        统一描述 event.subscription / event.dispatcher
│  ├─ ResourceLease           描述 subscription / dispatcher handler 生命周期
│  ├─ PolicyCheckpoint        event.subscribe / event.emit
│  └─ Snapshot                EventBusStats
│
└─ Policy                    -> 谁能做什么
   ├─ KernelObjectInfo        统一描述 policy.constraint
   ├─ ResourceLease           描述动态 constraint 的注册和释放
   ├─ PolicyCheckpoint        checkpoint 定义和评估入口
   └─ Snapshot                PolicyStats

横切到全部原语：
├─ KernelErrorKind            Registry / Pipeline / EventBus / Policy 共享错误分类
└─ Audit                      Registry / Pipeline / EventBus / Policy 共享结构化审计事实
```

层级规则：

- `Registry` / `Pipeline` / `Event Bus` / `Policy` 是唯一一等原语。
- `KernelObjectInfo`、`ResourceLease`、`PolicyCheckpoint`、`Snapshot`、`KernelErrorKind`、`Audit` 是原语的底座规范，不独立承载业务流程。
- `KernelObjectInfo` 只描述对象，不执行对象；执行仍由 Registry lookup + Pipeline stage 完成。
- `ResourceLease` 只管理持有和释放，不定义业务资源类型。
- `PolicyCheckpoint` 只定义检查点，不内置业务权限语义。

---

## 2. 内核不变量

四个原语必须共同遵守以下不变量。它们不是新原语，而是建筑规范。

### 2.1 Context 必须贯穿

每次注册、执行、发布事件、策略判断都必须携带 `KernelContext`。

```rust
struct KernelContext {
    trace_id: TraceId,
    scope: KernelScope,
    scope_key: String,            // 构造时缓存，热路径用 scope_key_ref() 避免重复 format；含 debug_assert 非空校验
    source: String,              // 业务层定义来源标识
    started_at: DateTime<Utc>,
    deadline: Option<DateTime<Utc>>,
    owner: Option<String>,       // 业务层定义归属（谁创建的这个操作）
    metadata: Option<SharedArc<Value>>,  // 业务层自定义扩展字段；None 表示无扩展，避免空 BTreeMap 分配
}

enum KernelScope {
    Global,                      // 全局操作
    Scoped { id: String, kind: String },  // 业务层定义 scope 类型和 id
    // 示例（业务层使用，非内核定义）：
    // Scoped { id: "sess_001", kind: "session" }
    // Scoped { id: "task_002", kind: "task" }
    // Scoped { id: "extension_x", kind: "extension" }
}
```

scope 隔离规则：

- `scope` 为 `Scoped { kind, id }` 时，Event Bus 的 `emit()` 通过 `EventEnvelope.context.scope` 携带 scope，订阅时按 `scope_key` 过滤。
- Policy 的 `PolicyInput.scope` 字段携带完整 scope 字符串，由业务层 Constraint 解读语义。
- Pipeline 的 `PipelineContext.trace_id` 用于串联所有原语。
- `Extension` scope 下，Policy 必须应用扩展权限和沙箱约束。
- `trace_id` 必填，用于串联 Registry / Pipeline / Event / Policy。
- `deadline` 存在时，Pipeline stage 不得忽略超时。

### 2.2 Event Bus 不做事实源

Event Bus 只通知，不保存最终事实。

事实源固定为：

| 事实 | 唯一事实源 |
|------|------------|
| 对话消息 | SQLite `messages` |
| Agent 执行过程 | SQLite `agent_timeline_parts` |
| 文件变更 | SQLite `session_changes` |
| 后台/子任务 | SQLite `tasks`（TaskManager 是内存索引，不是事实源） |
| 记忆 | SQLite `memories` |
| 配置 | Config + Storage |
| 能力注册 | Registry + manifest |
| 审计记录 | SQLite `audit_log` |

事实提交规则：

```
写事实 → 写成功 → 发事件
写事实 → 写失败 → 不发事件 → 返回错误
```

具体约束：
- **写事实和发事件必须在同一调用栈内顺序执行**，不允许异步拆分
- **写事实失败时，禁止发布事件**。调用方必须返回错误，不允许忽略写入失败
- **事件不携带事实内容**，只携带事实的 id 和变更摘要。消费者必须从事实源读取完整数据
- **不使用 outbox 模式**。桌面单进程场景下，同步顺序写入足够可靠，outbox 增加不必要的复杂度
- **如果事实写入成功但事件发布失败**，记录警告日志，不影响事实的正确性。UI 可通过轮询事实源补偿

后台/子任务结果回传也遵守同一规则：

- 子任务自身状态属于 `tasks` 事实源；右侧 Background Task 面板只读取任务投影。
- 子任务完成、失败或取消后，如果父 Agent 需要继续感知结果，业务层必须向父 `session` 写入一条 compact synthetic message。
- synthetic message 是父 Agent 的语义输入事实；写入成功后由 `SessionManager::add_message()` 发布 `session.message.added`。
- EventBus / `ExecutionObservationSink` 只用于 UI 投影、指标和执行过程观察，不负责把子任务结果注入父 Agent 上下文。
- synthetic message 只包含摘要、状态、task id、child session id、工具数、token 数、耗时和结构化结果/错误；完整 transcript 仍从 child session 读取，不能污染父上下文。

示例（Pipeline 中的业务事实写入 Stage + EmitEventStage）：

```rust
// WriteAgentTimelinePartStage：写业务事实源
impl Stage for WriteAgentTimelinePartStage {
    async fn process(&self, ctx: &mut PipelineContext, next: Next<'_>) -> Result<()> {
        let data = ctx.data.downcast_ref::<ToolPipelineData>().unwrap();
        self.store.insert_agent_step(&data.to_agent_step())?;  // 写 SQLite
        // 写失败 → 返回 Err，Pipeline 中止，后续 EmitEventStage 不会执行
        next.call().await
    }
}

// EmitEventStage：发事件（只在事实写入成功后执行）
impl Stage for EmitEventStage {
    async fn process(&self, ctx: &mut PipelineContext, next: Next<'_>) -> Result<()> {
        next.call().await?;  // 确保后续 Stage 完成
        let data = ctx.data.downcast_ref::<ToolPipelineData>().unwrap();
        let _ = self.bus.emit(EventEnvelope::new(
            "action.completed",
            ctx.context.clone(),
            data.to_event_payload(),
        ));
        // 事件发布失败不影响事实正确性，只记警告
        Ok(())
    }
}
```

### 2.3 Pipeline 必须原生支持取消、超时、进度

所有 Pipeline 必须支持：

- cancellation
- timeout / deadline
- execution observation（通过 `ExecutionObservationSink` 程序化消费 run/stage/capability 生命周期事件）
- retry budget
- structured error
- audit trace

这不是可选能力。Bash、webfetch、browser、Task Sidechain、MCP remote、Gateway stream 都依赖它。

### 2.4 Registry 和 Event 必须版本化

以下内容必须带版本：

- Capability schema
- Capability adapter
- Contribution manifest
- Renderer hint
- Policy rule
- Event payload

版本格式：

```rust
struct SchemaVersion {
    major: u16,
    minor: u16,
}
```

版本规则区分两种场景：

| 场景 | 策略 | 说明 |
|------|------|------|
| **注册时**（Tool schema、Extension manifest、Provider adapter） | major 不同 → **fail-closed**，拒绝加载 | 注册阶段必须严格，防止协议版本不同的能力进入系统 |
| **运行时**（Event payload、RPC 参数、AgentTimelinePart payload） | major 相同 → 按 schema 读取必需字段，忽略未知可选字段 | 运行时只接受当前 major 的 payload，缺失必需字段即报错 |

运行时读取规则：

- 消费者遇到未知字段 → 忽略，不报错
- 消费者遇到缺失必需字段 → 返回协议错误
- 消费者遇到缺失可选字段 → 使用 schema 默认值
- 生产者不应删除已有字段（只增不删）
- major 变更 = 协议语义变更（字段删除或语义改变）
- minor 变更 = schema 扩展（新增字段或可选字段）

版本不匹配时 fail-closed，不能静默转换。

### 2.5 Audit 必须内建

每次 Pipeline 执行至少记录：

- 谁发起：`KernelContext.source` + `KernelContext.scope`
- 调用了什么：能力 id / stage id / action
- Policy 为什么允许或拒绝
- 执行多久
- 输入摘要和输出摘要
- 是否失败、是否截断、是否重试

Audit 双通道输出：

| 通道 | 载体 | 用途 |
|------|------|------|
| **结构化事实源** | SQLite `audit_log` 表（通过 `AuditSink` trait 接入） | 可查询、可聚合、可回溯 |
| **日志通道** | tracing span | 实时观察、日志文件落盘 |

两通道独立，互为备份。
`foundation/logger::AuditLayer` 只属于日志通道：它消费 `audit=true` 的 tracing 事件并写入观察日志文件，不实现 `AuditSink`，也不作为结构化业务审计事实源。结构化审计事实必须通过 `kernel::AuditRecorder` / `AuditSink` 写入。

Pipeline 执行观测的完整通道模型：

| 通道 | 载体 | 消费者 | 特点 |
|------|------|--------|------|
| 结构化审计事实 | `AuditSink` → SQLite | 查询、聚合、回溯 | 持久化、结构化、面向合规 |
| tracing span/event | `tracing_subscriber` Layer | 日志文件、console | 非结构化、面向人类阅读 |
| 执行观察 | `ExecutionObservationSink` | UI 投影、指标采集 | 结构化、面向程序化消费、零拷贝 |

三通道独立，不强制语义对齐。Audit 关注"谁做了什么、结果如何"（合规事实），tracing 关注"执行路径长什么样"（调试可见性），ExecutionObservation 关注"执行过程中发生了什么"（程序化消费）。

**`BufferedAuditSink`（`crossbeam-channel`）：**

默认的 `InMemoryAuditSink` 只用于测试。生产环境使用 `BufferedAuditSink`，通过 `crossbeam-channel` 异步批量 flush：

```rust
pub struct BufferedAuditSink {
    tx: crossbeam_channel::Sender<Arc<AuditRecord>>,  // bounded(8192)
    // 后台线程每 100ms 或每 64 条 flush 一次到后端 sink
}
```

- `record_owned()` / `record_shared()` 用 `try_send` 非阻塞入队，不阻塞 Pipeline 执行
- 后台线程批量消费，减少 SQLite 写入次数
- sink 边界传递 `Arc<AuditRecord>`，后台缓冲不深拷贝审计记录
- channel 满时返回 `AuditSinkFailed`（fail-closed）

`AuditRecord` 提供 builder 风格补充字段，业务 Stage 不应重复手写字段赋值：

```rust
let record = AuditRecord::new(&ctx.context, "tool.execute", "capability.execute", AuditStatus::Success)
    .with_duration(duration)
    .with_policy_decision(json!({ "decision": "allow" }))
    .with_value_input_metadata(&input)
    .with_text_output(output_text, 512);

ctx.audit.record_owned(record)?;
```

`record(&AuditRecord)` 已标记为 `#[deprecated]`，因为它内部会 clone 整个 record 再包装为 `Arc`。
优先使用 `record_owned(record)` 或 `record_shared(Arc::new(record))` 避免不必要的堆分配。

`AuditRecorder::is_enabled()` 用于启动期或测试期断言结构化审计是否已经接入。
`PipelineContext::new()` 默认使用 disabled recorder；生产执行链必须通过
`with_audit(audit_recorder)` 显式注入。

```sql
CREATE TABLE audit_log (
    id          TEXT PRIMARY KEY,
    trace_id    TEXT NOT NULL,
    scope       TEXT NOT NULL,       -- KernelScope JSON
    source      TEXT NOT NULL,
    capability  TEXT NOT NULL,       -- 能力 id
    action      TEXT NOT NULL,
    policy_decision TEXT,            -- Allow/Deny/Ask + reason
    duration_ms INTEGER,
    input_digest  TEXT,              -- AuditDigest JSON
    output_digest TEXT,              -- AuditDigest JSON
    status      TEXT NOT NULL,       -- success/fail/truncated/retried
    created_at  TEXT NOT NULL
);
```

输入/输出摘要采用结构化枚举，不允许自由格式：

```rust
enum AuditDigest {
    /// 截断后的文本片段，保留原始大小
    Truncated { text: String, original_bytes: usize, truncated: bool },
    /// 只记录结构化元数据（字段名、类型、大小），不记录值
    Metadata { fields: Vec<FieldMeta> },
    /// 完全脱敏，只记录存在性
    Redacted { content_type: String },
    /// 不记录
    None,
}

struct FieldMeta {
    name: String,
    value_type: String,  // "string" / "number" / "object" / "array"
    byte_size: usize,
}
```

默认策略：工具输入/输出使用 `Truncated`（截断到 512 字节），敏感字段（API key、token、密码）自动降级为 `Redacted`。

审计记录不是日志的附属品，它是独立的可查询事实源。

### 2.6 默认 fail-closed

以下情况必须失败，不能静默成功：

- Registry 找不到能力。
- Pipeline 缺少必需 stage。
- Policy 无法判断。
- Event payload 版本不支持。
- 贡献方声明能力但运行时无法连接。
- 传输层没有真实的发现/调用能力。
- 适配器无法解析远端响应。

### 2.7 内核禁止包含业务语义

内核代码中不允许出现以下业务词汇：

| 禁止的词汇 | 说明 |
|-----------|------|
| `tool` / `read_file` / `bash` / `deploy` | 内核不知道什么是工具 |
| `session` / `agent` / `message` / `turn` | 内核不知道什么是会话 |
| `provider` / `openai` / `anthropic` / `mimo` | 内核不知道什么是模型提供方 |
| `extension` / `manifest` / `contributes` | 内核不知道什么是扩展 |
| `extension` / `menu` / `panel` / `theme` | 内核不知道什么是 UI 扩展 |
| `agent_step` / `permission` / `thinking` | 内核不知道什么是执行步骤 |
| `mcp` / `stdio` / `sse` / `websocket` | 内核不知道什么是传输协议 |

内核只使用以下抽象概念：

```
id / name / kind / topic / scope / subject / action / resource
version / state / metadata / trace_id / timestamp / payload
Stage / Constraint / Capability / Registry / Pipeline / EventBus
```

验证方法：`grep -r "tool\|session\|provider\|extension\|extension\|mcp" src-tauri/src/kernel/` 应返回零结果。

### 2.8 Linux-style 内核完备性边界

Navis Go Kernel 对标 Linux 内核的**设计模式**，不照搬 OS 功能。目标是形成应用内核的底座能力：

```text
少数稳定原语
  + 明确对象身份
  + 严格生命周期
  + 强制策略检查点
  + 可观测状态面
  + 结构化错误语义
```

这些能力不是第五个原语，而是挂在四个原语下面的内核规范：

| Linux-style 模式 | Navis Kernel 对应 | 挂载位置 | 说明 |
|------------------|-------------------|----------|------|
| `kobject` / `sysfs` | `KernelObjectInfo` / snapshot | 四原语对象描述 | 统一描述 Registry entry、Pipeline run、Event subscription、Policy constraint |
| refcount / resource ownership | `KernelResource` / `ResourceLease` | Registry / Pipeline / EventBus / Policy 生命周期 | 管理资源持有、释放、shutdown 顺序和泄漏检测 |
| LSM hook | Policy checkpoint | Policy 治理四原语入口 | Registry lifecycle、Pipeline run、Event subscribe 等关键入口可强制过 Policy |
| tracepoint / procfs | stats / snapshot | 四原语状态面 | 每个原语提供轻量可查询状态，不依赖日志反推 |
| errno 分类 | `KernelErrorKind` | 四原语共享错误语义 | 错误可区分 retryable、policy、invariant、resource、version |
| scheduler / workqueue | 不进入当前内核 | 不挂载 | Tokio runtime 已是调度层；只有出现明确内核后台队列需求才评估 |
| VFS | 不进入当前内核 | 不挂载 | 文件系统是业务能力，不应被 Kernel 吞并 |
| module loader | 不进入当前内核 | 不挂载 | Extension loader 是应用服务，Kernel 只承载注册和生命周期 |

**完备性判断：** 如果某个新能力能同时服务 Registry / Pipeline / EventBus / Policy / Audit 中至少两个原语，并且不包含业务词汇，才允许进入 Kernel。否则应留在业务域。

### 2.9 内核对象模型

内核对象模型已经落地在 `core/object.rs`，只描述对象身份和可观测信息，不提供业务执行接口。Registry entry、Pipeline、Pipeline stage、Event subscription、Event dispatcher、Policy constraint 都可以导出为同一种对象描述。

```rust
pub enum KernelObjectState {
    Registered,
    Enabled,
    Running,
    Stopping,
    Removed,
    Failed,
    Unknown,
}

pub struct KernelObjectInfo {
    pub id: String,
    pub kind: String,             // "registry.entry" / "pipeline.run" / "event.subscription" / ...
    pub state: KernelObjectState,
    pub scope: String,
    pub owner: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub metadata: SharedArc<Value>,
}
```

对象模型的用途：

- 统一导出内核状态快照，类似 Linux 的 `/proc` / `sysfs` 思路。
- 统一审计对象创建、替换、注销和生命周期变化。
- 让 UI/诊断工具查看对象状态，而不是读取各模块私有字段。

对象模型的禁止事项：

- 不把业务对象塞进内核。`Tool`、`Provider`、`Extension`、`Session` 都不是内核对象类型。
- 不让 `KernelObjectInfo` 成为执行入口。执行仍由业务 trait + Pipeline Stage 完成。
- 不在对象模型中存大 payload。metadata 只保存摘要和可观测字段。

### 2.10 资源租约与 shutdown

内核资源语义已经落地在 `core/object.rs`，用于统一资源持有、活跃租约计数和 shutdown 边界。它避免订阅、pipeline run、后台 dispatcher、registry entry 等资源只能靠 `drop` 顺序隐式释放。

```rust
pub struct ResourceLease {
    pub id: String,
    pub object_id: String,
    pub owner: Option<String>,
    pub acquired_at: DateTime<Utc>,
}

pub trait KernelResource: Send + Sync {
    fn object_info(&self) -> KernelObjectInfo;
    fn active_leases(&self) -> usize;
    fn shutdown(&self, mode: ShutdownMode) -> KernelResult<()>;
}

pub enum ShutdownMode {
    Graceful,
    Deadline(DateTime<Utc>),
    Immediate,
}
```

实现原则：

- lease id 使用时序 UUID v7；未来只有在真实出现 handle 复用检测需求时，才评估 generational handle。
- shutdown 必须给出明确结果：Graceful / Deadline 在仍有活跃租约时 fail-closed，Immediate 可以清理或中断。
- shutdown 顺序由对象依赖关系决定，不能靠 drop 顺序碰运气。
- `Registry`、`Pipeline`、`EventBus`、`PolicyEngine` 均实现 `KernelResource`；Audit 通过 recorder/sink stats 进入 snapshot，不作为第五个资源原语。

### 2.11 Policy checkpoint

Policy checkpoint 已落地在 `policy/mod.rs`，用于把“关键入口前必须经过 Policy”表达成通用内核概念。checkpoint 不自动写入业务语义，调用方只传入 `PolicyInput`，由 Constraint 解释 metadata。

| Checkpoint | 触发点 | 默认 action | 说明 |
|------------|--------|-------------|------|
| `registry.lifecycle` | capability enable/start/stop/remove | `registry.lifecycle` | 能力启停、注销前检查 |
| `pipeline.run` | Pipeline run 前 | `pipeline.run` | 可按 source/scope/stage 集合限制 |
| `event.subscribe` | 创建订阅前 | `event.subscribe` | 防止跨 scope 监听 |
| `event.emit` | 发布事件前，可选 | `event.emit` | 默认不开，敏感 topic 可启用 |
| `audit.read` | 查询审计前 | `audit.read` | 防止低权限主体读取审计事实 |

```rust
pub struct PolicyCheckpoint {
    pub id: String,
    pub input: PolicyInput,
}

impl PolicyEngine {
    pub fn evaluate_checkpoint(&self, checkpoint: &PolicyCheckpoint) -> PolicyDecision;
    pub fn ensure_checkpoint_allowed(&self, checkpoint: &PolicyCheckpoint) -> KernelResult<()>;
}
```

checkpoint 是内核通用概念，不包含业务动作。业务层通过 metadata 注入语义字段，由 Constraint 解释；内核只保证 deny / approval / no-match 都 fail-closed。

### 2.12 内核可观测快照

每个原语提供轻量 stats / objects，`snapshot.rs` 汇总为 `KernelSnapshot`。snapshot 不依赖日志反推状态，也不读取业务事实源。

```rust
pub struct KernelSnapshot {
    pub captured_at: DateTime<Utc>,
    pub objects: Vec<KernelObjectInfo>,
    pub registries: Vec<RegistryStats>,
    pub pipelines: Vec<PipelineStats>,
    pub event_buses: Vec<EventBusStats>,
    pub policies: Vec<PolicyStats>,
    pub audits: Vec<AuditStats>,
}
```

最小 stats 要求：

| 原语 | Stats 字段 |
|------|------------|
| Registry | entry_count、available_count、by_kind、by_state |
| Pipeline | active_runs、completed_runs、failed_runs、total_runs、total_duration_ms |
| EventBus | subscription_count、history_len、history_capacity、queue_len、queue_capacity、dispatcher_running |
| Policy | constraint_count、min_priority、max_priority |
| Audit | enabled、attempted_records、succeeded_records、failed_records |

性能要求：snapshot 只读、无阻塞 I/O；聚合使用原子计数器、无锁读结构或小集合读锁，不能扫描大 payload。延迟直方图、指标 facade 等属于压测驱动增强，不是当前内核完整性的缺口。

### 2.13 错误分类

`KernelErrorKind` 已落地在 `core/error.rs`，供 Pipeline retry、UI 呈现和审计统一判断：

```rust
pub enum KernelErrorKind {
    NotFound,
    AlreadyExists,
    NotEnabled,
    RequiredMissing,
    TypeMismatch,
    Cancelled,
    Deadline,
    Policy,
    RequiresApproval,
    Version,
    Payload,
    Resource,
    Transient,
    Invariant,
    Internal,
}

impl KernelErrorKind {
    pub fn is_retryable(&self) -> bool;        // Transient | Resource | Deadline
    pub fn is_policy_error(&self) -> bool;     // Policy | RequiresApproval
    pub fn is_invariant_violation(&self) -> bool; // AlreadyExists | RequiredMissing | TypeMismatch | Version | Payload | Invariant
}

impl std::fmt::Display for KernelErrorKind {
    // 输出 snake_case 小写形式：not_found, already_exists, policy, ...
}

impl KernelError {
    pub fn kind(&self) -> KernelErrorKind;
    // 委托方法（向后兼容）
    pub fn is_retryable(&self) -> bool;
    pub fn is_policy_error(&self) -> bool;
    pub fn is_invariant_violation(&self) -> bool;
    // 便捷构造器
    pub fn invalid_input(message: impl Into<String>) -> Self;
    pub fn transient(message: impl Into<String>) -> Self;
    pub fn cancelled() -> Self;
    pub fn deadline_exceeded() -> Self;
    pub fn policy_denied(reason: impl Into<String>) -> Self;
    pub fn policy_requires_approval(reason: impl Into<String>) -> Self;
    pub fn policy_undecidable(reason: impl Into<String>) -> Self;
    pub fn policy_constraint_not_found(id: impl Into<String>) -> Self;
    pub fn policy_constraint_already_registered(id: impl Into<String>) -> Self;
}
```

默认原则：

- Policy deny / approval / version mismatch / invariant violation 不重试。
- queue full、temporary sink failure、transient stage failure 可由调用方选择重试。
- Stage 失败会包装为 `StageFailed { id, kind, message }`，其中 `kind` 必须保留原始错误分类；Pipeline retry 以 `is_retryable()` 为准，不能把 Policy deny 当作临时失败重试。
- 所有错误必须可审计，不允许只返回自由字符串。

---

## 3. 内核目录

```text
src-tauri/src/kernel/
  mod.rs                   # re-export 所有公开类型
  snapshot.rs              # KernelSnapshot 聚合 objects / stats
  core/
    mod.rs
    context.rs             # KernelContext、KernelScope
    error.rs               # KernelError、KernelResult
    id.rs                  # CapabilityId / StageId / PolicyId / Topic / SubscriptionId / TraceId / SpanId
    object.rs              # KernelObjectInfo、ResourceLease、KernelResource、ShutdownMode
    version.rs             # SchemaVersion、版本判断
  registry/
    mod.rs                 # Capability trait、Registry / AsyncRegistry trait、InMemoryRegistry（arc-swap + im）
  pipeline/
    mod.rs                 # Stage trait、Pipeline、Next、PipelineContext（downcast-rs）、PipelineRetryPolicy（backon）
  policy/
    mod.rs                 # Constraint trait、PolicyEngine、PolicyDecision
  event/
    mod.rs                 # EventBus trait、EventEnvelope、InMemoryEventBus（flume + spawn_blocking/spawn）
  audit/
    mod.rs                 # AuditRecord、AuditSink、AuditRecorder、BufferedAuditSink（crossbeam-channel）
  observability/
    mod.rs                 # ExecutionEvent、ExecutionEventKind、ExecutionObservationSink（triomphe::Arc）
```

目录职责：

| 文件 | 职责 |
|------|------|
| `core/context.rs` | `KernelContext`、scope、deadline、trace |
| `core/id.rs` | 所有 ID newtype（`uuid::now_v7()` 时序有序），`From` impl 含非空校验（debug 构建 assert） |
| `core/error.rs` | `KernelError`（含 `PolicyErrorKind` 子枚举收敛 Policy 变体）、`KernelErrorKind`（含 `Display` impl + `is_retryable` 等分类方法）、fail-closed 错误类型 |
| `core/object.rs` | `KernelObjectInfo`（`with_updated_at_now()` builder）、`ResourceLease`、`KernelResource`、`ShutdownMode` |
| `core/version.rs` | schema version、注册时 fail-closed、运行时按当前 major 解析 |
| `registry/mod.rs` | 通用 Registry trait、异步生命周期 hook、`arc_swap::ArcSwap` 无锁读 + `im::HashMap` 结构共享写 |
| `pipeline/mod.rs` | Pipeline / Stage / Next / `PipelineData`（`downcast-rs`）/ 重试（`backon`） |
| `policy/mod.rs` | Subject / Action / Target / Scope / Decision / Constraint |
| `event/mod.rs` | EventEnvelope / EventBus trait / `InMemoryEventBus`（`flume` 异步分发 + `spawn_blocking` / `spawn`） |
| `audit/mod.rs` | AuditRecord / AuditSink / AuditRecorder / `BufferedAuditSink`（`crossbeam-channel` 批量 flush） |
| `observability/mod.rs` | `ExecutionEvent` / `ExecutionEventKind` / `ExecutionObservationSink`（`triomphe::Arc` 共享事件） |
| `snapshot.rs` | `KernelSnapshot`，聚合四原语 objects / stats 和 Audit stats |

内核目录不允许引用业务模块，例如 `agent`、`gateway`、`mcp`、`extension`、`ui`。业务模块可以引用内核。

---

## 4. 第三方库选型

### 4.1 必用库

| 库 | 用途 | 原因 |
|----|------|------|
| `tokio` | async runtime、spawn_blocking、timeout | Pipeline cancellation/progress 需要异步原语 |
| `tokio-util` | `CancellationToken` | Pipeline 和 Task 的取消/超时原生依赖 |
| `async-trait` | async trait | Rust stable trait async 仍不够直接 |
| `serde` / `serde_json` | Event payload、Policy input | 所有跨模块契约都需要稳定序列化 |
| `thiserror` | 内核错误枚举 | KernelError 必须结构化 |
| `uuid` | trace id、event id、capability id（`now_v7()` 时序有序） | SQLite B-tree 友好 |
| `chrono` | deadline、timestamp、audit time | 支持 serde |
| `tracing` | audit span、stage trace、policy decision trace | 贯穿四原语 |
| `parking_lot` | `RwLock` / `Mutex`（无中毒） | 消除锁中毒风险，Windows 性能 2-5x |

### 4.2 内核专用库

| 库 | 用途 | 模块 |
|----|------|------|
| `arc-swap` | Registry 无锁读快照 | registry |
| `im` | Registry 写路径结构共享 map | registry |
| `flume` | EventBus 异步事件分发（sync/async 双模、MPMC、背压） | event |
| `triomphe` | 轻量 `Arc`（无弱引用开销） | event（`SharedEventEnvelope`）、observability（`SharedExecutionEvent`）、registry（`CapabilityInfo.metadata`） |
| `glob` | EventBus topic 通配符匹配（subscribe 时编译） | event |
| `downcast-rs` | PipelineData 类型安全 downcast | pipeline |
| `backon` | Pipeline 指数退避重试（jitter + 条件重试） | pipeline |
| `futures` | 同步边界驱动 async Pipeline | pipeline |
| `crossbeam-channel` | AuditSink 异步批量 flush（MPMC 无锁） | audit |

### 4.3 压测驱动候选库

以下库只有在压测或诊断需求证明当前实现不够时引入；它们不是当前内核完整性的缺口，禁止“先加依赖再找用途”。

| 库 | 用途 | 对应能力 | 引入条件 |
|----|------|----------|----------|
| `slotmap` | generational handle，避免 ABA | KernelObject / ResourceLease | 需要对象句柄或 lease id 复用检测时 |
| `smallvec` | 小集合栈上存储 | Policy / Pipeline | stage、constraint、required_stage 等小列表成为热点时 |
| `indexmap` | 稳定顺序 map | Snapshot / diagnostics | 需要可重复排序的诊断输出时 |
| `hdrhistogram` | 延迟分布统计 | PipelineStats / AuditStats | 需要 p50/p95/p99 而不是简单计数时 |
| `quanta` | 高性能时间源 | latency metrics | 证明确认 `Instant` 采样成为热点时 |
| `metrics` | 指标 facade | KernelSnapshot export | 需要把内核 stats 输出到多后端时 |
| `arc-swap` | 热替换快照 | Snapshot / Policy table | 大量读、极少写的快照或策略表需要无锁读时 |

候选库选择原则：

- 优先成熟、下载量大、维护活跃、API 稳定的 crate。
- 只为内核通用问题引入，不为单个业务模块引入。
- 性能敏感路径优先使用第三方成熟实现，不手写队列、退避、直方图、generational arena。
- 若标准库已经足够且不在热点路径，不引入依赖。

### 4.4 不进入内核的库

| 库 | 说明 |
|----|------|
| `tower` | HTTP middleware，Gateway 可适配，Kernel Pipeline 不绑定 HTTP 语义 |
| `rusqlite` | 持久事实源，业务层通过 AuditSink trait 接入 |
| `anyhow` | 只在业务边界使用，内核内部优先 `thiserror` |

### 4.5 前端相关库

Kernel 在 Rust 后端，不依赖前端库。前端只消费 Kernel 产出的事实源投影：

| 前端库 | 用途 |
|--------|------|
| `solid-js` | 订阅状态、渲染 AgentTimelinePart / Task / Settings |
| `@tauri-apps/api` | IPC 和 Channel |
| `@kobalte/core` | Dialog / Menu / Select 等交互组件 |
| `CodeMirror` | Editor / Diff / LSP 展示 |
| `xterm` | Terminal 面板 |

前端不得直接实现 Kernel 行为，例如权限判断、工具进度伪造、扩展工具执行。

---

## 5. 原语一：Registry

### 5.1 定位

Registry 管”有什么能力可用”，不管能力怎么执行。

能力是业务层定义的。Registry 不关心注册的是什么，只关心它的 id、kind、version 和 lifecycle state。子系统各自持有自己的强类型 Registry 实例。

### 5.2 生命周期

```text
Registered -> Enabled -> Running -> Enabled -> Registered -> Removed
```

状态含义：

| 状态 | 含义 |
|------|------|
| `Registered` | 已注册但未启用 |
| `Enabled` | 已启用但未运行 |
| `Running` | 正在处理请求或监听事件 |
| `Removed` | 已注销 |

### 5.3 接口草案

Registry 是泛型容器，每个子系统持有自己的强类型实例。`Capability` trait 只用于统一描述，不用于统一调用。

查询和注册是同步操作（内存读写），只有涉及 I/O 的加载/卸载才是异步。

```rust
/// Capability trait：只提供元数据描述，不提供执行方法
/// 实际执行通过业务层各自的具体 trait
pub trait Capability: Send + Sync {
    fn id(&self) -> &str;
    fn kind(&self) -> &str;          // 业务层自定义："tool" / "provider" / "extension" / ...
    fn version(&self) -> SchemaVersion;
    fn metadata(&self) -> &Value;    // 业务层自定义扩展字段
}

pub struct CapabilityInfo {
    pub id: String,
    pub kind: String,
    pub version: SchemaVersion,
    pub state: LifecycleState,
    pub metadata: SharedArc<Value>,
}

/// 同步操作：内存索引查询和注册
pub trait Registry<T: Capability + ?Sized>: Send + Sync {
    fn register_arc(&self, item: Arc<T>) -> KernelResult<CapabilityId>;
    fn replace_arc(&self, item: Arc<T>) -> KernelResult<CapabilityId>;        // 保留已有 lifecycle state
    fn replace_arc_reset(&self, item: Arc<T>) -> KernelResult<CapabilityId>;  // 显式重置为 Registered
    fn unregister(&self, id: &str) -> KernelResult<()>;
    fn get(&self, id: &str) -> Option<Arc<T>>;              // 只返回 Enabled / Running
    fn get_registered(&self, id: &str) -> Option<Arc<T>>;   // 返回任意已注册状态
    fn info(&self, id: &str) -> Option<CapabilityInfo>;
    fn list(&self) -> Vec<CapabilityInfo>;
    fn objects(&self) -> Vec<KernelObjectInfo>;
    fn stats(&self) -> RegistryStats;
    fn is_registered(&self, id: &str) -> bool;
    fn is_available(&self, id: &str) -> bool;
    fn find(&self, predicate: &(dyn Fn(&CapabilityInfo) -> bool + Send + Sync)) -> Vec<CapabilityInfo>;
    fn lifecycle(&self, id: &str, action: LifecycleAction) -> KernelResult<LifecycleState>;
}

/// 可选异步生命周期：能力启停需要磁盘/网络 I/O 时实现
#[async_trait]
pub trait CapabilityLifecycle: Capability {
    async fn before_lifecycle(&self, action: LifecycleAction, ctx: &KernelContext) -> KernelResult<()> {
        Ok(())
    }

    async fn after_lifecycle(
        &self,
        action: LifecycleAction,
        state: LifecycleState,
        ctx: &KernelContext,
    ) -> KernelResult<()> {
        Ok(())
    }
}

#[async_trait]
pub trait AsyncRegistry<T: CapabilityLifecycle + ?Sized>: Registry<T> {
    async fn lifecycle_async(
        &self,
        id: &str,
        action: LifecycleAction,
        ctx: &KernelContext,
    ) -> KernelResult<LifecycleState>;
}

/// 可选加载边界：从 Extension manifest、磁盘目录或受控远端来源发现能力时使用
#[async_trait]
pub trait RegistryLoader<T: Capability + ?Sized>: Registry<T> {
    async fn load(&self, source: &str, ctx: &KernelContext) -> KernelResult<Vec<Arc<T>>>;
    async fn unload(&self, id: &str, ctx: &KernelContext) -> KernelResult<()>;
}
```

`get()` 是执行路径读取，只返回 `Enabled` / `Running` 能力；UI、配置面板、生命周期管理需要读取未启用能力时使用 `get_registered()` / `info()`。能力替换使用 `replace_arc()`，不要在业务层手写 `unregister()` + `register_arc()` 的组合流程。`replace_arc()` 保留已有生命周期状态；如果确实需要把能力重新变成 `Registered`，必须显式调用 `replace_arc_reset()`。

各子系统的强类型 Registry 实例：

```rust
// 具体实现是 InMemoryRegistry<T>（arc-swap 无锁读 + im::HashMap 结构共享写）
pub struct InMemoryRegistry<T: Capability + ?Sized> {
    entries: ArcSwap<im::HashMap<String, RegistryEntry<T>>>,
    write_lock: parking_lot::Mutex<()>,
}

impl<T: Capability + ?Sized> Registry<T> for InMemoryRegistry<T> { ... }

// 各子系统持有 Arc<InMemoryRegistry<T>> 或等价的领域门面。
// 命名必须表达宿主域，不新建“外层总线/桥接注册表”。
type McpToolCapabilityRegistry   = Arc<InMemoryRegistry<dyn MCPToolCapability>>;
type ProviderConfigRegistry      = Arc<InMemoryRegistry<ProviderConfig>>;
type TransportCapabilityRegistry = Arc<InMemoryRegistry<TransportAdapterCapability>>;

// 创建
let tool_capabilities: McpToolCapabilityRegistry = Arc::new(InMemoryRegistry::new());
let provider_configs: ProviderConfigRegistry = Arc::new(InMemoryRegistry::new());
```

如果能力启停需要 I/O（例如启动子进程或建立远端连接），能力实现 `CapabilityLifecycle`，调用方使用 `AsyncRegistry::lifecycle_async()`。动态资源由具体宿主负责创建和销毁，Kernel 不负责加载任意模块：

```rust
#[async_trait]
impl CapabilityLifecycle for MyCapability {
    async fn before_lifecycle(
        &self,
        action: LifecycleAction,
        ctx: &KernelContext,
    ) -> KernelResult<()> {
        if matches!(action, LifecycleAction::Start) {
            self.start(ctx).await?;
        }
        Ok(())
    }
}

capability_registry
    .lifecycle_async("some-capability", LifecycleAction::Start, &ctx)
    .await?;
```

调用路径分离：
- **查找/描述** → 通过 `Capability` trait 的 `descriptor()`，统一查询
- **执行** → 通过各自的具体执行边界，如 MCP Executor、Gateway Provider adapter、Agent control stage
- Registry 只管注册、发现、生命周期，不执行能力

### 5.4 实现细节

`InMemoryRegistry<T>` 使用 `arc_swap::ArcSwap` 实现无锁读，并用 `im::HashMap` 实现结构共享写路径：

```rust
pub struct InMemoryRegistry<T: Capability + ?Sized> {
    entries: ArcSwap<im::HashMap<String, RegistryEntry<T>>>, // 无锁读 + 结构共享写
    write_lock: parking_lot::Mutex<()>,                      // 写串行化 + fail-closed 判重
}
```

- **读路径**（`get` / `list` / `find` / `is_registered` / `is_available`）：`entries.load()` CAS 原子读，零锁零阻塞
- **写路径**（`register` / `unregister` / `lifecycle`）：`Mutex` 串行化 → `im::HashMap` 结构共享修改 → 原子 store
- **metadata 缓存**：`RegistryEntry` 构造时将 `metadata` 缓存为 `triomphe::Arc<Value>`，`info()` 只增加引用计数，不深拷贝 JSON
- **身份缓存**：`RegistryEntry` 缓存 id / kind / object kind / state 字符串，`info()` / `stats()` / `objects()` 不重复调用业务 trait 或 format
- **重复注册**：同 id 注册在写锁内基于当前快照判重，fail-closed，不覆盖

禁止：

- 用字符串到处临时查能力。
- 扩展绕过 Registry 直接改全局状态。
- Registry 内部直接执行能力。

---

## 6. 原语二：Pipeline

### 6.1 定位

Pipeline 管“怎么执行能力”。它是同步/异步链式执行，不是事件广播。

典型实例（业务层定义，非内核）：

- 能力执行 Pipeline（如 Agent Tool Pipeline）
- 请求处理 Pipeline（如 Gateway Request Pipeline）
- 消息传输 Pipeline（如 MCP Transport Pipeline）
- 事实记录 Pipeline（如 Review/Diff Pipeline）
- 钩子执行 Pipeline（如 Hook Pipeline）

### 6.2 接口草案

内核定义一个通用 Stage trait。业务层实现具体 Stage，通过 `PipelineContext.data` 传递业务数据。

```rust
/// 通用 Stage：内核不知道 Stage 做什么
#[async_trait]
pub trait Stage: Send + Sync {
    fn id(&self) -> &str;
    async fn process(&self, ctx: &mut PipelineContext, next: Next<'_>) -> KernelResult<()>;
}

/// Pipeline 上下文：内核只提供基础设施，业务数据由 data 承载
pub struct PipelineContext {
    /// 业务层注入的数据，内核不解释内容
    /// 业务层在自己的 Stage 内部 downcast 为具体类型
    data: Box<dyn PipelineData>,
    /// 通用内核上下文
    pub context: KernelContext,
    /// 通用取消信号
    pub cancellation: CancellationToken,
    /// Pipeline 执行生命周期观察 sink
    pub observations: ExecutionObservationSink,
    /// 通用审计记录器
    pub audit: AuditRecorder,
    /// 当前 run 的唯一标识（等于 context.trace_id）
    /// 使用 Arc<str> 避免 observation 热路径上反复 String clone
    run_id: Arc<str>,
    /// 观察事件的递增序列号
    next_observation_sequence: u64,
}

pub struct Pipeline {
    stages: Vec<Arc<dyn Stage>>,
    stage_ids: Vec<String>,
    stage_id_set: HashSet<String>,
}

impl Pipeline {
    pub fn new() -> Self;
    pub fn push(&mut self, stage: impl Stage + 'static);
    pub fn push_arc(&mut self, stage: Arc<dyn Stage>);
    pub fn require_stage(self, stage_id: impl Into<String>) -> Self;
    pub async fn run(&self, ctx: &mut PipelineContext) -> KernelResult<()>;
    pub fn run_blocking(&self, ctx: &mut PipelineContext) -> KernelResult<()>;
}

/// Next：单次 continuation，只暴露顺序执行，不允许跳过或回跳
pub struct Next<'a> {
    stages: &'a [Arc<dyn Stage>],
    next_index: usize,
}

impl<'a> Next<'a> {
    pub fn call(self, ctx: &'a mut PipelineContext) -> BoxFuture<'a, KernelResult<()>>;
}
```

Stage 只能调用 `next.call(ctx)` 推进到下一个 Stage，不能跳过、不能反向调用。`Next` 不实现 `Clone` / `Copy`，因此同一个 continuation 只能消费一次；Stage 可以选择短路，但不能重复执行后续链。

Pipeline 在 `push()` 时缓存 stage id 和 stage id set；`stage_ids()`、`objects()`、`validate_required_stages()` 不在每次 run 中重复遍历并分配。

Pipeline 不提供“回跳到某个 Stage”的 API。回跳会让 side effect、audit、resource lease、event emit 的 exactly-once 语义变复杂，属于工作流 / 状态机 / 编排层能力，不属于内核线性 Pipeline。需要重新执行时有两个边界：

- 当前或整体执行失败后的有界重试：使用 `run_with_retry()` / `run_with_retry_if()`，由 `backon` 提供指数退避和 jitter；默认只重试 `KernelError::is_retryable()` 为 true 的错误。
- 单个 Stage 内部的幂等重试：在业务 Stage 内用成熟 retry/backoff 策略包裹该 Stage 自己的 I/O，不改变 Pipeline 游标。

同步 API 边界必须使用 `Pipeline::run_blocking()`，不要在应用模块里散落手写 executor 或 `futures::executor::block_on(pipeline.run(...))`。异步边界必须使用 `run().await`。

`run_blocking()` 只允许出现在真实同步入口：CLI 命令、同步测试、非 Tokio 回调、或必须暴露给同步 trait 的兼容边界。以下位置禁止调用 `run_blocking()` 或手写 `block_on()`：

- Tauri async command
- Agent turn loop / tool loop
- Gateway provider request / stream
- MCP async transport
- 后台 task / sidechain 的 async runner

这些路径已经处在 Tokio runtime 内，必须 async 端到端进入 Kernel Pipeline。同步兼容 wrapper 可以保留，但只能作为外部同步 API 的薄壳，不能被 async 主路径反向调用。

取消语义同样属于 Pipeline 生命周期。用户点击 Stop、Task 取消、超时和审批拒绝必须映射到同一条执行链的 `PipelineContext.cancellation` 或等价的上层 cancel token；UI 事实源（例如 `agent_timeline_parts`）必须在取消发生时写入终态 `aborted`，再发布事件或 stream chunk。只取消前端 stream 而不关闭后端事实源，会造成 running step、计时器和审计状态泄漏。

**关于 `PipelineData` trait（`downcast-rs`）：**

内核使用 `downcast-rs` 替代裸 `Box<dyn Any>`，提供类型安全的 downcast 和诊断信息：

```rust
pub trait PipelineData: DowncastSync + Send {
    fn type_name(&self) -> &'static str;
}
impl_downcast!(sync PipelineData);

pub struct PipelineContext {
    data: Box<dyn PipelineData>,  // 类型安全 downcast
    // ...
}
```

优势：
- downcast 失败时错误信息包含**实际类型名和期望类型名**
- `DowncastSync` trait 在编译期约束 data 类型
- `data::<T>()` / `data_mut::<T>()` 返回 `KernelResult`，不 panic

**关于重试机制（`backon`）：**

Pipeline 原生支持重试，基于 `backon` 库实现指数退避：

```rust
pub struct PipelineRetryPolicy {
    pub max_retries: usize,       // 默认 3
    pub min_delay: Duration,      // 默认 100ms
    pub max_delay: Duration,      // 默认 5s
    pub jitter: bool,         // 默认 true
}

impl Pipeline {
    pub async fn run_with_retry(&self, ctx: &mut PipelineContext, policy: PipelineRetryPolicy) -> KernelResult<()>;
    pub async fn run_with_retry_if(&self, ctx: &mut PipelineContext, policy: PipelineRetryPolicy, should_retry: impl FnMut(&KernelError) -> bool) -> KernelResult<()>;
    pub fn run_with_retry_blocking(&self, ctx: &mut PipelineContext, policy: PipelineRetryPolicy) -> KernelResult<()>;
    pub fn run_with_retry_if_blocking(&self, ctx: &mut PipelineContext, policy: PipelineRetryPolicy, should_retry: impl FnMut(&KernelError) -> bool) -> KernelResult<()>;
}
```

业务层可按错误类型选择性重试。`StageFailed` 会保留原始 `KernelErrorKind`，所以 `PolicyDenied` / `PolicyRequiresApproval` / version mismatch 等被 Stage 包装后仍不会被默认重试；工具网络抖动、临时资源失败等应映射为 `KernelError::transient(...)` 或其他 retryable kind。

**业务层 typed adapter 规则：**

`Box<dyn Any>` 只在内核 Pipeline 内部暴露。业务层**必须**定义 typed adapter，禁止扩展直接接触 `Any`：

```rust
// ✅ 正确：业务层定义 typed adapter
pub struct ToolPipelineData {
    pub request: CapabilityRequest,
    pub result: Option<CapabilityResult>,
}

pub struct AgentTurnPipelineData {
    pub turn_id: String,
    pub messages: Vec<Message>,
    pub active_skill: Option<SkillDefinition>,
}

pub struct GatewayPipelineData {
    pub request: ProviderChatRequest,
    pub response: Option<ChatResponse>,
    pub stream: Option<StreamReceiver>,
}

// ✅ 正确：Stage 内部通过 typed adapter 访问
impl Stage for ExecuteCapabilityStage {
    async fn process(&self, ctx: &mut PipelineContext, next: Next<'_>) -> Result<()> {
        let data = ctx.data.downcast_ref::<ToolPipelineData>()
            .ok_or(KernelError::PayloadTypeMismatch)?;
        // ...
    }
}

// ❌ 禁止：扩展直接操作 Box<dyn Any>
// ❌ 禁止：Stage 内部 match ctx.data.type_id()
```

每个 Pipeline 实例必须有且只有一个 typed adapter。Stage 和 Pipeline 不匹配导致的 downcast 失败是编译/测试期错误，不是运行时降级。

**通用 Stage（跨 Pipeline 复用）只访问 PipelineContext 的通用字段：**

```rust
/// Policy Stage：只访问 trace_id 和 data（取出 subject/action/target）
struct PolicyCheckStage { engine: Arc<PolicyEngine> }

/// Audit Stage：只访问 trace_id 和 audit 记录器
struct AuditStage;

/// Cancellation Stage：只检查 cancellation token
struct CancellationGuardStage;
```

**业务 Stage 访问 data：**

```rust
/// Tool 执行 Stage：downcast data 为 ToolRequest
struct ExecuteToolStage;

impl Stage for ExecuteToolStage {
    fn id(&self) -> &str { "execute_tool" }
    async fn process(&self, ctx: &mut PipelineContext, next: Next<'_>) -> KernelResult<()> {
        let req = ctx.data.downcast_ref::<ToolRequest>()
            .ok_or(KernelError::PayloadTypeMismatch)?;
        // 执行工具...
        next.call().await
    }
}
```

### 6.3 标准 Stage（业务层参考，非内核定义）

以下 Stage 是业务层推荐实现，内核不定义也不感知：

Agent Tool Pipeline 推荐阶段：

```text
PolicyCheckStage        — 从 data 取出 subject/action/target，调用 Policy
BeforeHookStage         — 执行业务层注册的前置 Hook
ExecuteToolStage        — downcast data，执行具体工具
WriteAgentTimelinePartStage     — 写入业务事实源（AgentTimelinePart）；如果已有外层 turn loop 统一写入，则不要重复写
ObserveToolResultStage  — 可选，只写 tracing 观察日志，不是事实源
AfterHookStage          — 执行业务层注册的后置 Hook
AuditStage              — 记录执行摘要到 audit_log
```

Gateway Pipeline 推荐阶段：

```text
AssembleRequestStage
TokenBudgetStage
CompactionStage
RateLimitStage
ProviderCallStage
ParseProviderStreamStage
RetryStage
UsageAccountingStage
```

MCP Transport Pipeline 推荐阶段：

```text
ResolveServerStage
SerializeJsonRpcStage
TransportSendStage
ReceiveResponseStage
DeserializeJsonRpcStage
MapToolResultStage
```

### 6.4 关键规则

- Pipeline 是唯一可插 Hook 的地方。
- 扩展 Hook 本质是 Stage，不是独立执行系统。
- ExecutionObservation 事件必须由后端真实 stage 产生，前端不能伪造。
- Pipeline 输出必须能映射到事实源或错误。

### 6.5 Pipeline 执行观察（ExecutionObservation）

Pipeline 执行过程中的生命周期事件通过 `ExecutionObservationSink` 程序化消费。这不同于 tracing span（面向日志/追踪），而是面向程序化消费者（UI 投影、指标采集、运行时诊断）。

```rust
/// Pipeline 执行生命周期事件类型
pub enum ExecutionEventKind {
    RunStarted,
    RunCompleted,
    RunFailed,
    RunCancelled,
    StageStarted,
    StageDelta,          // Stage 内部增量数据（如流式输出、中间结果）
    StageCompleted,
    StageFailed,
    CapabilityCalled,
    CapabilityCompleted,
    CapabilityFailed,
}

/// 单次执行观察事件
pub struct ExecutionEvent {
    pub id: String,                        // uuid::now_v7()
    pub kind: ExecutionEventKind,
    pub run_id: String,                    // 对应 PipelineContext.run_id（即 trace_id）
    pub stage_id: Option<String>,
    pub capability_id: Option<String>,
    pub context: KernelContext,
    pub sequence: u64,                     // 同一 run 内递增序列号
    pub message: Option<String>,
    pub error_kind: Option<KernelErrorKind>,
    pub duration_ms: Option<u64>,
    pub payload: SharedArc<Value>,         // triomphe::Arc，默认使用 OnceLock 共享的 Value::Null 单例
    pub created_at: DateTime<Utc>,
}

/// 观察 sink：业务层注入消费回调
pub struct ExecutionObservationSink {
    observer: Arc<dyn Fn(SharedExecutionEvent) + Send + Sync>,
}

impl ExecutionObservationSink {
    pub fn disabled() -> Self;
    pub fn new(observer: ExecutionObserver) -> Self;
    pub fn from_fn(observer: impl Fn(SharedExecutionEvent) + Send + Sync + 'static) -> Self;
    pub fn event_bus(event_bus: Arc<dyn EventBus>) -> Self;
    pub fn observe(&self, event: ExecutionEvent);
    pub fn is_enabled(&self) -> bool;
}
```

**与 tracing 的分工：**

| 通道 | 载体 | 消费者 | 特点 |
|------|------|--------|------|
| tracing span/event | `tracing_subscriber::Registry` + Layer | 日志文件、console、审计日志 | 非结构化、面向人类阅读和日志持久化 |
| ExecutionObservationSink | `Arc<dyn Fn>` 回调 | UI 投影、指标采集、运行时诊断 | 结构化、面向程序化消费、零拷贝 |

两通道独立，互为补充。Pipeline 的 `run()` 同时产生 tracing span 和 `ExecutionEvent`，但不强制两者语义完全对齐。`ExecutionObservationSink::disabled()` 是真正的低开销路径：未启用观察时不构造 `ExecutionEvent`。

**EventBus 投影：**

`ExecutionObservationSink::event_bus(event_bus)` 会把 `ExecutionEvent` 包装成 `EventEnvelope`，topic 来自 `ExecutionEventKind::topic()`：

```text
execution.run.started
execution.run.completed
execution.run.failed
execution.run.cancelled
execution.stage.started
execution.stage.delta
execution.stage.completed
execution.stage.failed
execution.capability.called
execution.capability.completed
execution.capability.failed
```

这不是新的 EventBus，也不是 UI stream。它只是把 Pipeline 的结构化执行事实投影到现有 EventBus，应用/UI 可以再投影成 timeline、状态面板或诊断视图。

**OpenTelemetry 边界：**

当前不引入 OpenTelemetry。Kernel 只产出 `tracing` span/event、`ExecutionEvent`、Audit 和 Snapshot。若未来出现跨进程/跨服务 trace、企业观测平台或 OTLP/Jaeger/Tempo 接入需求，应在 `foundation/telemetry` 或 `foundation/logger` 作为可选 `tracing_subscriber` Layer 接入，不修改 Kernel 语义。

**PipelineContext 便利方法：**

```rust
impl PipelineContext {
    /// 发出自定义观察事件
    pub fn observe(&mut self, kind: ExecutionEventKind);

    /// 发出 Stage 增量数据（流式输出、中间结果等）
    pub fn observe_stage_delta(
        &mut self,
        stage_id: impl Into<String>,
        message: impl Into<String>,
        payload: Value,
    );

    /// 构造时注入观察 sink
    pub fn with_observations(self, observations: ExecutionObservationSink) -> Self;

    /// 构造时注入简单回调（不需要关心完整事件语义时使用）
    pub fn with_observer_fn(
        self,
        observer: impl Fn(SharedExecutionEvent) + Send + Sync + 'static,
    ) -> Self;
}
```

**默认行为：** `PipelineContext::new()` 默认使用 `ExecutionObservationSink::disabled()`，不产生运行时开销。业务层通过 `with_observations()` 或 `with_observer_fn()` 显式注入消费者。

**设计意图：** `ProgressCallback` 只服务"进度百分比"单一语义，但 Pipeline 执行过程中消费者关心的信息远不止进度——run 开始/结束、stage 生命周期、capability 调用链、错误分类、增量数据都是执行观察的一部分。`ExecutionObservationSink` 用统一的观察模型替代单一的进度回调，避免为每种观察需求新增独立 callback 字段。

---

## 7. 原语三：Event Bus

### 7.1 定位

Event Bus 管”发生了什么，谁需要知道”。它是异步通知，不参与结果传递。

### 7.2 事件结构

```rust
struct EventEnvelope {
    id: String,                  // uuid::now_v7() 时序有序
    topic: Topic,                // 业务层定义，newtype 包装 String
    version: SchemaVersion,      // 事件 payload 版本
    context: KernelContext,       // 包含 scope、source、trace_id 等
    payload: Option<SharedArc<Value>>,  // 业务层定义结构；None 表示无负载，避免空 JSON 分配
    created_at: DateTime<Utc>,
}
```

`EventEnvelope` 通过 `triomphe::Arc` 共享，clone 只增加引用计数，不深拷贝 payload。
`payload` 为 `Option` 类型：大多数事件不需要携带完整 JSON 负载（通知语义），`None` 避免了空 `BTreeMap` 分配。需要携带负载时用 `Some(SharedArc::new(json!({...})))`。

### 7.3 接口

```rust
pub trait EventBus: Send + Sync {
    fn emit(&self, envelope: EventEnvelope) -> KernelResult<()>;
    fn subscribe(&self, topic: Option<Topic>, scope_key: Option<String>, handler: EventHandler) -> KernelResult<SubscriptionId>;
    fn subscribe_async(&self, topic: Option<Topic>, scope_key: Option<String>, handler: AsyncEventHandler) -> KernelResult<SubscriptionId>;
    fn unsubscribe(&self, id: &SubscriptionId) -> KernelResult<()>;
    fn recent(&self, limit: usize) -> Vec<SharedEventEnvelope>;
    fn stats(&self) -> EventBusStats;
    fn objects(&self) -> Vec<KernelObjectInfo>;
}
```

- `emit()` 写入 history ring-buffer + 入 flume channel（O(1)，非阻塞）。队列满时记录 `overflow_count` 并 warn 日志，事件仍保留在 history 中供 `recent()` 查询，不阻断调用方
- `subscribe()` 按 topic（支持 glob 通配符）+ scope_key 过滤，同步 handler 进入 Tokio blocking pool
- `subscribe_async()` 接收 `SharedEventEnvelope`，异步 handler 进入 Tokio runtime
- `unsubscribe()` 找不到订阅时返回 `EventSubscriptionNotFound`，不静默吞错
- `recent()` 返回 `SharedEventEnvelope`（`triomphe::Arc`），零 payload 拷贝
- `stats()` 返回订阅数、history 长度、队列长度、dispatcher 状态和 `overflow_count`，用于健康检查
- `stats().has_overflow()` 便捷方法检查是否发生过事件溢出
- 订阅变更时刷新 `Arc<Vec<EventSubscription>>` 快照，dispatcher 每个事件只 clone 快照指针，不扫描和 clone 订阅 HashMap
- `shutdown()` 后 `emit()` / `subscribe()` 都 fail-closed，不允许静默重启已关闭资源
- history 保留可查询 ring-buffer 语义；不使用 `crossbeam::ArrayQueue` 替代，因为它不提供无破坏性 recent snapshot

### 7.4 线程模型

```text
emit() → history write → flume try_send → 返回
                          │
                          ▼
                dispatcher thread (×1, 懒启动)
                rx.recv() → subscriptions 匹配
                              ├─ sync handler  → Handle::spawn_blocking(handler)
                              └─ async handler → Handle::spawn(async handler)
```

**`InMemoryEventBus` 构造要求：**

```rust
// 必须显式传入 Tokio runtime Handle
let bus = InMemoryEventBus::new(1000, tokio::runtime::Handle::current());
```

- 构造时传入 `Handle`，不搞隐式 `try_current()` 兜底
- dispatcher 线程在首次 `subscribe()` 时懒启动
- 无订阅者时 `emit()` 只写 history，不启动 dispatcher
- 同步 handler 在 `spawn_blocking` 中执行，异步 handler 在 `spawn` 中执行
- handler panic 通过 `catch_unwind` 隔离，不影响 dispatcher
- `shutdown()` drop channel + join dispatcher thread（不等待已提交的 sync/async handler 任务）

### 7.5 事件分类

事件 topic 和 payload 由业务层定义，内核不感知。内核只提供 topic 匹配和 scope 隔离机制。

示例（业务层定义，非内核）：

| topic | 来源 | 说明 |
|-------|------|------|
| `action.completed` | 业务层 | 能力执行完成 |
| `config.changed` | 业务层 | 配置变更 |
| `health.check` | 业务层 | 健康检查结果 |
| `session.created` | 业务层 | 会话创建 |

内核只关心：topic 是字符串、scope 是字符串、payload 是 JSON 值。

### 7.6 禁止事项

- 禁止用 Event Bus 传父子 Task 结果。
- 禁止用 Event Bus 代替数据库。
- 禁止消费者靠事件顺序修复事实源。
- 禁止跨 session 订阅泄漏。

---

## 8. 原语四：Policy

### 8.1 定位

Policy 管”是否允许一个操作”。它是通用约束求解，内核不关心 subject/action/target 是什么。

业务层定义约束语义，内核只负责按优先级评估所有约束并返回决策。

### 8.2 接口草案

```rust
pub struct PolicyInput {
    pub subject: String,    // 业务层定义：”agent” / “extension:x” / “user”
    pub action: String,     // 业务层定义：”capability.execute” / “data.write” / “command.execute”
    pub target: String,     // 业务层定义：”/path/to/file” / “bash” / “openai:gpt-4o”
    pub scope: String,      // 业务层定义：”session:xxx” / “global” / “task:xxx”
    pub metadata: Value,    // 业务层自定义扩展
}

pub enum PolicyDecision {
    Allow { reason: String },
    Ask { prompt: String, grant_spec: Value },  // grant_spec 由业务层定义结构
    Deny { reason: String },
}

/// Policy 相关错误子枚举，收敛 KernelError 中 5 个 Policy 变体
pub enum PolicyErrorKind {
    Denied,
    RequiresApproval,
    Undecidable,
    ConstraintNotFound,
    ConstraintAlreadyRegistered,
}
// KernelError::Policy { kind: PolicyErrorKind, detail: String }
// 便捷构造器：policy_denied(), policy_requires_approval(), policy_undecidable(),
//             policy_constraint_not_found(), policy_constraint_already_registered()

/// 约束 trait：业务层实现
pub trait Constraint: Send + Sync {
    fn id(&self) -> &str;
    fn priority(&self) -> i32;  // 数字越小优先级越高
    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision>;
    // 返回 None 表示此约束不适用（交给下一个约束）
    // 返回 Some 表示此约束命中（Allow / Ask / Deny）
}

pub struct ConstraintInfo {
    pub id: String,
    pub priority: i32,
}

pub struct PolicyEngine {
    constraints: RwLock<Vec<Arc<dyn Constraint>>>,
}

impl PolicyEngine {
    pub fn new() -> Self;
    pub fn add(&self, constraint: impl Constraint + 'static) -> KernelResult<()>;
    pub fn add_arc(&self, constraint: Arc<dyn Constraint>) -> KernelResult<()>;
    pub fn replace(&self, constraint: impl Constraint + 'static) -> KernelResult<()>;
    pub fn replace_arc(&self, constraint: Arc<dyn Constraint>) -> KernelResult<()>;
    pub fn remove(&self, constraint_id: &str) -> KernelResult<()>;
    pub fn contains(&self, constraint_id: &str) -> bool;
    pub fn list(&self) -> Vec<ConstraintInfo>;
    pub fn objects(&self) -> Vec<KernelObjectInfo>;
    pub fn stats(&self) -> PolicyStats;
    
    pub fn evaluate(&self, input: &PolicyInput) -> PolicyDecision {
        // 按 priority 排序后逐个评估
        // 第一个返回 Some 的约束决定结果
        // 无约束命中 → Deny（fail-closed）
    }
}
```

PolicyEngine 是可共享的内核服务，调用方持有 `Arc<PolicyEngine>` 即可；不需要在外层再维护一份 constraint HashMap。热更新策略使用 `replace()` / `replace_arc()`，撤销策略使用 `remove()`。
`add()` / `add_arc()` 遇到重复 constraint id 必须 fail-closed；重复 id 的热更新只能用 `replace()` / `replace_arc()`，避免影子规则留在策略链里。

### 8.3 约束优先级

内核只按 `Constraint.priority()` 数字排序，不理解约束的业务含义。

业务层推荐的优先级分配：

| 优先级范围 | 业务层用途 | 示例 |
|-----------|-----------|------|
| 0-99 | 硬性禁止规则（不可覆盖） | 禁止访问 /etc/shadow |
| 100-199 | 企业管控规则 | 管理员策略 |
| 200-299 | 用户全局规则 | 用户 settings |
| 300-399 | 项目规则 | .navis/settings.json |
| 400-499 | 会话级授权 | 用户本次确认后由业务层生成的 grant Constraint |
| 500-599 | 模式约束 | Plan Mode 禁写 |
| 600+ | 默认策略 | 当前模式的默认行为 |

默认：无约束命中 → `Deny`。

### 8.4 策略来源与合并规则

Policy 约束来自多个来源，按加载顺序和优先级合并：

```rust
/// 策略来源
pub struct PolicySource {
    pub id: String,              // 来源标识
    pub kind: String,            // "kernel" / "enterprise" / "user" / "project" / "session" / "grant"
    pub constraints: Vec<Box<dyn Constraint>>,
    pub is_active: bool,
}

/// PolicySet：管理所有策略来源的加载和合并
pub struct PolicySet {
    sources: RwLock<Vec<PolicySource>>,
}
```

#### 加载顺序（从先到后）

```
启动时加载：
  ① Kernel 内置约束（priority 0-99，不可覆盖）
  ② 企业管控约束（priority 100-199，从 managed config 加载）
  ③ 用户全局约束（priority 200-299，从 ~/.navis/settings.json 加载）
  ④ 项目约束（priority 300-399，从 .navis/settings.json 加载）

运行时动态添加：
  ⑤ 会话级授权（priority 400-499，用户确认后由业务层写入 Constraint）
  ⑥ 模式约束（priority 500-599，Plan Mode / FullAuto 等）
  ⑦ 扩展约束（priority 由扩展 manifest 声明）
  ⑧ 默认策略（priority 600+，兜底）
```

#### 合并规则

| 规则 | 说明 |
|------|------|
| **priority 越小越优先** | priority=10 的硬性禁止优先于 priority=400 的会话授权 |
| **同 priority 内，Deny > Ask > Allow** | 同优先级内拒绝优先于询问优先于允许 |
| **Deny 不可被低优先级覆盖** | priority=10 的 Deny 不能被 priority=400 的 Allow 覆盖 |
| **Allow 可被高优先级 Deny 覆盖** | priority=400 的 Allow 可被 priority=10 的 Deny 覆盖 |
| **首个命中即返回** | 按 priority 排序后，第一个返回 Some 的约束决定结果 |
| **无命中即 Deny** | 所有约束都返回 None 时，默认 Deny（fail-closed） |

#### 策略来源的生命周期

```
① Kernel 约束：启动时加载，永不卸载
② 企业约束：启动时加载，配置热更新时刷新
③ 用户约束：启动时加载，用户修改设置时刷新
④ 项目约束：项目切换时重新加载
⑤ 会话授权：会话开始时清空，用户确认后由业务层转换为 Constraint 动态添加，会话结束时丢弃
⑥ 模式约束：模式切换时替换
⑦ 扩展约束：扩展安装时添加，扩展卸载时移除
⑧ 默认策略：启动时加载，永不卸载
```

#### 冲突处理示例

```
场景：用户在 Plan Mode 下尝试写文件

约束 ① Kernel: Deny file.write to /etc/**         → 不匹配（目标不在 /etc）
约束 ⑤ Session: Allow file.write to /worktree/**  → 命中 → Allow
约束 ⑥ Mode: Deny file.write to * (Plan Mode)     → 命中 → Deny

合并：priority(500) < priority(400)，Mode 约束优先
结果：Deny（Plan Mode 下禁止写操作，即使会话级授权允许）
```

### 8.5 业务层审批映射（非内核定义）

Kernel 不定义 UI 弹窗、菜单、按钮或 prompt matrix。UI 只能产生用户审批证据；最终是否可执行仍由业务层把审批证据转换成一次性 / 会话 / 项目 Constraint 后交给 `PolicyEngine` 评估。任何 `allow_*` 都不是绕过 Policy 的 bypass flag。

当前 UI 审批决策到 Policy grant 的映射：

| UI 决策 | Policy grant（业务层 Constraint 实现） |
|---------|--------------------------------------|
| `allow_once` | 当前 call id 一次性授权 |
| `allow_session` | scope 级授权（带 action + target pattern） |
| `allow_project` | 项目级授权（带 action + target pattern） |
| `deny_always` | 写入高优先级 deny Constraint |

grant 记录必须包含 action（做什么）和 target pattern（对什么），不能只按 target 缓存。

Settings 中的 tool permission matrix 属于 UI approval prompt policy：它只决定 UI 是否静默附加审批证据、是否弹出确认、是否拒绝生成审批证据。它不是内核原语，也不是最终授权事实源。高优先级 hardline deny、Sandbox Constraint、模式 Constraint 和扩展 Constraint 仍可覆盖 UI prompt policy 产生的 approval evidence。

---

## 9. 内核之上的子系统

子系统不是内核，但必须使用内核。

| 子系统 | Registry | Pipeline | Policy | Event |
|--------|----------|----------|--------|-------|
| Agent | Mode/Task registry | Agent turn pipeline | Mode/Task policy | AgentTimelinePart/Task event |
| Agent Tool Chain (`tool/agent`) | Tool Catalog consumes MCP Tool Registry | Agent Tool Pipeline | Tool permission policy | Tool/AgentTimelinePart event |
| Gateway | ProtocolAdapterRegistry + Provider/Model 路由索引 | GatewayPipeline | Quota/Rate/Trust policy | Usage/Error event |
| MCP | TransportRegistry + ToolRegistry | MCPTransportPipeline | Network/Extension policy | Server/Tool event |
| Extension | ExtensionStore 只存安装状态/声明；贡献进入各宿主 Registry | HookPipeline / Agent Tool Pipeline | Extension sandbox policy | Extension lifecycle event |
| UI | Host view declarations；Renderer 由 UI 域白名单承接 | UI command pipeline | UI command policy | View refresh event |
| Storage | Storage service / schema DTO index | Migration pipeline | Data access policy | Storage event |

---

## 10. 与 Navis Go 核心概念的关系

内核不吞掉产品概念，但所有产品概念都通过内核原语工作。以下是每个概念和内核的精确关系。

### 10.0 一等概念

这些概念是 Navis Go 的核心产品实体，每个都有明确的内核接入方式：

| 概念 | 说明 | 内核接入方式 |
|------|------|------------|
| **Tool** | 模型可调用的能力单元（含内置和外部接入） | MCP ToolDefinition/能力实例注册到 Kernel Registry；Agent 通过 Tool Catalog 投影 provider-safe 名称，通过 Pipeline 执行，受 Policy 管控，执行结果发布到 Event Bus |
| **Provider** | LLM 连接实例，包含 ProviderConfig 与 ModelConfig | Gateway 通过 ProtocolAdapterRegistry 解析 Model.api_protocol；Provider 不作为 Kernel 原语，请求通过 Gateway Pipeline 执行，配额受 Policy 管控 |
| **Extension** | 安装状态、manifest 和宿主 contribution 事务 | Extension lifecycle 通过各宿主 capability port 登记/释放，生命周期事实通过 Event Bus 通知 |
| **Skill** | 提示词包 + 工具白名单 + 匹配规则 | 存入 `SkillStore`（非 Registry），通过 Pipeline 中的 SkillMatchStage 激活，受 Policy 管控 |
| **Task** | 任务事实（用户 turn / 子任务 / 后台任务） | 存入 `TaskManager`（非 Registry），通过 Pipeline 执行，状态变更发布到 Event Bus，权限受 Policy 管控 |
| **AgentTimelinePart** | Agent 执行过程的只读事实记录 | 写入 Storage（事实源），状态变更通过 Event Bus 通知 UI |
| **Memory** | 持久记忆事实源 | 写入 Storage，通过 Tool（memory.recall / memory.store）读写 |
| **Session** | 对话会话 | 存入 SessionManager，生命周期事件通过 Event Bus 发布 |

### 10.0.1 应用域一等服务，不进入 Kernel

| 概念 | 原因 | 现状 |
|------|------|------|
| **Extension** | Extension 安装、生命周期、贡献声明和 hook 登记属于 `extension` 大域，不能被 Kernel 吞并 | `extension::{store,lifecycle,loader,installer,models}` 是应用服务；它通过 Kernel EventBus/Policy/Registry 风格接口接入宿主能力，但不成为内核原语 |
| **contributes** | contributes 是 Extension manifest 的声明语言，不是 Kernel 类型 | 由 `ExtensionLifecycle` 解析后投影到 Tool、Provider、UI、Skills、MCP 等宿主子系统 |
| **UI Host View Renderer** | `host:*` renderer 是 UI Framework 的内置视图渲染策略，不是 Kernel 类型，也不是完整 Extension runtime | 扩展 view 通过 `contributes.views + commands + menus` 声明；UI Host 解释 placement、renderer 和 config 并渲染完整界面。新增 renderer 不修改 Kernel；即使未来某个 renderer runtime 需要安装、启停或替换，也只是 UI 域复用通用 capability 生命周期，不要求 Kernel 增加 renderer 概念；Kernel 仍不感知 DOM、菜单、布局、placement 或 renderer 字符串 |
| **MCP** | MCP 是 `tool` 大域的协议引擎和外部工具接入层，不是 Kernel 原语 | `tool/mcp` 负责 transport、server discovery、tools/list、tools/call；Agent 侧通过 Tool Catalog 消费，不直接依赖裸 MCPTool |
| **Sandbox** | Sandbox 是 `security` 大域的执行约束服务，不是 Kernel Policy 的替代品 | Sandbox constraints 注册进 Policy，由 Policy 做统一决策；Sandbox 模块保留路径、命令、网络、资源等运行时约束能力 |

### 10.0.2 概念和内核原语的完整映射

```
概念              Registry    Pipeline    Event Bus    Policy    Storage
──────────────────────────────────────────────────────────────────────────
Tool              ✅ 注册     ✅ 执行      ✅ 通知      ✅ 管控
Provider          ✅ 注册     ✅ 执行      ✅ 通知      ✅ 管控
Extension         ✅ 注册                 ✅ 订阅更新
UI Host Renderer  UI 域内部承接           UI 域解释 placement/render，不进 Kernel 语义
Skill                     ✅ 匹配激活     ✅ 通知
Task                      ✅ 执行         ✅ 通知      ✅ 管控    ✅ 持久化
AgentTimelinePart                              ✅ 通知                   ✅ 持久化
Memory                                                     ✅ 管控  ✅ 持久化
Session                                  ✅ 通知                   ✅ 持久化
Transport         ✅ 注册传输  ✅ 传输执行  ✅ 通知      ✅ 管控
```

---

## 10.1 业务层如何消费内核

内核提供机制，业务层定义语义。以下示例展示业务层如何用内核的四个原语构建具体功能。

### 10.1.1 业务层定义具体 trait

```rust
// ═══════════════════════════════════════════
// 这些 trait 定义在业务层，不在内核
// ═══════════════════════════════════════════

/// MCP 工具能力（业务层定义）
pub trait MCPToolCapability: Capability {
    fn definition(&self) -> &ToolDefinition;
    fn builtin_tool(&self) -> Option<Arc<dyn MCPTool>>;
}

/// Gateway 的协议适配器（Gateway 业务层定义）
pub trait ProviderAdapter: Send + Sync {
    fn transform_request(&self, request: &ChatRequest, model: &ModelConfig) -> Result<Value>;
    fn transform_response(&self, raw: &Value, model: &ModelConfig) -> Result<ChatResponse>;
    fn transform_stream_chunk(&self, raw: &Value, model: &ModelConfig) -> Result<Option<StreamChunk>>;
}

// UI View 不是 Kernel capability。Extension 只声明 contributes.views，
// 由 UI HostView 宿主读取并投影为受控视图，不把渲染 trait 注册进 Kernel。
```

### 10.1.2 业务层创建具体 Registry 实例

```rust
// ═══════════════════════════════════════════
// 子系统各自持有自己的 Registry，内核不知道这些实例的存在
// ═══════════════════════════════════════════

let mcp_tool_registry: Registry<dyn MCPToolCapability> = Registry::new();
let provider_config_registry: Registry<ProviderConfig> = Registry::new();

// 注册内置工具（业务层代码）
mcp_tool_registry.register(FsReadToolCapability { ... });
mcp_tool_registry.register(TerminalToolCapability { ... });

// Extension 的 tools contribution 先由 Extension lifecycle 校验并转换为
// MCP ToolDefinition/能力实例，再通过 MCP capability port 登记到宿主 Registry。
// manifest 不直接加载任意实现，也不把加载细节暴露给 Kernel。

// 查找工具（消费侧）
let tool = mcp_tool_registry.get("fs.read_file").ok_or("not found")?;
```

### 10.1.3 业务层定义具体 Stage

```rust
// ═══════════════════════════════════════════
// Stage 实现在业务层，内核只提供 Stage trait
// ═══════════════════════════════════════════

/// 权限检查 Stage（业务层实现）
struct PolicyCheckStage { engine: Arc<PolicyEngine> }

impl Stage for PolicyCheckStage {
    fn id(&self) -> &str { "policy_check" }
    async fn process(&self, ctx: &mut PipelineContext, next: Next<'_>) -> Result<()> {
        let req = ctx.data.downcast_ref::<ToolRequest>()
            .ok_or(Error::PayloadTypeMismatch)?;
        let decision = self.engine.evaluate(&PolicyInput {
            subject: "agent".into(),
            action: "capability.execute".into(),
            target: req.capability_id.clone(),
            scope: format!("session:{}", req.session_id),
            metadata: json!({}),
        });
        match decision {
            PolicyDecision::Allow { .. } => next.call().await,
            PolicyDecision::Deny { reason } => Err(Error::Denied(reason)),
            PolicyDecision::Ask { .. } => { /* 暂停等待用户确认 */ }
        }
    }
}

/// 工具执行 Stage（业务层实现）
struct ExecuteToolStage { mcp: Arc<MCP> }

impl Stage for ExecuteToolStage {
    fn id(&self) -> &str { "execute_capability" }
    async fn process(&self, ctx: &mut PipelineContext, next: Next<'_>) -> Result<()> {
        let req = ctx.data.downcast_ref::<ToolPipelineData>().unwrap();
        let result = self.mcp.call_tool_async(req.request.clone()).await?;
        // 将结果写回 data（业务层定义的可变容器）
        ctx.data.downcast_mut::<ToolPipelineData>().unwrap().result = Some(result);
        next.call().await
    }
}
```

### 10.1.4 业务层组装 Pipeline

```rust
// ═══════════════════════════════════════════
// Pipeline 组装在业务层，内核只提供 Pipeline 容器
// ═══════════════════════════════════════════

fn build_tool_pipeline(
    policy: Arc<PolicyEngine>,
    mcp: Arc<MCP>,
) -> Pipeline {
    let mut pipeline = Pipeline::new();
    pipeline.push(PolicyCheckStage { engine: policy });
    pipeline.push(BeforeHookStage);
    pipeline.push(ExecuteToolStage { mcp });
    pipeline.push(WriteAgentTimelinePartStage);     // 如果当前业务链路还没有统一写 AgentTimelinePart
    pipeline.push(ObserveToolResultStage);  // 只写 tracing 观察日志，可选
    pipeline.push(AfterHookStage);
    pipeline.push(AuditStage);
    pipeline
}

// 使用
let data = ToolPipelineData { request: tool_call, result: None };
let mut ctx = PipelineContext::new(
    data,
    KernelContext::new("tool", KernelScope::scoped("session", session_id)),
)
.with_observer_fn(|event| { /* 通知 UI 投影：event.kind / event.stage_id / event.payload */ })
.with_audit(audit_recorder);
pipeline.run(&mut ctx).await?;
let result = ctx.data_mut::<ToolPipelineData>()?.result.take();

// 只有真实同步 API 边界才使用内核提供的 blocking runner。
// 不要从 Agent turn loop / Tauri async command / Gateway stream 中调用它。
pipeline.run_blocking(&mut sync_ctx)?;
```

### 10.1.5 业务层定义事件 topic

```rust
// ═══════════════════════════════════════════
// 事件 topic 由业务层定义，内核只提供 pub/sub 机制
// EventBus 必须在 Tokio runtime 内创建
// ═══════════════════════════════════════════

// 创建（必须传入 Tokio runtime Handle）
let event_bus = InMemoryEventBus::new(1000, tokio::runtime::Handle::current());

// 发布（业务层代码，在能力执行完成后）
event_bus.emit(EventEnvelope::new(
    "action.completed",
    KernelContext::new("agent", KernelScope::scoped("session", "sess_001")),
    json!({ "call_id": "call_001", "capability_id": "read_file", "duration_ms": 120 }),
))?;

// 订阅（业务层代码，在 UI 初始化时）
// topic 支持 glob 通配符："action.*" 匹配 "action.completed" / "action.failed"
let sub_id = event_bus.subscribe(
    Some(Topic::from("action.*")),
    None,  // 不限 scope
    Arc::new(|event| {
        render_timeline(event);
    }),
)?;

// 异步订阅：适合需要 async I/O 的投影刷新、远端通知等 handler
let async_sub_id = event_bus.subscribe_async(
    Some(Topic::from("action.completed")),
    Some("session:sess_001".to_string()),
    Arc::new(|event| {
        async move {
            refresh_projection(event).await;
        }
        .boxed()
    }),
)?;

// 取消订阅
event_bus.unsubscribe(&sub_id)?;
event_bus.unsubscribe(&async_sub_id)?;

// 查询历史（返回 SharedEventEnvelope，零 payload 拷贝）
let recent = event_bus.recent(50);
```

### 10.1.6 业务层定义 Policy 约束

```rust
// ═══════════════════════════════════════════
// 约束由业务层实现，内核只提供 Constraint trait 和评估引擎
// ═══════════════════════════════════════════

/// 路径访问约束（业务层实现）
struct PathAccessConstraint;

impl Constraint for PathAccessConstraint {
    fn id(&self) -> &str { "path_access" }
    fn priority(&self) -> i32 { 50 }  // 高优先级
    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        if input.action == "data.write" && input.target.starts_with("/etc") {
            Some(PolicyDecision::Deny { reason: "禁止写入系统目录".into() })
        } else {
            None  // 不适用，交给下一个约束
        }
    }
}

/// 会话级授权约束（业务层实现）
struct SessionGrantConstraint { grants: Arc<RwLock<HashMap<String, Vec<Grant>>>> }

impl Constraint for SessionGrantConstraint {
    fn id(&self) -> &str { "session_grant" }
    fn priority(&self) -> i32 { 400 }  // 会话级优先级
    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        // 检查当前 scope 是否有匹配的授权记录
        // ...
    }
}

// 组装 Policy 引擎
let mut policy = PolicyEngine::new();
policy.add(PathAccessConstraint);
policy.add(SessionGrantConstraint { grants: ... });
policy.add(PlanModeConstraint { ... });
policy.add(DefaultModeConstraint { ... });
```

### 10.1.7 依赖注入

所有内核实例通过 Tauri `State<T>` 注入到业务模块：

```rust
// ═══════════════════════════════════════════
// lib.rs：注册内核和业务层实例
// EventBus 必须传入 Tokio runtime Handle
// ═══════════════════════════════════════════

let runtime_handle = tokio::runtime::Handle::current();
let event_bus = Arc::new(InMemoryEventBus::new(1000, runtime_handle));
let policy_engine = Arc::new(PolicyEngine::new());
let mcp_tool_registry: Arc<InMemoryRegistry<dyn MCPToolCapability>> =
    Arc::new(InMemoryRegistry::new());

app.manage(event_bus);
app.manage(policy_engine);
app.manage(mcp_tool_registry);

// 业务模块通过 State<T> 获取
#[tauri::command]
pub async fn do_something(
    tools: State<'_, Arc<InMemoryRegistry<dyn MCPToolCapability>>>,
    policy: State<'_, Arc<PolicyEngine>>,
    bus: State<'_, Arc<InMemoryEventBus>>,
) -> Result<()> {
    // 消费内核
}
```

---

## 10.2 各子系统接入边界

当前 Navis Go 的执行基线是九大域：`ai/app/extension/foundation/kernel/security/tool/ui/project`。
Extension、MCP、Sandbox 都是应用域服务，不删除、不降级为内核内部类型；它们通过 Kernel Registry / Pipeline / EventBus / Policy 的通用契约接入。

每个子系统需要回答三个问题：**注册什么到 Registry？发布什么到 Event Bus？消费什么 Policy？**

总览：

| 子系统 | 注册 | 发布事件 | 消费 Policy | 当前边界 |
|--------|------|---------|------------|---------|
| **Agent** (`16`) | Agent 模式和任务状态 | turn/action/reasoning/response 事件 | turn loop/子任务/模式切换 | `ai/agent` 负责 turn loop、决策和任务状态；`tool/agent` 负责工具运行链 |
| **Gateway** (`12`) | ProviderConfig 到 Gateway 持有的 Kernel Registry；ModelRouter 只维护路由索引 | stream/quota/throttled 事件 | 请求/配额 | `ai/gateway` 是模型调用统一入口 |
| **MCP 协议引擎** (`13`) | Transport / ToolDefinition 到 `tool/mcp` 注册表 | connection/discovery/tool 事件 | 连接/外部工具调用 | 保留为 `tool` 大域服务 |
| **Task** (`17`) | 不注册 | task 事件 | 子任务创建/执行 | Task Sidechain 是业务编排，不是 Kernel 原语 |
| **Skills** (`19`) | 不注册（Skill 非可执行能力） | skill 事件 | 不消费 | Skill 只影响上下文和意图匹配 |
| **Session** (`08`) | 不注册 | session 事件 | 不消费 | Session 是 project/worktree/session 事实源 |
| **Sandbox** (`06`) | Sandbox constraints 注册进 Policy | 安全审计事件 | 路径/命令/网络/资源约束 | 保留为 `security` 大域服务 |
| **Extension** (`07`) | ExtensionStore 只存安装状态和声明；contributes 分发到宿主子系统 | extension lifecycle / hook 事件 | 扩展权限/资源约束 | 保留为 `extension` 大域服务 |
| **Terminal** (`11`) | 不注册 | terminal 事件 | 命令执行前检查 | 终端属于 Tool 能力 |
| **UI** (`22`) | Host view declarations；renderer 由 UI 域白名单承接 | 不发布 | 订阅 action/session 事件 | UI 只读事实投影 |
| **File** (`09`) | 不注册 | 不发布 | 数据操作前检查 | 文件能力属于 Tool |
| **LSP** (`14`) | 语言服务到 LanguageRegistry | lsp 事件 | 不消费 | LSP 属于 Tool 能力 |
| **Config** (`03`) | 不注册 | `config.changed` | 配置作为约束来源 | Config 属于 foundation |

以下是逐模块的改造方案。

### 10.2.1 Agent Tool Chain（`tool/agent/`）

**现状：** 工具执行已经归入 `tool/agent/`，由 catalog / pipeline / runtime / guardrail / hooks / special 共同承接；`ai/agent/` 只保留 turn loop、决策、任务状态和上下文装配。`tool/agent` 不再维护独立 `AgentTool` trait、批量执行器或第二张可执行工具目录。

**落地边界：**

- MCP ToolDefinition / MCPToolCapability 是工具能力声明和执行实例的主要来源，注册事实进入 `tool/mcp/registry.rs` 的 Kernel Registry 门面。
- Tool Catalog 只做 provider-safe 名称、Gateway function schema、风险、UI hint 和 displayKind 投影。
- Agent runtime 把模型 tool call 规范化为 `ToolPipelineData`，再进入 Kernel Pipeline。
- 实际执行由 `McpExecutionStage` 调用 MCP Executor；Agent control tool 由 `tool/agent/special.rs` 显式处理，但仍输出同构 Gateway tool result 和 AgentTimelinePart。

#### 执行链

```text
Gateway tool call
  -> Tool Catalog 反查 provider-safe toolName
  -> ToolPipelineData
  -> Kernel Pipeline:
       PolicyCheckStage
       McpExecutionStage / Agent control stage
       ObserveToolResultStage
       EmitEventStage
       AuditStage
  -> MCP ToolCallResult / Agent control result
  -> Gateway tool result
  -> AgentTimelinePart
```

#### 发布事件

工具链只发布事实通知，不保存第二套事实源。当前 Agent turn loop 统一写入 AgentTimelinePart；Pipeline 内 `ObserveToolResultStage` 只负责 tracing 观察，不写第二份事实：

| 事件 topic | 触发点 | payload 关键字段 |
|-----------|--------|-----------------|
| `action.started` | MCP Executor 开始工具调用 | `sessionId`, `callId`, `tool`, `kind`, `args` |
| `authorization.requested` | Sandbox/Policy 需要审批 | `sessionId`, `callId`, `tool`, `message`, `args` |
| `action.completed` | 工具执行完成 | `sessionId`, `callId`, `tool`, `kind`, `durationMs`, `isError`, `result` |
| `action.failed` | 工具执行失败 | `sessionId`, `callId`, `tool`, `error`, `durationMs` |

Agent turn / reasoning / response 事件由 `ai/agent` turn loop 和 Session/Storage 写入路径发布；工具链不伪造这些事件。

#### 消费 Policy

| 操作 | PolicyInput |
|------|------------|
| 工具执行前 | `subject="agent", action="tool.*", target="{path|command|url}", scope="{session_id}"` |
| 扩展 hook 裁决 | `subject="extension:{id}", action="agent.extension_hook", target="{tool_name}", scope="{session_id}"` |
| 子任务派生前 | `subject="agent", action="task.create", target="{task_kind}", scope="session:{id}"` |
| 模式切换前 | `subject="agent", action="mode.switch", target="{mode_name}", scope="session:{id}"` |

### 10.2.2 Gateway（`gateway/`）

**当前边界：** Gateway 使用 `ProtocolAdapterRegistry` 统一管理内建和 Extension Adapter。Provider、Model、Adapter 是 Gateway 业务类型，不进入 Kernel。GatewayMiddleware 的执行 stage 由 Kernel Pipeline 承载。

**不变量：** Gateway 请求主流程只 resolve Adapter，不增加按协议的 `match` 分支；Extension lifecycle 只依赖 `GatewayCapabilityPort`。

#### 注册

```rust
// ProviderAdapter 定义在 Gateway 业务层，只负责协议转换。
pub trait ProviderAdapter: Send + Sync {
    fn transform_request(&self, request: &ChatRequest, model: &ModelConfig) -> Result<Value>;
    fn transform_response(&self, raw: &Value, model: &ModelConfig) -> Result<ChatResponse>;
    fn transform_stream_chunk(&self, raw: &Value, model: &ModelConfig) -> Result<Option<StreamChunk>>;
    fn endpoint(&self, model: &ModelConfig) -> String;
}

// 内建 Adapter 与 Extension Adapter 统一进入 ProtocolAdapterRegistry。
// Extension lifecycle 只通过 GatewayCapabilityPort acquire/register/release。
protocol_adapter_registry.register(ChatCompletionsAdapter::new()?);
protocol_adapter_registry.register(ResponsesAdapter::new()?);
```

ProviderConfig 由 Gateway router 维护，ModelConfig.api_protocol 决定 Registry resolve 的 Adapter。Extension 的 `contributes.gateway.adapters/providers` 经过 Extension plan 校验后注册，不修改 Gateway 主流程。

#### 发布事件

| 事件 topic | 触发点 | payload 关键字段 |
|-----------|--------|-----------------|
| `stream.started` | LLM 请求发出 | `request_id`, `model`, `provider`, `token_count` |
| `stream.chunk` | 流式数据块 | `request_id`, `chunk_type`, `data` |
| `stream.completed` | LLM 请求完成 | `request_id`, `model`, `usage`, `duration_ms` |
| `stream.error` | LLM 请求失败 | `request_id`, `error_type`, `status_code`, `message` |
| `quota.warning` | 配额警告 | `model`, `used_pct`, `remaining` |
| `throttled` | 被限流 | `model`, `retry_after_ms` |

#### 消费 Policy

| 操作 | PolicyInput |
|------|------------|
| Provider 调用前 | `subject="gateway", action="stream.send", target="{provider}:{model}", scope="global"` |
| 配额检查 | `subject="gateway", action="quota.check", target="{model}", scope="session:{id}"` |
### 10.2.3 MCP 协议引擎（`tool/mcp/`）

> 当前基线：MCP 保留为 `tool/mcp` 协议引擎，负责 transport、server discovery、tools/list、tools/call、内置和外部 MCP 工具接入。Agent 通过 `tool/agent/catalog.rs` 和 provider-safe 名称映射消费 MCP 工具，不直接依赖裸 MCPTool。

**现状：** MCP 保留为 `tool/mcp` 协议引擎，负责 transport、server discovery、tools/list、tools/call。MCP 工具能力和自定义 transport adapter 原型已通过 Kernel Registry 承载；MCP 层只保留协议 DTO、内置工具实例、server 配置/状态和已连接 transport 实例。

**落地边界：** MCP 继续作为协议层服务接入 Kernel 原语：工具发现进入 Kernel Registry，普通工具执行进入 Kernel Pipeline，Sandbox/Policy/Audit/EventBus 由统一执行链调用。Agent 通过 Tool Catalog 消费 MCP 工具，不直接依赖裸 MCPTool。

#### 模块边界

| 模块 | 处理 | 原因 |
|------|------|------|
| `tool/mcp/registry.rs` | 保留为 Kernel Registry 门面 | MCP canonical ToolDefinition、内置工具实例和 server tools 查询 DTO；注册/注销走 `InMemoryRegistry` |
| `tool/agent/catalog.rs` | 保留 | Agent 消费 MCP ToolDefinition 并做 provider-safe 名称映射 |
| `tool/mcp/server_manager.rs` | 保留 | MCP server lifecycle、tools/list、tools/call 接入；server 配置/状态/已连接 adapter 是领域运行状态 |
| `tool/mcp/builtin/*` | 保留 | 内置工具继续以 MCP canonical tool 进入 Tool Catalog |

#### 保留

| 模块 | 处理 | 原因 |
|------|------|------|
| `mcp/transport/stdio` | 保留为内置 transport | 内置 transport 由业务层枚举创建，不作为 server 运行状态 |
| `mcp/transport/sse` | 保留为内置 transport | 同上 |
| `mcp/transport/websocket` | 保留为内置 transport | 同上 |
| `mcp/transport/rest` | 保留为内置 transport | 同上 |
| `mcp/executor.rs` | 接入 Kernel Pipeline | Sandbox check、执行、事件、审计、调试历史通过 Stage 组合表达 |

#### 注册

```rust
// TransportAdapter trait 定义在业务层
pub trait TransportAdapter: Capability {
    fn connect(&self, config: Value) -> Result<()>;
    fn send(&self, message: Value) -> Result<Value>;
    fn receive(&self) -> Result<Value>;
    fn disconnect(&self) -> Result<()>;
}

// 自定义传输适配器原型注册到 Kernel Registry 门面
server_manager.register_transport("custom".into(), Box::new(CustomTransport::new()))?;
```

#### 外部 Tool 接入流程

```
外部 MCP Server 配置
  │
  ▼
ServerManager.create_transport_adapter(config)   // 内置 transport 或 Kernel Registry 中的自定义原型
  │
  ▼
transport.send({method: "tools/list"})           // 发现外部工具
  │
  ▼
对每个发现的外部工具：
  ToolDefinition::new(...)
  mcp_tool_registry.register_batch(...)           // 内部走 Kernel InMemoryRegistry
  │
  ▼
外部工具和内置工具在 Agent Catalog 看来完全一致
```

#### 发布事件

| 事件 topic | 触发点 | payload 关键字段 |
|-----------|--------|-----------------|
| `connection.established` | 传输连接建立 | `connection_id`, `transport`, `capability_count` |
| `connection.lost` | 传输连接断开 | `connection_id`, `reason` |
| `connection.error` | 传输连接错误 | `connection_id`, `error` |
| `discovery.completed` | 外部能力发现完成 | `connection_id`, `capability_ids[]` |

#### 消费 Policy

| 操作 | PolicyInput |
|------|------------|
| 传输连接前 | `subject="transport", action="connection.establish", target="{connection_id}", scope="global"` |
| 外部工具调用前 | `subject="transport", action="capability.execute", target="{connection_id}:{capability_id}", scope="session:{id}"` |

#### 内置 Server 收敛

内置 MCP 工具（filesystem/terminal/git/web/clipboard/memory/lsp）继续以 MCP canonical ToolDefinition 注册；Agent Tool Catalog 只负责 provider-safe 名称映射和执行策略：

```
当前：MCP 内置工具 → MCPTool trait → tool/mcp registry → tool/agent catalog/provider-safe name → tool/agent runtime → tool/mcp executor
```

外部 MCP Server 的工具仍然通过 MCP 传输适配器接入，注册为 MCP canonical ToolDefinition；Agent 侧只消费 Tool Catalog 投影，不绕过 MCP executor。

### 10.2.4 Session（`session/`）

**现状：** 会话管理独立，不发布事件，不消费 Policy。

**改造：**

#### 发布事件

| 事件 topic | 触发点 | payload 关键字段 |
|-----------|--------|-----------------|
| `session.created` | 创建会话 | `session_id`, `worktree_root` |
| `session.switched` | 切换会话 | `session_id`, `previous_id` |
| `session.archived` | 归档会话 | `session_id` |
| `session.deleted` | 删除会话 | `session_id` |
| `session.message.added` | 消息追加 | `session_id`, `message_id`, `role` |

#### 消费

- 订阅 `agent.turn.completed` 更新会话统计（token 用量、费用）
- 订阅 `action.completed` 更新会话文件变更列表

### 10.2.5 Sandbox 与 Policy 协作（`security/sandbox/`）

> 当前基线：Sandbox 保留为 `security` 大域模块。Policy 消费 Sandbox constraints，Sandbox 继续承载路径、命令、网络、资源和 project/worktree trust 等运行时约束。

**现状：** 独立的沙箱模块，路径/命令/网络黑白名单，操作分级 Level 0-3。

**落地边界：** Sandbox 继续维护运行时约束，Policy 通过这些约束做统一决策。Sandbox 的 `SandboxAuditView` 只保留近期查询视图/DTO 适配缓存；允许、拒绝、确认等结构化审计事实由 Sandbox 在校验入口通过 `kernel::AuditRecorder` 写入，不能由缓存模块另行作为事实源写入。

```rust
// Sandbox 约束以 Policy Constraint 形式参与决策

/// 路径访问约束
struct PathAccessConstraint { rules: Vec<PathRule> }
impl Constraint for PathAccessConstraint {
    fn id(&self) -> &str { "sandbox.path" }
    fn priority(&self) -> i32 { 10 }
    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        if input.action.starts_with("data.") || input.action.starts_with("capability.") {
            // 检查路径是否在白名单/黑名单中
        }
        None
    }
}

/// 命令执行约束
struct CommandConstraint { rules: Vec<CommandRule> }
impl Constraint for CommandConstraint {
    fn id(&self) -> &str { "sandbox.command" }
    fn priority(&self) -> i32 { 10 }
    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        if input.action == "command.execute" {
            // 检查命令是否在白名单/黑名单中
        }
        None
    }
}

/// 网络访问约束
struct NetworkConstraint { rules: Vec<NetworkRule> }
impl Constraint for NetworkConstraint {
    fn id(&self) -> &str { "sandbox.network" }
    fn priority(&self) -> i32 { 10 }
    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        if input.action.starts_with("net.") || input.target.contains("://") {
            // 检查网络目标是否允许
        }
        None
    }
}
```

**落地边界：** `security/sandbox` 保留为运行时约束服务；Path / Command / Network / Resource 等约束注册进 Policy，由 Policy 做统一决策。

### 10.2.6 Extension 扩展系统（`extension/`）

**当前边界：** Extension 是应用域的安装、状态和生命周期服务，不是 Kernel 原语。Kernel 只提供 Registry、Pipeline、Policy 和 EventBus；Extension lifecycle 通过 capability port 调用具体宿主，不直接依赖宿主实现类型。

**实际状态模型：** `ExtensionState` 保存 id、status、manifest、install_path、installed_at、enabled_at 和 error。`ExtensionStore` 负责状态索引、manifest 查询、冲突检测、hook/work mode 声明登记和生命周期事实通知；它不把 Extension 本身作为 Kernel Registry item。

**宿主能力端口：**

| Port | 宿主职责 |
|------|----------|
| `GatewayCapabilityPort` | 事务登记/释放 Gateway Adapter、Provider、Model 和 protocol owner |
| `McpCapabilityPort` | 登记/启动/移除 MCP server，登记工具和应用工具覆盖 |
| `LspCapabilityPort` | 登记/移除语言与 LSP server 配置 |
| UI HostView projection | 读取已启用 Extension 的 views/menus/commands 声明并按白名单渲染 |
| Skills / Agent hook 宿主 | 将声明转换为 SkillDefinition 或受控 hook registration；没有真实执行宿主时拒绝启用 |

**贡献处理原则：** `extension/lifecycle/register.rs` 只把 manifest 数据转换成宿主 DTO/注册计划，不加载 Extension 模块、不执行 manifest 中的任意入口、不创建 Gateway 或 MCP 的第二套 Registry。未接入真实宿主的 contribution 必须 fail-closed。

#### 生命周期流程

```text
安装：Installer 读取 extension.json
  -> Loader 解析并校验 manifest、资源路径和版本
  -> ExtensionStore.register(ExtensionState)
  -> 发布 extension.installed

启用：ExtensionLifecycle 校验权限与 contribution
  -> 构造 Gateway/MCP/LSP/Skills/UI 注册计划
  -> 按依赖顺序通过 capability port 登记
  -> 任一步失败则逆序回滚并进入 Error
  -> 发布 extension.enabled 或 extension.error

禁用/卸载：停止新激活
  -> 逆序释放宿主 contribution、权限和声明索引
  -> 更新 ExtensionState
  -> 发布 extension.disabled / extension.uninstalled
```

#### ExtensionStore 领域接口

```rust
pub fn register(&self, state: ExtensionState) -> Result<()>;
pub fn unregister(&self, extension_id: &str) -> Result<()>;
pub fn get(&self, extension_id: &str) -> Option<ExtensionState>;
pub fn list(&self) -> Vec<ExtensionState>;
pub fn list_by_status(&self, status: &ExtensionStatus) -> Vec<ExtensionState>;
pub fn update_status(&self, extension_id: &str, status: ExtensionStatus, error: Option<String>) -> Result<()>;
```

ExtensionStore 只管理 Extension 状态和声明索引；Gateway Provider/Model、MCP server/tool、LSP language 和 UI view 的运行事实分别由各自宿主维护。

#### Extension 事件与 Policy

| 事件 topic | 触发点 |
|-----------|--------|
| `extension.installed` / `extension.uninstalled` | 安装状态变更 |
| `extension.enabling` / `extension.enabled` | 启用开始/完成 |
| `extension.disabling` / `extension.disabled` | 禁用开始/完成 |
| `extension.updated` | 版本或 manifest 更新 |
| `extension.error` | 校验、注册或回滚失败 |

Extension 权限通过 Policy constraint 参与决策；ExtensionStore 不绕过 Policy，也不把权限判断复制到各宿主。

### 10.2.7 Skills（`skills/`）

**当前边界：** Skills 是 Extension/Agent 之间的提示词与激活域。`Skills` 持有 `SkillStore`、`SkillLoader`、`SkillActivationService`、命令/角色管理器和统一 EventBus；Skill 不是可执行能力，不进入 Kernel Registry。

**当前流程：** `contributes.skills` 或内置 SKILL.md 经 Loader/Parser/Validator 转换为 `SkillDefinition`，写入 `SkillStore`。`tool/agent` 的 `SkillMatchStage` 负责按显式 trigger 匹配，`SkillActivationService` 生成 prompt/tool whitelist/step activation plan，后续执行仍走 Agent Tool Pipeline、Policy 和审计链路。

- Skill 不加载任意脚本，不绕过 Agent Tool Pipeline。
- Skill 只影响上下文提示、可用工具和声明式步骤，不替代 Agent、Gateway 或 Policy。
- Extension 禁用时，来源 Skill 和相关触发声明必须从 SkillStore/宿主索引移除。

#### Skill 定位

```
Skill = 提示词包 + 工具白名单 + 匹配规则
Skill 不可执行，不注册到 Registry
Skill 通过 Agent Pipeline 中的 SkillMatchStage 被激活
```

#### 自动调用机制

SkillMatchStage 是 Agent Pipeline 的第一个 Stage，在能力执行之前运行：

```rust
struct SkillMatchStage {
    skill_store: Arc<SkillStore>,
    event_bus: Arc<EventBus>,
}

impl Stage for SkillMatchStage {
    fn id(&self) -> &str { "skill_match" }

    async fn process(&self, ctx: &mut PipelineContext, next: Next<'_>) -> Result<()> {
        let req = ctx.data.downcast_ref::<AgentTurnRequest>().unwrap();

        // Step 1：精确匹配（用户显式输入 /commit）
        if let Some(skill) = self.skill_store.match_trigger(&req.user_message) {
            ctx.data.downcast_mut::<AgentTurnRequest>().unwrap()
                .active_skill = Some(skill);
            return next.call().await;
        }

        // Step 2：意图匹配（自动识别）
        if let Some(m) = self.skill_store.match_intent(&req.user_message, &req.context) {
            self.event_bus.emit(EventEnvelope::new("skill.auto_matched", ctx.clone(), json!({
                "skill_id": m.skill.id,
                "confidence": m.confidence,
                "reason": m.reason,
            }))?;

            ctx.data.downcast_mut::<AgentTurnRequest>().unwrap()
                .active_skill = Some(m.skill);
        }

        next.call().await
    }
}
```

#### 意图匹配器

SkillStore 支持三种匹配策略，按优先级依次尝试：

```rust
/// 精确触发匹配（用户输入 /commit）
fn match_trigger(&self, input: &str) -> Option<SkillDefinition> {
    // 匹配 "/commit" → commit Skill
}

/// 意图匹配（自动识别）
fn match_intent(&self, input: &str, context: &ContextData) -> Option<SkillMatch> {
    // 依次尝试：PatternMatcher → EmbeddingMatcher → ClassifierMatcher
}

struct SkillMatch {
    skill: SkillDefinition,
    confidence: f32,        // 0.0 ~ 1.0
    reason: String,         // "检测到 staged 变更，建议执行 commit"
    requires_confirm: bool, // 低置信度时需要用户确认
}
```

三种匹配策略：

| 策略 | 原理 | 速度 | 准确度 | 适用场景 |
|------|------|------|--------|---------|
| `PatternMatcher` | 关键词/正则匹配 | < 1ms | 高（确定性） | "commit" / "部署" / "测试" 等明确意图 |
| `EmbeddingMatcher` | 嵌入向量相似度 | ~10ms | 中（语义级） | "代码改完了" → commit Skill |
| `ClassifierMatcher` | 小模型分类 | ~50ms | 高 | 复杂意图、多候选消歧 |

#### 置信度分级

| 置信度 | 处理方式 | 事件 | UI 表现 |
|--------|---------|------|---------|
| ≥ 0.9 | 直接激活 | `skill.auto_matched` | 静默执行 |
| 0.7 ~ 0.9 | 激活但标注 | `skill.auto_matched` | 显示 "自动识别：X Skill" |
| 0.5 ~ 0.7 | 询问用户 | `skill.match_suggested` | 弹出 "检测到可能需要 X Skill，是否启用？" |
| < 0.5 | 不激活 | 不发布事件 | 无特殊表现 |

#### Skill 激活后的行为

```
Skill 被激活（手动或自动）
  │
  ▼
Agent 上下文注入：
  system_prompt += skill.prompt          // 注入 Skill 提示词
  available_tools = filter(              // 用工具白名单约束
    tool_registry.list(),
    |t| skill.tools_whitelist.contains(&t.id)
  )
  │
  ▼
Agent 按 Skill 指引执行（和正常 LLM 调用完全一致）
  → Skill 只影响"用什么提示词"和"有哪些工具可用"
  → 不影响 Agent 循环、Pipeline、Policy
```

#### 发布事件

| 事件 topic | 触发点 | payload 关键字段 |
|-----------|--------|-----------------|
| `skill.matched` | 精确匹配成功 | `skill_id`, `trigger` |
| `skill.auto_matched` | 意图匹配成功 | `skill_id`, `confidence`, `reason` |
| `skill.match_suggested` | 低置信度建议 | `skill_id`, `confidence`, `reason` |
| `skill.activated` | Skill 被激活 | `skill_id`, `source`（user / auto / suggested） |
| `skill.deactivated` | Skill 被停用 | `skill_id` |

#### Agent Pipeline 完整 Stage 链（含 Skill）

```text
SkillMatchStage           — 匹配并激活 Skill（精确/意图）
PolicyCheckStage          — 权限检查
BeforeHookStage           — 前置 Hook
ExecuteCapabilityStage    — 执行能力
WriteAgentTimelinePartStage       — 写入业务事实源（若外层未统一写入）
ObserveToolResultStage    — 可选观察日志，不作为事实源
EmitEventStage            — 发布事件
AuditStage                — 审计记录
```

#### Skill 定义扩展

```rust
struct SkillDefinition {
    id: String,
    name: String,
    description: String,
    prompt: String,                    // 提示词内容
    tools_whitelist: Vec<String>,      // 工具白名单
    parameters: Vec<SkillParameter>,
    steps: Vec<SkillStep>,
    
    // 新增：自动调用相关
    triggers: Vec<String>,             // 精确触发词 ["/commit", "/提交"]
    intent_patterns: Vec<String>,      // 意图匹配模式 ["代码.*改完", ".*提交.*"]
    auto_invoke: bool,                 // 是否允许自动调用
    min_confidence: f32,               // 最低置信度阈值（默认 0.7）
}
```

```json
// manifest.json 中的 Skill 定义
{
  "skills": [
    {
      "id": "commit",
      "name": "Git Commit",
      "prompt": "分析 staged 变更，生成规范的 commit message...",
      "tools_whitelist": ["git", "terminal", "read_file"],
      "triggers": ["/commit", "/提交"],
      "intent_patterns": ["代码.*改完", ".*提交.*变更", ".*commit.*"],
      "auto_invoke": true,
      "min_confidence": 0.7
    }
  ]
}
```

### 10.2.8 Task Sidechain 与子任务协调（`task_sidechain/` + `task_manager`）

**定位：** TaskManager 管理任务事实和运行时索引，Task Sidechain 管理子任务编排和 sidechain session。执行、通知、权限全部依赖内核原语。

**边界：** TaskManager 和 Task Sidechain 都是业务层服务，不是内核原语。它们通过 Pipeline 执行子任务，通过 Event Bus 通知状态，通过 Policy 判断权限。

#### 事实源

| 存储层 | 职责 | 说明 |
|--------|------|------|
| **SQLite `tasks` 表** | 事实源 | 唯一持久化存储，所有 Task 的最终状态以此为准 |
| **TaskManager（内存）** | 运行时索引 + 协调器 | 热数据缓存、并发协调、事件发布。崩溃后从 SQLite 重建 |

规则：
- Task 的 create / start / complete / fail 必须**先写 SQLite，再更新内存，再发布事件**
- 写 SQLite 失败 → 不更新内存、不发布事件
- TaskManager 启动时从 SQLite 重建内存索引
- 历史任务查询走 SQLite，不走 TaskManager 内存

#### Task 数据模型

```rust
pub struct Task {
    pub id: String,
    pub parent_id: Option<String>,     // 父任务 id
    pub kind: String,                  // "turn" / "sidechain" / "background" / "autonomous"
    pub status: String,                // "pending" / "running" / "completed" / "failed"
    pub input: Value,                  // 派发时的输入
    pub output: Option<Value>,         // 执行结果
    pub error: Option<String>,         // 错误信息
    pub scope: String,                 // "session:{id}" / "task:{parent_id}"
    pub progress: Option<f32>,         // 进度百分比
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

#### TaskManager 接口

```rust
pub struct TaskManager {
    index: RwLock<HashMap<String, Task>>,  // 内存索引（热数据）
    store: Arc<dyn TaskStore>,             // 事实源（SQLite）
    event_bus: Arc<EventBus>,
}

/// TaskStore trait：TaskManager 不直接依赖 SQLite，通过 trait 访问
pub trait TaskStore: Send + Sync {
    fn insert(&self, task: &Task) -> Result<()>;
    fn update(&self, task: &Task) -> Result<()>;
    fn get(&self, task_id: &str) -> Result<Option<Task>>;
    fn get_children(&self, parent_id: &str) -> Result<Vec<Task>>;
    fn all_completed(&self, parent_id: &str) -> Result<bool>;
}

impl TaskManager {
    /// 创建子任务（先写 SQLite，再更新内存，再发布事件）
    pub fn create(&self, input: CreateTaskInput) -> Result<Task> {
        let task = Task { ... };
        self.store.insert(&task)?;           // ① 写事实源
        self.index.write().insert(task.id.clone(), task.clone());  // ② 更新内存
        self.event_bus.emit(EventEnvelope::new("task.created", ctx.clone(), json!({...})))?;  // ③ 发布事件
        Ok(task)
    }

    /// 子任务开始执行
    pub fn start(&self, task_id: &str) -> Result<()> { ... }
    /// 子任务写入结果
    pub fn complete(&self, task_id: &str, result: Value) -> Result<()> { ... }
    /// 子任务报告失败
    pub fn fail(&self, task_id: &str, error: String) -> Result<()> { ... }
    /// 子任务报告进度（进度不写 SQLite，只更新内存 + 发布事件）
    pub fn progress(&self, task_id: &str, pct: f32, message: &str) { ... }
    /// 查询（优先内存，内存 miss 走 SQLite）
    pub fn get(&self, task_id: &str) -> Option<Task> { ... }
    /// 查询父任务的所有子任务
    pub fn get_children(&self, parent_id: &str) -> Vec<Task> { ... }
    /// 检查是否全部完成
    pub fn all_completed(&self, parent_id: &str) -> bool { ... }
    /// 取消任务
    pub fn cancel(&self, task_id: &str) -> Result<()> { ... }
}
```

#### Task Sidechain 执行模型

每个子任务 = 一次独立的 Pipeline 执行，并绑定一个 sidechain session。子任务共享同一个 Registry 和 PolicyEngine，但不继承父任务的临时授权：

```rust
async fn run_sidechain_task(
    task_id: String,
    sidechain_session_id: String,
    input: Value,
    pipeline: Arc<Pipeline>,
    task_manager: Arc<TaskManager>,
    cancellation: CancellationToken,
) {
    task_manager.start(&task_id);

    let tm = task_manager.clone();
    let tid = task_id.clone();
    let mut ctx = PipelineContext::new(
        SidechainTaskRequest {
            task_id: task_id.clone(),
            sidechain_session_id,
            input,
        },
        KernelContext::new("sidechain", KernelScope::scoped("task", &task_id)),
    )
    .with_cancellation(cancellation)
    .with_observer_fn(move |event| {
        if let Some(pct) = event.duration_ms {
            // 将执行观察事件投影为业务层进度
            tm.progress(&tid, 0.0, &event.message.as_deref().unwrap_or(""));
        }
    })
    .with_audit(AuditRecorder::disabled());

    match pipeline.run(&mut ctx).await {
        Ok(()) => {
            let result = ctx.data.downcast_ref::<SidechainTaskRequest>()
                .unwrap().output.take();
            task_manager.complete(&task_id, result.unwrap_or(json!(null)));
        }
        Err(e) => {
            task_manager.fail(&task_id, e.to_string());
        }
    }
}
```

#### 主任务派发 10 个子任务

```rust
async fn dispatch_sidechain_tasks(
    main_turn_id: &str,
    sub_tasks: Vec<SubTaskInput>,
    pipeline: Arc<Pipeline>,
    task_manager: Arc<TaskManager>,
    policy: Arc<PolicyEngine>,
) -> Vec<Task> {
    // 1. 创建子任务
    let tasks: Vec<Task> = sub_tasks.iter().map(|input| {
        task_manager.create(CreateTaskInput {
            parent_id: main_turn_id.into(),
            kind: "sidechain".into(),
            input: input.clone(),
            scope: format!("task:{}", uuid()),
        })
    }).collect();

    // 2. Policy 检查（子任务独立评估，不继承父任务权限）
    for task in &tasks {
        let decision = policy.evaluate(&PolicyInput {
            subject: "sidechain".into(),
            action: "task.create".into(),
            target: task.id.clone(),
            scope: task.scope.clone(),
            metadata: json!({}),
        });
        // Ask → 弹出确认，Deny → 跳过该子任务
    }

    // 3. 并行派发
    let handles: Vec<_> = tasks.iter().map(|task| {
        let pipeline = pipeline.clone();
        let tm = task_manager.clone();
        let id = task.id.clone();
        let input = task.input.clone();
        let sidechain_session_id = format!("sidechain:{}", id);
        let cancel = CancellationToken::new();
        tokio::spawn(run_sidechain_task(id, sidechain_session_id, input, pipeline, tm, cancel))
    }).collect();

    // 4. 等待全部完成
    futures::future::join_all(handles).await;

    // 5. 收集结果
    tasks.iter().map(|t| task_manager.get(&t.id).unwrap()).collect()
}
```

#### 发布事件

| 事件 topic | 触发点 | payload 关键字段 |
|-----------|--------|-----------------|
| `task.created` | 子任务创建 | `task_id`, `parent_id`, `kind` |
| `task.started` | 子任务开始执行 | `task_id` |
| `task.progress` | 子任务进度更新 | `task_id`, `progress_pct`, `message` |
| `task.completed` | 子任务完成 | `task_id`, `output_summary` |
| `task.failed` | 子任务失败 | `task_id`, `error` |
| `task.cancelled` | 子任务被取消 | `task_id`, `reason` |

#### 消费 Policy

| 操作 | PolicyInput |
|------|------------|
| 创建子任务前 | `subject="agent", action="task.create", target="{task_kind}", scope="session:{id}"` |
| 子任务执行能力前 | `subject="sidechain", action="capability.execute", target="{capability_id}", scope="task:{id}"` |

#### 子任务与主任务的交互总结

```
交互环节          机制              依赖内核？
────────────────────────────────────────────────
父→子 派发指令    TaskManager.create + Pipeline  ✅ Pipeline + Registry
子任务独立执行    各自 Pipeline 执行              ✅ Pipeline
子→父 结果回传    TaskManager.complete           ❌ 业务层服务
父→子 取消       CancellationToken               ✅ Pipeline 内置
子→父 进度通知    Event Bus emit                  ✅ Event Bus
父→子 权限管控    Policy evaluate                 ✅ Policy
主→主 并发协调    tokio::spawn + join_all         ❌ Tokio 运行时
```

**结论：子任务的执行、通知、权限、取消全部依赖内核。TaskManager 和 Task Sidechain 是业务层服务，但它们的事件通知和权限检查仍然通过内核原语。**

### 10.2.9 Terminal（`terminal/`）

**现状：** 命令执行引擎，双通道输出，不发布事件。

**改造：**

#### 发布事件

| 事件 topic | 触发点 | payload 关键字段 |
|-----------|--------|-----------------|
| `terminal.spawned` | 终端创建 | `terminal_id`, `shell` |
| `terminal.output` | 命令输出 | `terminal_id`, `stream`, `data` |
| `terminal.exited` | 终端关闭 | `terminal_id`, `exit_code` |

#### 消费 Policy

| 操作 | PolicyInput |
|------|------------|
| 命令执行前 | `subject="terminal", action="command.execute", target="{command}", scope="session:{id}"` |

### 10.2.10 UI 框架（`src/components/`）

**现状：** 宿主内建 view 与扩展 `contributes.views` 都由 UI 域承接；完整界面通过 HostView surface 渲染。

**改造：**

#### 注册

UI 不把 renderer 注册成 Kernel 概念。内置面板、扩展 view、菜单入口和命令入口都先进入 UI 域的 Host view declaration：

```rust
struct HostViewDeclaration {
    id: String,
    placement: String,     // rightWorkspace / bottomPanel / chatAside / tab / ...
    renderer: String,      // 当前落地：host:panel
    config: Value,
}
```

UI Host 只接受白名单 renderer。`host:panel` 是通用宿主面板 renderer，具体界面由 view contract、config、UI IPC、Kernel EventBus 投影或 `foundation::stream` 提供数据。renderer 字符串、placement 和 DOM 布局不进入 Kernel。

#### 消费

UI 组件订阅 Event Bus 事件驱动更新：

| UI 组件 | 订阅的事件 | 更新行为 |
|---------|-----------|---------|
| AgentTimeline | `action.*` / `reasoning.*` / `response.*` | 追加/更新步骤卡片 |
| ChatView | `response.delta` | 流式追加文本 |
| StatusBar | `quota.warning` / `throttled` | 显示配额警告 |
| Editor | `session.message.appended` | 更新文件变更 Diff |
| CommandPalette | （启动时查询 Registry） | 构建命令列表 |
| Notification | `turn.failed` / `stream.error` | 弹出 Toast |

### 10.2.11 File（`file/`）

**现状：** 文件操作封装，经 Sandbox 校验。

**改造：**

#### 消费 Policy

```rust
// 原来经 Sandbox 校验，改为经 Policy
fn write_file(path: &str, content: &str, ctx: &PipelineContext) -> Result<()> {
    let decision = policy_engine.evaluate(&PolicyInput {
        subject: "file".into(),
        action: "file.write".into(),
        target: path.into(),
        scope: ctx.scope_string(),
        metadata: json!({ "size": content.len() }),
    });
    match decision {
        PolicyDecision::Allow { .. } => { /* 执行写入 */ }
        PolicyDecision::Deny { reason } => Err(Error::Denied(reason)),
        PolicyDecision::Ask { .. } => { /* 暂停等待确认 */ }
    }
}
```

### 10.2.12 LSP（`lsp/`）

**现状：** 多 Server 生命周期管理，语言注册表。

**改造：**

#### 注册

```rust
// 语言服务注册到 LanguageRegistry
language_registry.register(RustLanguageServer::new());
language_registry.register(TypeScriptLanguageServer::new());
language_registry.register(PythonLanguageServer::new());
```

#### 发布事件

| 事件 topic | 触发点 | payload 关键字段 |
|-----------|--------|-----------------|
| `lsp.server.started` | LSP Server 启动 | `language`, `server_id` |
| `lsp.diagnostics` | 诊断更新 | `language`, `file`, `diagnostics[]` |
| `lsp.server.error` | LSP Server 错误 | `server_id`, `error` |

### 10.2.13 其他模块

| 模块 | 改造内容 | 发布事件 |
|------|---------|---------|
| **Config** (`03`) | 配置变更为 Policy 约束来源之一 | `config.changed` |
| **Knowledge** (`20`) | 不改。知识文件通过 Context Manager 注入上下文 | — |
| **Git** (`21`) | 消费 Policy（git push 前检查） | `git.status.changed` |
| **Clipboard** (`32`) | 消费 Policy（读写前检查） | — |
| **Logger** (`01`) | 订阅所有 `*` 事件写入日志文件 | — |

---

## 10.3 新增功能验证场景

### 场景 A：Agent 可视化面板

```
需求：菜单项打开实时可视化面板，展示 Agent 在做什么

实现：
  1. Extension manifest 声明 contributes.views、contributes.menus 和必要的 contributes.commands
  2. UI HostView 读取 projection，面板订阅 action.* / reasoning.* / response.* 事件
  3. event_bus.recent(100) 回放历史

前提：Agent 子系统必须发布 10.2.1 中列出的事件
改动：Agent Pipeline 加 EmitEventStage（业务层改动，不改内核）
```

### 场景 B：Computer Use

```
需求：菜单开关启用后，模型可自动触发截屏/鼠标/键盘操作

实现：
  1. Extension manifest 声明 contributes.tools 和 contributes.menus
  2. 菜单回调通过宿主 capability/command contract 切换 computer-use 能力
     true  → ToolRegistry.register(screenshot/mouse/keyboard/screen_read)
     false → ToolRegistry.unregister(...)
  3. ComputerUseConstraint 注册到 PolicyEngine
  4. Agent Tool Catalog 从 ToolRegistry 投影可用工具，Context 组装读取同一份投影
  5. 模型看到新工具，根据用户意图自动决定是否调用

约束：工具必须通过 ToolDefinition、Agent Tool Catalog、Policy 和 Pipeline 全链路接入；UI 菜单只触发宿主命令，不直接注册或执行工具
```

### 场景 C：第三方 LLM Provider（如 MiMo）

```
需求：接入一个使用新协议的 MiMo Provider，不改 Gateway 主流程

实现：
  1. Extension manifest 声明 `contributes.gateway.adapters` 和 `contributes.gateway.providers`
  2. Extension lifecycle 校验 adapterId、模型能力、defaultModel 和 secretRef
  3. 通过 GatewayCapabilityPort 生成注册计划并登记 Adapter/Provider/Model
  4. Gateway 根据 Model.api_protocol 通过 ProtocolAdapterRegistry resolve Adapter

前提：Provider/Model 进入 Gateway 领域目录，Kernel 只提供 Registry/Pipeline/Policy/EventBus 原语
改动：只增加 Adapter/声明和 projection 数据，不改 Gateway router、UI Settings 或 Kernel
```

### 场景 D：自定义主题

```
需求：用户安装暗紫色主题扩展

实现：
  1. manifest 声明 theme contribution
  2. ExtensionLifecycle 校验 theme contribution 有真实 UI 宿主落点
  3. UI Host 从 ExtensionStore/Config 投影读取可用主题声明

前提：UI 主题系统有真实 Host contribution 落点；不把主题做成 Kernel 类型
改动：主题加载改为 UI 域声明投影 + Config 应用
```

### 场景 E：Skill 自动调用

```
需求：用户输入 "代码改完了"，系统自动识别意图并激活 commit Skill

实现：
  1. Skill 定义中配置 auto_invoke=true 和 intent_patterns
  2. SkillStore 新增 match_intent() 方法（PatternMatcher / EmbeddingMatcher）
  3. Agent Pipeline 首个 Stage 为 SkillMatchStage
     - 精确匹配：用户输入 "/commit" → 直接激活
     - 意图匹配：用户输入 "代码改完了" → match_intent() → confidence 0.85 → 自动激活
  4. event_bus.emit(EventEnvelope::new("skill.auto_matched", ctx, ...)) → UI 显示 "自动识别：Git Commit"
  5. Skill 激活后注入提示词 + 工具白名单 → Agent 按 Skill 指引执行

前提：Agent Pipeline 支持在能力执行前运行 SkillMatchStage
改动：Agent Pipeline 加 SkillMatchStage（业务层改动，不改内核）
      SkillStore 新增意图匹配能力（业务层改动）
```

### 场景 F：自定义 LLM 对话风格

```
需求：用户安装一个 "专业严谨" 对话风格扩展，注入 system prompt

实现：
  1. manifest 注册一个 Skill（prompt="请用专业严谨的风格回答..."）
  2. SkillStore 加载该 Skill
  3. 用户在设置中选择 "专业严谨" 风格 → Skill 始终激活
  4. 或配置 auto_invoke=true + intent_patterns=[".*"] → 所有对话自动注入

前提：Skill 支持始终激活模式（无 trigger，通过配置激活）
改动：SkillDefinition 新增 default_active 字段（业务层改动）
```

---

## 11. 当前执行边界

Kernel V1 的通用底座已经补齐：四个一等原语保持稳定，跨切纪律落在对象、资源、checkpoint、snapshot、错误分类和审计上。后续修改 Kernel 接口必须继续满足 2.8 的完备性判断：服务多个原语、不包含业务词汇、能降低外层平行系统出现的概率。

- `Registry` 负责能力登记和生命周期。
- `Pipeline` 负责可追踪、可取消、可审计的执行链。
- `EventBus` 负责离散事实通知。
- `Policy` 负责权限、沙箱和扩展约束的统一决策。
- `Audit` 是内建不变量，通过 `AuditRecorder` / `AuditSink` 写入结构化事实源。

执行约束：

- 不新增平行 EventBus、Registry、Pipeline 或 Policy。
- 不新增过渡、兼容或 bridge 分支。
- 不把 Extension、MCP、Sandbox 降级为 Kernel 内部类型。
- 不为了单个业务功能修改 Kernel 接口；业务需求必须先抽象为通用对象、资源、checkpoint、snapshot 或错误语义。
- 能使用成熟高性能第三方库解决的内核通用问题，不手写自研实现。

### 11.1 已落地内核完备性

| 能力 | 说明 |
|------|------|
| `KernelErrorKind` + retry/error 分类 + `Display` impl + `KernelErrorKind` 上的方法 | 统一错误语义，snake_case 输出，`is_retryable` 等分类方法数据驱动，支撑 Pipeline retry、审计和 UI 呈现 |
| `PolicyErrorKind` 子枚举收敛 | 5 个 Policy 相关 `KernelError` 变体收敛为 `Policy { kind, detail }` + 5 个便捷构造器，减少变体膨胀 |
| 各原语 Stats / `KernelSnapshot` | 内核状态可见，避免靠日志猜测 |
| `KernelObjectInfo` | 统一对象身份，为 snapshot、audit、resource lease 提供基础 |
| `PolicyCheckpoint` | 给关键入口提供通用治理点，避免业务层绕开 Policy |
| `ResourceLease` / `KernelResource` / `ShutdownMode` | 统一资源持有、活跃租约计数和 shutdown 语义 |
| `ExecutionObservationSink` / `ExecutionEvent` | Pipeline 执行生命周期的程序化消费入口，与 tracing span 互补；用统一观察模型替代单一 ProgressCallback |
| ID newtype 非空校验 | `From` impl 含 `assert!(!is_empty())`，debug 构建中立即暴露编程错误 |
| 零拷贝 payload 传递 | `EventEnvelope.payload: Option<SharedArc<Value>>` + `KernelContext.metadata: Option<SharedArc<Value>>`，`None` 避免空 JSON 分配，`Some` 仅增加引用计数 |
| 热路径分配优化 | `run_id: Arc<str>` + `OnceLock<SharedArc<Value>>` null 单例 + `AuditRecorder::record_owned()` 优先路径，observation 路径零冗余分配 |
| EventBus 溢出降级 | 满队列 warn + `overflow_count` 计数 + `has_overflow()` 便捷方法，不阻断调用方；事件仍保留在 history ring-buffer |
| 高性能第三方库 | `arc-swap`、`flume`、`triomphe`、`downcast-rs`、`backon`、`crossbeam-channel`、`futures` 已用于对应热点 |

不实现 scheduler、VFS、module loader、storage abstraction；这些会把业务或 OS 语义带进内核。只有当它们能被抽象为四原语之上的通用对象/资源/策略/观测能力时，才重新评估。

---

## 12. 验收标准

Kernel V1 的验收标准：

- 只保留 Registry / Pipeline / EventBus / Policy 四个一等原语。
- Audit、KernelObjectInfo、ResourceLease、PolicyCheckpoint、KernelSnapshot、KernelErrorKind 只作为跨切内核纪律存在，不成为业务入口。
- `src-tauri/src/kernel` 不出现业务模块词汇或业务专用 trait。
- Registry 支持注册、注销、生命周期、热替换、对象导出和 stats。
- Pipeline 支持类型安全 data、异步 stage、重试策略、对象导出、运行计数和 shutdown 边界。
- EventBus 支持同步/异步 handler、背压队列、注销、历史查询、对象导出和 stats。
- Policy 支持 fail-closed 决策、重复 constraint 拒绝、热替换、checkpoint、对象导出和 stats。
- Audit 支持结构化记录、禁用 recorder、后台缓冲 sink 和 stats。
- 新增 Tool / Provider / MCP Transport / Policy rule 不需要修改 Kernel；新增 UI renderer 由 UI 域承接，不修改 Kernel。未来如果某个 renderer runtime 需要安装、启停或替换，UI 域也只是复用通用 capability 生命周期，Kernel 仍不增加 renderer 语义。
- `cargo fmt`、`cargo test kernel`、`cargo check` 通过。

---

## 13. 反模式

以下写法禁止：

- 为一个功能新增平行事实源。
- 在 UI 里执行 Agent tool。
- 扩展直接操作 chat DOM。
- Skill 直接执行系统命令。
- MCP server 连接失败后注册空工具。
- Policy 判断失败后默认 allow。
- Pipeline stage 用固定文案伪造 progress。
- Event consumer 修复数据库事实。
- Gateway 猜工具名或做临时 `.` / `_` 转换。
- 保留旧协议分支。
- Policy 中硬编码业务逻辑（Policy 只判断"是否允许"，不包含"如何执行"）。
- Event Bus 用于同步等待（Event Bus 是异步通知，禁止 `await for event` 模式）。
- Registry 返回可变引用（绕过生命周期管理，必须通过 `lifecycle()` 方法改变状态）。
- Pipeline Stage 依赖外部可变状态（Stage 应无状态，状态通过 PipelineContext 传递）。

---

## 14. 核心场景闭环用例

以下用例覆盖四原语的完整交互，用于验证业务域是否按当前边界接入 Kernel 原语。

### 用例 1：Tool 调用全链路

```
用户输入 "帮我读一下 main.rs"
  │
  ▼
Agent Loop 决定调用 provider-safe 工具 read
  │
  ├─ Tool Catalog 反查 read → fs.read_file
  ├─ Pipeline.run(ToolPipelinePayload):
  │     [GenericStage: PolicyStage]
  │       Policy.evaluate(session, read_file, /path/main.rs)
  │       → Allow
  │     [GenericStage: AuditStage]
  │       记录：谁、调什么、Policy 结果、开始时间
  │     [TypedStage: ExecuteToolStage]
  │       mcp.call_tool("fs.read_file", {path: "/path/main.rs"})
  │       → 文件内容
  │     [TypedStage: PersistStepStage]
  │       写入 agent_timeline_parts 表（事实源先写）
  │     [GenericStage: AuditStage]
  │       记录：耗时、输出摘要、status=success
  │
  ├─ Event Bus.emit(EventEnvelope::new("action.completed", ctx, ...))  → UI 订阅者更新 Timeline
  ├─ StreamChannel 推送结果到前端            → 用户看到文件内容
  │
  └─ Agent Loop 继续下一步
```

验证点：
- Policy 在 Execute 之前执行（不是之后）
- agent_timeline_parts 先写入再发事件（事实源优先）
- Audit 记录完整（trace_id、duration、policy decision）
- Tool Catalog、MCP 调用、权限判断和 AgentTimelinePart 写入都经过同一 Pipeline

### 用例 2：权限审批全链路

```
Agent 决定调用 bash（高风险 Tool）
  │
  ▼
Pipeline.run:
  [PolicyStage]
    Policy.evaluate(session, bash, "rm -rf /tmp/test")
    → Ask { grant_spec: GrantSpec { scope: Once, constraint: CommandPattern { "rm *" } } }
    │
    ▼
  Pipeline 暂停，前端弹出确认对话框
  用户点击 "allow_session"
    │
    ▼
  Pipeline 恢复：
    Policy 写入 session grant: { session_id, bash, "rm *" }
    → 后续同一 session 内 bash 调用 "rm xxx" 自动 Allow
    → bash 调用 "curl xxx" 仍然需要审批（不匹配 grant）
```

验证点：
- GrantSpec 是结构化类型，前端直接渲染，不需要解析字符串
- Session grant 包含 permission（工具名）+ constraint（命令模式），不是只按路径缓存
- Grant 过期或 session 结束后自动失效

### 用例 3：Extension 注册新模型协议

```
用户启用 Extension "custom-llm-extension"
  │
  ▼
Extension Loader 校验 manifest.contributes.gateway
  ├─ gateway.adapters: protocolId / kind / config
  └─ gateway.providers: adapterId / baseUrl / secretRef / models
  │
  ▼
Extension lifecycle 生成 Gateway plan
  │  通过 GatewayCapabilityPort 注册 CustomProtocolConfig
  │  通过 GatewayCapabilityPort 注册 ProviderConfig/ModelConfig
  │
  ▼
ProtocolAdapterRegistry.resolve(model.api_protocol)
  │
  ▼
Gateway router -> Adapter transform -> HTTP dispatch -> normalized response
```

验证点：
- 新 Provider、Model 或协议不修改 Gateway 主流程。
- adapterId、Provider ID、Model ID、defaultModel 和 secret_ref 校验失败即拒绝。
- Extension 禁用或回滚时逆序释放 Provider 和 protocol owner。
- MCP server 连接失败 -> fail-closed，不注册空工具。

### 用例 4：跨原语协作（全链路验证）

```
场景：Extension 提供了一个 "deploy" Tool，需要网络权限

1. Registry:  Extension 暴露 deploy MCP ToolDefinition / capability
2. Policy:    deploy 需要 network 访问 → Extension manifest 声明 permissions.network
3. Pipeline:  Agent 调用 deploy → PolicyStage 检查 Extension 权限 → Ask
4. Event Bus: deploy 执行中发布 progress 事件 → UI 显示进度
5. Audit:     记录 Extension 来源、Policy 决策、执行耗时、网络目标
6. Fact:      AgentTimelinePart 写入（事实源），Event Bus 通知 UI（通知）
```

---

## 15. 非功能需求

### 15.1 性能阈值

| 操作 | 阈值 | 说明 |
|------|------|------|
| Registry.get() | < 1μs | 内存 HashMap 查询 |
| Registry.register() | < 10μs | 内存写入 + lifecycle 状态变更 |
| Policy.evaluate() | < 100μs | 单次约束求解（不含 UI 等待） |
| Pipeline.run() 开销 | < 50μs | 不含 Stage 自身耗时，纯框架调度开销 |
| Audit 写入 | < 1ms | SQLite WAL 模式单条写入 |
| Event Bus emit | < 10μs | 不含 handler 执行时间 |

### 15.2 测试模板

每个内核原语必须有：

| 测试类型 | 数量要求 | 覆盖点 |
|---------|---------|--------|
| 单元测试 | 每个 trait 方法 ≥ 3 个 | 正常路径、边界条件、错误路径 |
| 集成测试 | 每个核心场景 ≥ 2 个 | 用例 1-4 的简化版本 |
| fail-closed 测试 | 每个 fail-closed 规则 ≥ 1 个 | 确认失败时返回错误而非默认值 |

### 15.3 监控指标

内核必须暴露以下指标（通过 tracing span 和可查询接口）：

| 指标 | 来源 | 用途 |
|------|------|------|
| Pipeline 执行耗时分布 | AuditStage | 发现慢 Stage |
| Policy 决策统计（allow/deny/ask 比例） | PolicyStage | 安全审计 |
| Registry 能力数量（按 kind/source 分组） | Registry.list() | 扩展生态健康度 |
| Event Bus 积压（未处理事件数） | EventBus | 消费者健康度 |
| Audit 写入延迟 | AuditSink | 存储性能 |

---

## 16. 命名规范

所有内核及业务层的命名必须**见名知意**，禁止使用历史遗留的缩写或内部代号。

### 16.1 命名原则

| 原则 | 正确 | 错误 |
|------|------|------|
| 见名知意 | `capability` / `action` / `target` | `tool` / `op` / `res` |
| 用完整单词 | `connection` / `execution` / `evaluation` | `conn` / `exec` / `eval` |
| 类型名用名词 | `AgentAction` / `PolicyDecision` | `DoAgent` / `CheckPolicy` |
| 事件名用名词+动词过去式 | `connection.established` / `action.completed` | `connect` / `do_action` |
| 函数名用动词开头 | `register_capability()` / `evaluate_policy()` | `capability()` / `policy()` |
| 布尔字段用形容词 | `is_running` / `is_enabled` | `running` / `enabled`（易与状态枚举混淆） |
| 集合用复数 | `constraints` / `stages` / `subscriptions` | `constraint_list` / `stage_vec` |

### 16.2 内核命名

| 概念 | 命名 | 禁止 |
|------|------|------|
| 能力标识 | `capability_id` | `tool_id` / `name`（歧义） |
| 能力类型 | `kind`（字符串） | `capability_kind` / `type`（Rust 保留字） |
| 事件主题 | `topic` | `event_name` / `name` |
| 事件信封 | `EventEnvelope` | `KernelEvent` / `AppEvent` |
| 策略输入 | `subject` / `action` / `target` / `scope` | `who` / `what` / `where` / `ctx` |
| 策略决策 | `PolicyDecision` | `PermissionResult` / `AuthResult` |
| 管道阶段 | `Stage` | `Pipe` / `Step` / `Phase` |
| 管道上下文 | `PipelineContext` | `PipeCtx` / `RunContext` |
| 观察 sink | `ExecutionObservationSink` | `ProgressCallback` / `ProgressSink` / `OnProgress` |
| 取消令牌 | `CancellationToken` | `CancelToken` / `Abort` |

### 16.3 事件 topic 命名

格式：`{领域}.{实体}.{动作}`

| 命名 | 说明 |
|------|------|
| `action.started` | 能力执行开始 |
| `action.progress` | 能力执行进度 |
| `action.completed` | 能力执行完成 |
| `action.failed` | 能力执行失败 |
| `reasoning.started` | 推理开始 |
| `reasoning.delta` | 推理增量 |
| `reasoning.completed` | 推理完成 |
| `response.delta` | LLM 文本增量 |
| `response.completed` | LLM 文本完成 |
| `authorization.requested` | 权限请求 |
| `authorization.responded` | 权限响应 |
| `turn.started` | 回合开始 |
| `turn.completed` | 回合完成 |
| `connection.established` | 连接建立 |
| `connection.lost` | 连接断开 |
| `connection.error` | 连接错误 |
| `discovery.completed` | 能力发现完成 |
| `stream.chunk` | 流式数据块 |
| `stream.completed` | 流式完成 |
| `stream.error` | 流式错误 |
| `quota.warning` | 配额警告 |
| `throttled` | 被限流 |
| `session.created` | 会话创建 |
| `session.switched` | 会话切换 |
| `session.archived` | 会话归档 |
| `session.message.appended` | 消息追加 |
| `terminal.spawned` | 终端创建 |
| `terminal.output` | 终端输出 |
| `terminal.exited` | 终端退出 |
| `diagnostics.updated` | 诊断更新 |
| `config.changed` | 配置变更 |
| `health.snapshot` | 健康快照 |
| `health.alert` | 健康告警 |
| `job.enqueued` | 任务入队 |
| `job.started` | 任务开始 |
| `job.completed` | 任务完成 |
| `job.failed` | 任务失败 |
| `task.created` | 子任务创建 |
| `task.started` | 子任务开始执行 |
| `task.progress` | 子任务进度更新 |
| `task.completed` | 子任务完成 |
| `task.failed` | 子任务失败 |
| `task.cancelled` | 子任务被取消 |
| `extension.installed` | Extension 安装完成 |
| `extension.uninstalled` | Extension 卸载完成 |
| `extension.enabled` | Extension 启用完成 |
| `extension.disabled` | Extension 禁用完成 |
| `extension.updated` | Extension 更新完成 |
| `extension.error` | Extension 生命周期失败 |
| `skill.matched` | Skill 精确匹配成功 |
| `skill.auto_matched` | Skill 意图匹配成功 |
| `skill.match_suggested` | Skill 低置信度建议 |
| `skill.activated` | Skill 被激活 |
| `skill.deactivated` | Skill 被停用 |

**命名规则：**
- 领域名用单数名词：`action` / `reasoning` / `response` / `session` / `connection`
- 动作用过去式或状态：`started` / `completed` / `failed` / `delta` / `updated`
- 禁止缩写：`connection` 不是 `conn`，`authorization` 不是 `auth`
- 禁止内部代号：内核 topic 不携带 `agent` / `tool` 这类产品侧执行细节，统一用 `action.started` 等能力事件

### 16.4 PolicyInput 字段命名

```rust
pub struct PolicyInput {
    pub subject: String,    // "agent" / "terminal" / "file" / "gateway" / "extension:{id}"
    pub action: String,     // "capability.execute" / "connection.establish" / "data.write"
    pub target: String,     // "/path/to/file" / "bash" / "openai:gpt-4o" / "mcp:server_id:tool_name"
    pub scope: String,      // "session:{id}" / "task:{id}" / "global"
    pub metadata: Value,
}
```

- `target` 不叫 `resource`（`resource` 太泛，`target` 明确表示"操作的目标"）
- `action` 用 `领域.动作` 格式：`capability.execute` / `data.write` / `connection.establish`
- `subject` 用实体名：`agent` / `terminal` / `file` / `gateway`
- 禁止在 `subject` / `action` / `target` 中使用业务层内部 ID（如 `tool_catalog_entry_001`）

---

## 17. 使用手册

面向开发者：拿到内核后，怎么接入自己的功能。

### 17.1 场景：新增一个可调用能力

**目标：** 让 Agent 能调用你的能力（如截图、部署、翻译）。

```rust
// Step 1：定义能力 trait（如果还没有合适的）
pub trait ScreenshotTool: Capability {
    fn capture(&self, region: Option<Rect>) -> Result<ScreenshotResult>;
}

// Step 2：实现 Capability trait
impl Capability for MyScreenshotTool {
    fn id(&self) -> &str { "screenshot" }
    fn kind(&self) -> &str { "capability" }
    fn version(&self) -> SchemaVersion { SchemaVersion { major: 1, minor: 0 } }
    fn metadata(&self) -> &Value { &self.meta }  // 包含 description, input_schema 等
}

// Step 3：注册
tool_registry.register(MyScreenshotTool::new());

// Step 4：能力自动出现在 Context Manager 组装的 LLM 工具列表中
// 无需其他改动
```

### 17.2 场景：新增一个 UI 面板

**目标：** 在界面上挂一个自定义面板。

```text
1. Extension manifest 声明 `contributes.views`、必要的 `menus` 和 `commands`。
2. Extension lifecycle 校验 placement、renderer、权限和资源引用。
3. UI HostView 读取已启用 Extension 的 projection，按白名单 renderer 挂载面板。
4. 面板通过 UI store/useEvent 消费后端投影；不把 UI View 作为 Kernel capability，也不让 manifest 直接执行任意代码。
```

### 17.3 场景：新增一个权限约束

**目标：** 限制某个操作只能在特定条件下执行。

```rust
// Step 1：实现 Constraint trait
struct MaxFileSizeConstraint { limit_bytes: usize }

impl Constraint for MaxFileSizeConstraint {
    fn id(&self) -> &str { "max_file_size" }
    fn priority(&self) -> i32 { 200 }  // 用户级优先级
    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        if input.action == "data.write" {
            let size = input.metadata["byte_size"].as_u64().unwrap_or(0);
            if size > self.limit_bytes as u64 {
                return Some(PolicyDecision::Deny {
                    reason: format!("文件大小 {} 超过限制 {}", size, self.limit_bytes),
                });
            }
        }
        None  // 不适用
    }
}

// Step 2：注册
policy_engine.add(MaxFileSizeConstraint { limit_bytes: 10 * 1024 * 1024 });
```

### 17.4 场景：新增一个 Pipeline 阶段

**目标：** 在能力执行链中插入自定义逻辑。

```rust
// Step 1：实现 Stage trait
struct RateLimitStage { limiter: Arc<RateLimiter> }

impl Stage for RateLimitStage {
    fn id(&self) -> &str { "rate_limit" }
    async fn process(&self, ctx: &mut PipelineContext, next: Next<'_>) -> Result<()> {
        if !self.limiter.try_acquire() {
            return Err(Error::RateLimited);
        }
        next.call().await
    }
}

// Step 2：加入 Pipeline
pipeline.push(RateLimitStage { limiter: Arc::new(RateLimiter::new(60)) });
```

### 17.5 场景：订阅事件

**目标：** 监听系统事件并做出响应。

```rust
// EventBus 必须在 Tokio runtime 内创建
let bus = InMemoryEventBus::new(1000, tokio::runtime::Handle::current());

// 订阅指定 topic（支持 glob 通配符）
let sub_id = bus.subscribe(
    Some(Topic::from("action.completed")),
    None,
    Arc::new(|event| {
        println!("能力执行完成: topic={}, scope={}", event.topic, event.context.scope_key());
    }),
)?;

// 订阅带 scope 过滤的事件
let sub_id = bus.subscribe(
    Some(Topic::from("action.*")),
    Some("session:sess_001".to_string()),
    Arc::new(|event| { /* 只处理特定会话的事件 */ }),
)?;

// 取消订阅
bus.unsubscribe(&sub_id)?;

// 查询历史（返回 SharedEventEnvelope，零 payload 拷贝）
let recent = bus.recent(50);
```

### 17.6 场景：查询已注册的能力

```rust
// 按 id 精确查找
let capability = tool_registry.get("screenshot");

// 列出所有
let all = tool_registry.list();

// 按条件筛选
let dangerous = tool_registry.find(|info| {
    info.metadata["risk_level"].as_str() == Some("high")
});

// 检查是否可用
if tool_registry.is_available("screenshot") {
    // 可以调用
}
```

### 17.7 场景：评估策略

```rust
// 完整评估（返回 Allow / Ask / Deny）
let decision = policy_engine.evaluate(&PolicyInput {
    subject: "agent".into(),
    action: "capability.execute".into(),
    target: "bash".into(),
    scope: "session:sess_001".into(),
    metadata: json!({ "command": "ls -la" }),
});

match decision {
    PolicyDecision::Allow { reason } => { /* 执行 */ }
    PolicyDecision::Ask { prompt, grant_spec } => { /* 弹出确认 */ }
    PolicyDecision::Deny { reason } => { /* 拒绝 */ }
}

// 快捷判断
if policy_engine.is_allowed("agent", "capability.execute", "read_file", "session:sess_001") {
    // 允许
}
```

---

## 18. 一句话原则

```text
Kernel 只管四件事：
能力在哪，怎样执行，如何通知，是否允许。

Navis Go 产品概念在 Kernel 之上生长：
Tool、Task、AgentTimelinePart、SessionChange、Memory、Extension、Skill。
```

---

## 19. Kernel 边界守护规则

Kernel 层是 Navis Go 的基础抽象层，所有业务概念（Tool、Agent、Session、Gateway 等）应在 Kernel 之上构建。为维护此边界，本节定义自动化检测机制和人工审核准则。

### 19.1 禁止导入路径

Kernel 内的任何 `.rs` 文件不得包含以下 `use` 语句（反向依赖检测）：

```text
use crate::agent
use crate::gateway
use crate::mcp
use crate::extension
use crate::ui
use crate::session
use crate::terminal
use crate::file
use crate::tool
use crate::extension
use crate::project
use crate::ai
use crate::security
use crate::foundation
```

**例外**：`#[cfg(test)]` 模块内的测试代码不受此限制（测试需要验证集成行为）。

### 19.2 禁止业务词汇

Kernel 的非注释、非字符串字面量代码中不得出现以下业务域词汇（全词匹配）：

```text
tool, session, agent, provider, extension, extension,
mcp, permission, thinking, terminal, gateway, sandbox
```

**说明**：
- 包含注释（行内 `//`、块 `/* */`）和字符串字面量（`"..."`）的内容自动豁免
- 若确需使用某个词汇（如 `config` 作为通用术语），需在本节白名单中添加说明

**当前白名单**（已在代码中验证为合理用法，暂不触发违规）：
- `config`：通用配置术语，kernel core 层允许作为结构体字段名使用

### 19.3 自动化检测

检测通过两个独立测试函数实现，位于 `src-tauri/src/kernel/boundary_test.rs`：

| 测试函数 | 检测内容 | 运行命令 |
|---------|---------|---------|
| `kernel_does_not_import_business_modules()` | 扫描所有 `.rs` 文件，检查 `use crate::*` 导入 | `cargo test kernel::boundary` |
| `kernel_does_not_contain_business_vocabulary()` | 逐行扫描，检查非注释/字符串中的禁止词汇 | `cargo test kernel::boundary` |

**运行方式**：
```bash
cd src-tauri
cargo test kernel::boundary -- --nocapture
```

**工作原理**：
1. 通过 `env!("CARGO_MANIFEST_DIR")` 获取 kernel 目录路径
2. 递归收集所有 `.rs` 文件（包含子目录）
3. 对每个文件逐行解析，跳过注释和字符串内容
4. 检查代码部分是否包含禁止导入或禁止词汇
5. 发现违规时 panic 并输出完整违规列表（文件路径 + 行号 + 原始行）

### 19.4 CI 集成

**推荐的 CI 配置**（GitHub Actions 示例）：

```yaml
kernel-boundary-check:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        toolchain: stable
    - name: Run kernel boundary tests
      working-directory: src-tauri
      run: cargo test kernel::boundary
```

**PR 门禁规则**：
- 所有修改 `src-tauri/src/kernel/` 下文件的 PR 必须通过 `kernel::boundary` 测试
- 测试失败时 PR 自动标记为 blocked

### 19.5 新增 Kernel 原语审批准则

当需要向 kernel 层添加新功能时，必须满足以下条件：

1. **通用性**：新功能必须是至少 3 个不同业务模块都需要的抽象，而非单一模块的专属功能
2. **无业务语义**：新类型、trait、函数名不得包含业务域词汇（§19.2 禁止词汇）
3. **单向依赖**：新代码只能依赖 kernel 内部模块或外部 crate，不能依赖 `crate::agent`、`crate::tool` 等业务层
4. **文档更新**：每次向 kernel 添加新原语时，必须更新本文件相应章节（§2-§17）
5. **边界测试覆盖**：新文件必须自动被 `boundary_test.rs` 的递归扫描覆盖（无需额外配置）

**违反边界时的补救流程**：
1. 确认该功能是否真的需要在 kernel 层（优先考虑提升到业务层）
2. 若确需在 kernel 层，说明理由并在 `boundary_test.rs` 的白名单中添加注释说明
3. 在 PR 描述中附上边界守护审核说明

### 19.6 HostView Contract 独立性保证

当前前端扩展 UI 不再依赖本地组件注入注册表；真实入口是 `contributes.views`，由宿主 HostView surface 与内置 renderer 承接。Kernel 仍然不感知 DOM、布局、placement 或 renderer 字符串。

| 层级 | 当前事实 | 代码位置 |
|------|---------|---------|
| UI 域（前端） | 负责 surface、placement、view 打开状态与 renderer 选择 | `src/components/HostView/*`、`src/stores/app.ts`、`src/stores/menu-actions.ts` |
| Extension 域（后端） | 负责解析 `contributes.views`、维护 extension store、做 view 可渲染性 gate | `extension/host_view.rs`、`extension/store.rs`、`ui/extensions.rs` |
| Kernel（基础设施） | 只提供 Registry / Pipeline / EventBus / Policy 通用原语，不承载 renderer 语义 | `kernel/*` |

**关键保证**：
- HostView contract 是 UI 域和 extension 域之间的事实约定，不是新的 Kernel 原语。
- extension 只能声明 view、menu、command、placement、renderer 等 contract；是否可渲染、如何落到具体 surface，由宿主决定。
- Kernel 不感知 `rightWorkspace`、`chatAside`、`bottomDrawer`、`settingsSection` 等 placement，也不感知 `host:panel` 这类 renderer ID。
- 未来新增 placement 或 renderer 时，只修改 UI Framework 与 extension host；Kernel 继续保持无 UI 语义。

