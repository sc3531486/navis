# 01 - Logger 日志系统 详细设计

> 模块编号：01 | 层级：系统设施层
> 依赖：无（最底层模块）
> 被依赖：所有模块

---

## 一、模块概述

### 1.1 定位

Logger 是整个应用的日志基础设施，基于 Rust `tracing` 生态构建，为所有模块提供统一的日志记录、分级、轮转、脱敏、导出能力。

### 1.2 技术选型

```
基础框架：tracing + tracing-subscriber + tracing-appender（第三方）
├── tracing               — 核心日志框架（Span/Event/Level）
├── tracing-subscriber    — 订阅者框架（Layer 叠加、过滤、格式化）
└── tracing-appender      — 文件输出与轮转

自研扩展（4 个自定义 Layer）：
├── MaskingLayer          — 敏感信息脱敏
├── AuditLayer            — tracing 审计观察日志通道
├── QueryLayer            — 前端查询用内存环形缓冲
└── Export                — 日志导出（JSON/Text/CSV）
```

### 1.3 职责边界

```
Logger 负责：
├── 日志记录（分级：DEBUG / INFO / oARN / ERROR / TRACE）
├── 日志格式化（tracing-subscriber::fmt）
├── 日志轮转（tracing-appender::rolling）
├── 敏感信息脱敏（MaskingLayer）
├── 日志输出（控制台、文件、前端查看器）
├── 日志导出（供用户排查问题）
└── 审计观察日志（AuditLayer，`audit=true` tracing 事件独立落盘）

Logger 不负责：
├── 业务事件分发 → `crate::kernel::EventBus`
├── 结构化业务审计事实源 → `crate::kernel::AuditRecorder` / `AuditSink`
├── 错误处理策略 → 各模块自身
├── 性能指标采集 → 各业务模块或运行时约束
└── 前端日志展示 → UI 层（Logger 只提供数据）
```

---

## 二、架构设计

### 2.1 子模块划分

```
logger/
├── mod.rs              # 模块入口、tracing 初始化
├── masking.rs          # MaskingLayer（自定义 tracing Layer，敏感信息脱敏）
├── audit.rs            # AuditLayer（自定义 tracing Layer，审计观察日志）
├── query.rs            # QueryLayer（自定义 tracing Layer，前端查询）
└── export.rs           # 日志导出（JSON/Text/CSV）

// 以下由第三方 crate 提供，无需自研：
// - 日志级别    → tracing::Level
// - 格式化      → tracing_subscriber::fmt::Layer
// - 过滤        → tracing_subscriber::filter::EnvFilter
// - 文件输出    → tracing_appender::rolling
// - 异步写入    → tracing_appender::non_blocking
// - 宏          → tracing::debug! / info! / warn! / error!
```

### 2.2 架构图

```
日志写入方（所有模块）
     │
     │  tracing::info!("..." , field = value)
     ▼
┌─────────────────────────────────────────────────────────┐
│              tracing-subscriber Registry                  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │          EnvFilter（过滤 Layer）                   │   │
│  │  level=INFO,gateway=DEBUG,agent=TRACE             │   │
│  └──────────────────────┬───────────────────────────┘   │
│                         │                                │
│         ┌───────────────┼───────────────┐                │
│         ▼               ▼               ▼                │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐        │
│  │ fmt::Layer  │  │MaskingLayer│  │AuditLayer  │        │
│  │ 格式化输出  │  │ 脱敏处理    │  │ 审计记录    │        │
│  │ (第三方)    │  │ (自研)      │  │ (自研)      │        │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘        │
│        │               │               │                │
│        ▼               ▼               ▼                │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐        │
│  │ non_blocking│  │ QueryLayer │  │ 审计文件    │        │
│  │ 异步写入    │  │ 前端查询    │  │ (append-only│        │
│  │ (第三方)    │  │ (自研)      │  │  独立文件)  │        │
│  └─────┬──────┘  └─────┬──────┘  └────────────┘        │
│        │               │                                │
│        ▼               ▼                                │
│  ┌────────────┐  ┌────────────┐                         │
│  │ rolling    │  │ Tauri Event│                         │
│  │ 按天轮转   │  │ 前端订阅    │                         │
│  │ (第三方)    │  │            │                         │
│  └────────────┘  └────────────┘                         │
└─────────────────────────────────────────────────────────┘
```

### 2.3 Cargo.toml 依赖

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json", "registry"] }
tracing-appender = "0.2"
```

---

## 三、数据模型

### 3.1 日志级别（使用 tracing 内置）

```rust
// tracing::Level 直接使用，无需自定义
// TRACE < DEBUG < INFO < oARN < ERROR

// Span（结构化日志上下文，tracing 原生）
tracing::info_span!("gateway_request", request_id = %req_id, model = %model);
```

### 3.2 审计日志条目（kernel::audit 模块）

```rust
// --- AuditRecord：结构化审计记录（唯一事实源） ---
pub struct AuditRecord {
    pub id: String,                         // UUID v7
    pub schema_version: i32,                // 模式版本（当前 1）
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub scope: String,                      // 来源作用域
    pub source: String,                     // 操作来源标识
    pub operation_id: String,               // 操作标识
    pub action: String,                     // 操作类型
    pub policy_decision: Option<Value>,     // 策略决策（沙箱/权限结果）
    pub duration_ms: Option<i64>,
    pub input_digest: AuditDigest,          // 输入摘要
    pub output_digest: AuditDigest,         // 输出摘要
    pub status: AuditStatus,
    pub created_at: DateTime<Utc>,
}

// --- AuditStatus：审计状态枚举 ---
pub enum AuditStatus {
    Success,
    Failed,
    Truncated,
    Retried,
    Cancelled,
}

// --- AuditDigest：输入/输出摘要，零堆分配优化 ---
pub enum AuditDigest {
    Truncated { text: String, original_bytes: usize, truncated: bool },
    Metadata { fields: Vec<FieldMeta> },
    Redacted { content_type: String },
    None,
}

pub struct FieldMeta {
    pub name: String,
    pub value_type: String,                 // "null" / "boolean" / "number" / "string" / "array" / "object"
    pub byte_size: usize,
}

// --- AuditSink trait：审计存储后端抽象 ---
pub trait AuditSink: Send + Sync {
    fn record(&self, record: &AuditRecord) -> KernelResult<()>;
    fn record_shared(&self, record: Arc<AuditRecord>) -> KernelResult<()>;
}

// --- 内置 Sink 实现 ---
// InMemoryAuditSink   — 内存存储（测试用）
// BufferedAuditSink   — 有界缓冲 + 批量刷写（生产用）
//   默认：容量 8192、批大小 64、刷写间隔 100ms

// --- AuditRecorder：统一记录器，可选后端 ---
pub struct AuditRecorder {
    sink: Option<Arc<dyn AuditSink>>,
    counters: AuditCounters,  // attempted / succeeded / failed（AtomicU64）
}
```

**AuditDigest 零堆分配设计**：`estimate_byte_size(value)` 函数替代了 `value.to_string().len()` 方案，遍历 `serde_json::Value` 树形结构直接计算序列化后字节数，无需分配中间 `String`。精度：字符串用 `str::len()`，整数用数学数位计算，浮点用 f64 估算（±1-2 字节），容器递归加 JSON 分隔符。

### 3.3 日志配置

```rust
struct LoggerConfig {
    // 全局过滤级别
    level: String,                      // tracing 过滤指令（默认 "info"）
                                        // 支持："info", "gateway=debug,agent=trace"

    // 控制台输出
    console_enabled: bool,              // 是否输出到控制台（默认 true）

    // 文件输出
    file_enabled: bool,                 // 是否输出到文件（默认 true）
    file_dir: PathBuf,                  // 日志目录（默认 ~/.navis/logs/）
    file_prefix: String,                // 文件名前缀（默认 "navis"）

    // 轮转策略（tracing-appender rolling）
    rotation: RotationStrategy,         // 按天/按小时/按大小

    // 脱敏
    masking_enabled: bool,              // 是否启用脱敏（默认 true）

    // 审计
    audit_enabled: bool,                // 是否启用审计（默认 true）
    audit_dir: PathBuf,                 // 审计日志目录（默认 ~/.navis/logs/audit/）
}

enum RotationStrategy {
    Daily,      // 按天轮转（tracing_appender::rolling::daily）
    Hourly,     // 按小时轮转（tracing_appender::rolling::hourly）
}
```

---

## 四、接口定义

### 4.1 Rust API（内部使用）

```rust
// 初始化（应用启动时调用一次）
Logger::init(config: LoggerConfig) -> Result<()>;

// 日志记录（直接使用 tracing 宏，无需自定义宏）
tracing::debug!("Model request completed, tokens={}", tokens);
tracing::info!("oorkspace switched to {}", path);
tracing::warn!("Token usage at {}%", ratio);
tracing::error!("Tool call failed: {}", error);

// 结构化日志（tracing 原生支持）
tracing::info!(
    request_id = %req_id,
    model = %model,
    duration_ms = %duration,
    "Gateway request completed"
);

// Span（追踪上下文，自动记录进入/退出时间）
let span = tracing::info_span!("agent_task", session_id = %sid);
let _guard = span.enter();
// span 内的所有日志自动携带 session_id

// 审计日志（kernel::audit 模块）
// 优先使用 record_owned()，避免克隆
let record = AuditRecord::new(&context, operation_id, action, status);
recorder.record_owned(record)?;               // 推荐：转移所有权
// recorder.record(&record)?;                 // #[deprecated] - 会 clone，仅兼容旧代码

// 有界缓冲记录（生产环境，不阻塞调用方）
let buffered_recorder = AuditRecorder::new(Arc::new(BufferedAuditSink::new(backend)?));

// 查看审计统计
let stats = recorder.stats();
// AuditStats { enabled, attempted_records, succeeded_records, failed_records }

// 日志查询（供前端查看器，通过 QueryLayer）
Logger::query(filter: LogFilter) -> Vec<LogEntry>;
Logger::tail(lines: u32) -> Vec<LogEntry>;

// 日志导出
Logger::export(filter: LogFilter, format: ExportFormat, path: &Path) -> Result<()>;

// 日志清理
Logger::cleanup(older_than_days: u32) -> Result<u64>;
```

### 4.2 IPC 命令（前端调用）

```typescript
// 查询日志
logger.query(filter: {
  level?: 'DEBUG' | 'INFO' | 'oARN' | 'ERROR';
  module?: string;
  sessionId?: string;
  startTime?: string;
  endTime?: string;
  keyword?: string;
  limit?: number;
  offset?: number;
}): Promise<{ entries: LogEntry[]; total: number }>;

// 导出日志
logger.export(filter: LogFilter, format: 'json' | 'text' | 'csv', path?: string): Promise<string>;

// 获取日志统计
logger.stats(): Promise<{
  totalSize: number;
  fileCount: number;
  oldestLog: string;
  newestLog: string;
}>;

// 清理日志
logger.cleanup(olderThanDays?: number): Promise<{ cleanedFiles: number; freedSize: number }>;

// 获取审计日志
logger.getAudit(filter?: {
  action?: string;
  actor?: string;
  startTime?: string;
  endTime?: string;
  limit?: number;
}): Promise<AuditEntry[]>;
```

---

## 五、日志格式

### 5.1 控制台格式（tracing-subscriber::fmt）

```
2026-06-01T14:32:01.123Z  INFO gateway: Gateway request completed request_id=req_abc123 model=claude-sonnet-4-6 duration=1234ms
2026-06-01T14:32:02.456Z ERROR agent: Tool call failed session_id=sess_xyz tool=write error="Permission denied"
2026-06-01T14:32:03.789Z  oARN sandbox: Command blocked by rules command="sudo rm -rf /tmp/test" rule="sudo.*rm"
```

### 5.2 文件格式（JSON，tracing-subscriber::fmt::json）

```json
{"timestamp":"2026-06-01T14:32:01.123Z","level":"INFO","target":"gateway","fields":{"request_id":"req_abc123","model":"claude-sonnet-4-6"},"message":"Gateway request completed"}
```

### 5.3 审计日志格式（自定义）

```
[2026-06-01T14:32:01.123Z] [AUDIT] actor=agent action=Fileorite target=./src/main.ts result=allowed session=sess_xyz
[2026-06-01T14:32:02.456Z] [AUDIT] actor=agent action=CommandExecute target="sudo apt update" result=denied reason="Command blocked by rules"
```

---

## 六、MaskingLayer（自研）

### 6.1 实现方式

```rust
/// MaskingLayer 是一个 tracing Layer，在事件写入前对消息内容进行脱敏处理
struct MaskingLayer {
    patterns: Vec<Regex>,       // 脱敏正则模式
}

impl<S: tracing::Subscriber> tracing::Layer<S> for MaskingLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing::Context<'_, S>) {
        // 1. 提取事件消息
        // 2. 对消息内容执行正则脱敏
        // 3. 将脱敏后的内容传递给下一层
    }
}
```

### 6.2 内置脱敏规则

```rust
MASKING_PATTERNS = [
    // API Keys
    r"(sk-[a-zA-Z0-9]{20,})",           // OpenAI style
    r"(sk-ant-[a-zA-Z0-9]{20,})",       // Anthropic style
    r"(AKIA[0-9A-Z]{16})",              // AoS Access Key

    // 通用密钥模式
    r"(password|passwd|pwd|secret|token|key|auth)\s*[:=]\s*['\"]?([^\s'\"]+)",

    // JoT
    r"(eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*)",

    // 私钥
    r"(-----BEGIN\s+(RSA\s+)?PRIVATE KEY-----)",
]
```

### 6.3 脱敏输出示例

```
原始：API Key: sk-abc123def456ghi789jkl012mno
脱敏：API Key: sk-abc***jkl012mno

原始：password="mySecretPassword123"
脱敏: password="***"
```

---

## 七、AuditLayer（自研）

### 7.1 实现方式

```rust
/// AuditLayer 是一个 tracing Layer，将标记为 audit=true 的事件独立写入观察日志文件。
/// 它不是结构化业务审计事实源；事实源统一通过 kernel::AuditRecorder / AuditSink。
struct AuditLayer {
    writer: Arc<Mutex<Auditoriter>>,    // 观察日志文件写入器（append-only）
}

impl<S: tracing::Subscriber> tracing::Layer<S> for AuditLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing::Context<'_, S>) {
        // 只处理带有 audit=true 标记的事件
        // 提取 actor/action/target/result 字段
        // 写入独立的审计观察日志文件
    }
}

// 使用方式（需要额外日志观察的模块中）:
tracing::info!(
    audit = true,
    actor = "agent",
    action = "Fileorite",
    target = "./src/main.ts",
    result = "allowed",
    "Audit: file write"
);
```

### 7.2 审计观察日志特性

```
├── append-only 模式（不可删除，不可修改）
├── 独立文件（不与普通日志混合）
├── 每日轮转（audit.2026-06-01.log）
└── 启动时自动清理超过 90 天的审计文件
```

---

## 八、观察职责分工：tracing / AuditRecorder / ExecutionObservationSink

审计与运行时观察由三条独立通道承载，各司其职：

```
┌─────────────────────────────────────────────────────────────────────┐
│                        写入方（所有模块）                            │
├─────────────────────┬──────────────────────┬────────────────────────┤
│ tracing span/event  │ AuditRecorder        │ ExecutionObservationSink│
│ （Logger 层）        │ （kernel::audit 层）  │ （kernel::observability）│
├─────────────────────┼──────────────────────┼────────────────────────┤
│ 日志文件 / 控制台    │ 结构化审计事实源      │ 程序化消费              │
│ 人类可读诊断         │ 合规 / 追溯 / 策略验证 │ UI 实时投影 / 指标采集   │
│ EnvFilter 过滤       │ AuditSink 后端持久化   │ EventBus 桥接可选      │
│ AuditLayer（audit=  │ BufferedAuditSink    │ ExecutionObserver 闭包  │
│  true 标记独立落盘） │ InMemoryAuditSink    │ SharedArc 零拷贝分发    │
└─────────────────────┴──────────────────────┴────────────────────────┘
```

**ExecutionObservationSink**（`kernel::observability/mod.rs`）用于执行流程的程序化观察：

```rust
// 观察回调：接收 SharedExecutionEvent（triomphe::Arc，引用计数零拷贝）
pub type ExecutionObserver = Arc<dyn Fn(SharedExecutionEvent) + Send + Sync>;

pub struct ExecutionObservationSink {
    observer: ExecutionObserver,
    is_enabled: bool,
}

// 构造方式
ExecutionObservationSink::disabled()                    // 空操作（默认）
ExecutionObservationSink::from_fn(|event| { ... })     // 自定义闭包
ExecutionObservationSink::event_bus(event_bus)          // 桥接到 EventBus（序列化后按 topic 发布）
```

**ExecutionEvent 类型体系**：

```rust
pub enum ExecutionEventKind {
    RunStarted | RunCompleted | RunFailed | RunCancelled,
    StageStarted | StageDelta | StageCompleted | StageFailed,
    CapabilityCalled | CapabilityCompleted | CapabilityFailed,
}
// 每个 kind 映射到 EventBus topic（如 "execution.run.started"）

pub struct ExecutionEvent {
    pub id: String,
    pub kind: ExecutionEventKind,
    pub run_id: String,
    pub stage_id: Option<String>,
    pub capability_id: Option<String>,
    pub context: KernelContext,
    pub sequence: u64,
    pub message: Option<String>,
    pub error_kind: Option<KernelErrorKind>,
    pub duration_ms: Option<u64>,
    pub payload: SharedArc<Value>,   // 零拷贝共享载荷（null_payload() 静态单例优化）
    pub created_at: DateTime<Utc>,
}
```

**选择指南**：
| 场景 | 使用 |
|------|------|
| 人工排查、日志文件 | `tracing::info!(...)` |
| 合规审计、策略记录、历史回溯 | `AuditRecorder::record_owned(...)` |
| UI 进度条、实时状态面板 | `ExecutionObservationSink` |
| 运行时指标（metrics counter/gauge） | `ExecutionObservationSink` 或自定义 tracing subscriber |

---

## 九、QueryLayer（自研）

### 9.1 实现方式

```rust
/// QueryLayer 是一个 tracing Layer，维护内存环形缓冲区供前端查询
struct QueryLayer {
    buffer: Arc<Mutex<RingBuffer<LogEntry>>>,   // 环形缓冲区（1000 条）
}

impl<S: tracing::Subscriber> tracing::Layer<S> for QueryLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing::Context<'_, S>) {
        // 1. 将事件转换为 LogEntry
        // 2. 写入环形缓冲区
    }
}
```

### 9.2 查询能力

```
Logger::query(filter)  → 从环形缓冲区 + 历史文件中查询
Logger::tail(lines)    → 获取最近 N 条日志
```

QueryLayer 不提供实时订阅，也不是 EventBus。需要实时 UI 的日志 tail 时，由 UI 侧周期查询或由专门的 Stream 通道承接；业务事实通知仍只走 `crate::kernel::EventBus`。

---

## 十、日志轮转策略（tracing-appender）

```rust
// 直接使用 tracing-appender 的 rolling 模块
let file_appender = match config.rotation {
    RotationStrategy::Daily => {
        tracing_appender::rolling::daily(&config.file_dir, &config.file_prefix)
    }
    RotationStrategy::Hourly => {
        tracing_appender::rolling::hourly(&config.file_dir, &config.file_prefix)
    }
};

// 配合 non_blocking 实现异步写入
let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
```

### 10.1 轮转规则

```
触发条件：
├── Daily 模式：日期变更时自动创建新文件
└── Hourly 模式：小时变更时自动创建新文件

文件命名（tracing-appender 自动生成）：
navis.2026-06-01     ← 今天的日志
navis.2026-05-31     ← 昨天的
navis.2026-05-30     ← 前天的
```

### 10.2 清理策略

```
应用启动时：
├── 删除超过 max_days 的日志文件
├── 删除损坏/空的日志文件
└── 记录清理结果到日志
```

---

## 十一、初始化流程

```rust
pub fn init(config: LoggerConfig) -> Result<()> {
    // 1. 创建过滤器（tracing-subscriber::filter）
    let env_filter = EnvFilter::try_new(&config.level)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // 2. 创建格式化 Layer（tracing-subscriber::fmt）
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    // 3. 创建文件 Layer（tracing-appender）
    let file_appender = tracing_appender::rolling::daily(&config.file_dir, &config.file_prefix);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = tracing_subscriber::fmt::layer()
        .json()                                          // JSON 格式
        .with_writer(non_blocking);

    // 4. 创建自研 Layers
    let masking_layer = MaskingLayer::new();
    let audit_layer = AuditLayer::new(&config.audit_dir)?;
    let query_layer = QueryLayer::new(1000);             // 环形缓冲区 1000 条

    // 5. 叠加所有 Layers
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)                                 // 控制台输出
        .with(file_layer)                                // 文件输出
        .with(masking_layer)                             // 脱敏
        .with(audit_layer)                               // 审计
        .with(query_layer)                               // 前端查询
        .init();

    Ok(())
}
```

---

## 十二、错误处理

| 场景 | 处理策略 |
|------|----------|
| 日志文件创建失败 | tracing-appender 自动降级，不阻塞主流程 |
| 日志文件写入失败 | non_blocking 模式自动重试 |
| 磁盘空间不足 | 发出 logger.disk.warning 事件 |
| 脱敏正则匹配失败 | 原样输出，不阻塞日志记录 |
| 日志目录无权限 | 降级到用户目录下创建 |
| QueryLayer 缓冲区满 | 环形自动淘汰旧日志 |

---

## 十三、事件定义

```typescript
type LoggerEvents = {
  'logger.rotation':      { fileName: string; newSize: number }
  'logger.cleanup':       { cleanedFiles: number; freedSize: number }
  'logger.disk.warning':  { freeSpace: number; threshold: number }
}
```

---

## 十四、EventBus 溢出监控（kernel::event 模块）

`InMemoryEventBus` 在调度器队列满时记录溢出，事件仍写入 history ring-buffer 但不会分发给 handler：

```rust
pub struct EventBusStats {
    pub queue_len: usize,           // 当前队列长度
    pub queue_capacity: usize,      // 队列容量
    pub dispatcher_running: bool,
    pub overflow_count: u64,        // 累计溢出事件数
}

impl EventBusStats {
    /// 如果有任何事件因队列满被丢弃，返回 true。
    pub fn has_overflow(&self) -> bool {
        self.overflow_count > 0
    }
}
```

**溢出时的降级策略**：
- 溢出事件仍进入 `history` ring-buffer，`EventBus::recent()` 可查询
- Handler 不会收到溢出事件，由 `overflow_count` 追踪丢弃数量
- 溢出时自动记录 `tracing::warn!`（含 event_id、topic、当前 overflow_count）
- UI 层可通过 `EventBus::stats().has_overflow()` 检测并在状态栏提示用户

---

## 十五、性能指标

| 指标 | 要求 | 说明 |
|------|------|------|
| 单条日志写入 | < 1ms | non_blocking 模式，不阻塞调用方 |
| 脱敏处理 | < 0.1ms/条 | 正则匹配效率 |
| 日志查询 | < 100ms | 1 万条以内 |
| 内存占用 | < 5MB | 环形缓冲区 + tracing 内部缓冲 |
| 文件轮转 | < 50ms | tracing-appender 内部处理 |

---

## 十六、测试策略

```
单元测试：
├── MaskingLayer 脱敏规则匹配（各模式）
├── AuditLayer 审计观察日志记录
├── QueryLayer 环形缓冲区操作
├── Export 导出格式正确性
└── LoggerConfig 解析

集成测试：
├── tracing 初始化完整链路
├── 多 Layer 叠加不冲突
├── 并发写入不丢日志
├── 轮转后文件正确切割
├── 前端 IPC 查询/订阅端到端
├── 审计日志独立文件写入
└── 脱敏在所有输出目标生效

性能测试：
├── 高并发写入吞吐量
├── 脱敏正则性能
└── 大量日志查询性能
```

---

## 十七、与旧设计对比

| 维度 | 旧设计（全自研） | 新设计（tracing 生态） |
|------|----------------|---------------------|
| 自研模块数 | 11 个 | 4 个（Masking/Audit/Query/Export） |
| 日志级别 | 自定义 LogLevel | tracing::Level（5 级） |
| 结构化日志 | 自定义 LogEntry + LogContext | tracing 原生 Span/Field |
| 格式化 | 自定义 Formatter | tracing-subscriber::fmt |
| 文件输出 | 自定义 oriter | tracing-appender |
| 轮转 | 自定义 Rotation | tracing-appender::rolling |
| 过滤 | 自定义 Filter | tracing-subscriber::filter::EnvFilter |
| 异步写入 | 自定义 Ring Buffer | tracing-appender::non_blocking |
| 宏 | 自定义 log_debug! 等 | tracing::debug! 等 |
| 生态兼容 | 无 | 兼容所有 tracing 生态 crate（tower-http, axum, sqlx 等） |
