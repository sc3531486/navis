# 02b - Stream 流式数据通道 详细设计

> 模块编号：02b | 大域：foundation/stream
> 依赖：foundation/ipc, kernel/EventBus（Tauri Channel API）
> 被依赖：project/session, tool/terminal, ai/gateway, ai/agent, extension

---

## 一、模块概述

### 1.1 定位

Stream 模块是 Navis Go 所有流式数据通信的**唯一出口**。它位于 `foundation/stream`，封装 Tauri 2 原生 `Channel<T>` API、tokio mpsc 通道、节流发射器，为所有业务模块提供统一的流式数据接入层。Terminal、Agent、Gateway、Extension 等模块统一通过此模块接入流，不各自实现。

### 1.2 核心问题

Stream 模块需要收束所有流式数据处理逻辑：Gateway 的 tokio mpsc channel、Terminal 的高频输出节流、Agent 的流式转发都必须接入同一套类型和生命周期。这避免：

- **代码重复**：ThrottledEmitter 的缓冲、合并、flush 逻辑在多个模块中重复实现
- **类型不一致**：各模块对 StreamChunk、流 ID、流状态的定义不统一
- **流管理分散**：无法统一查看活跃流列表、流的吞吐统计、调试信息
- **扩展困难**：新增流式场景时需要在多个模块中重复实现基础能力

统一 Stream 模块将所有流式基础设施收归一处，业务模块只关心"发什么数据"，不关心"怎么发"。

### 1.3 职责边界

```
负责：
├── EventBus vs Stream 分治规则
├── 统一 Stream 模块（后端进程内 tokio mpsc + 跨进程 Tauri Channel）
├── StreamSource 开放结构体（已知常量 + 未知字符串，扩展可扩展）
├── StreamChannel Builder 模式（流式通道构建标准化）
├── ThrottledEmitter 节流模式（高频输出标准方案）
├── StreamIndex 流索引（全局活跃流管理）
├── 前端流订阅规范（useTauriStream hook 使用指南）
└── 扩展 Stream 支持（Extension runtime API + extension.json 声明）

不负责：
├── Tauri Channel 底层实现 → Tauri 2 框架原生提供
├── 具体业务流的创建 → 各业务模块通过 StreamChannel::builder() 接入
└── Gateway 的流式发送 → Gateway 通过 stream::StreamSender 发送，属于 Gateway 内部逻辑
```

---

## 二、分治规则（EventBus vs Stream）

### 2.1 决策树

```
数据是否是"一段连续数据流"？
├── 否 → EventBus（离散状态变更）
│   例：created / closed / switched / failed / progress
│
└── 是 → 数据频率如何？
    ├── 低频（< 1次/秒）→ 可用 EventBus（带 throttle 保护）
    │   例：lsp.indexing.progress / agent.task.progress
    │
    └── 高频（> 1次/秒）→ 必须用 Stream 模块
        例：terminal.output / agent.message.stream / agent.thinking.chunk
```

### 2.2 分类表

| 数据类型 | 频率 | 通道 | 理由 |
|---------|------|------|------|
| 终端输出（stdout/stderr） | 10-2000行/秒 | **Stream** | 连续数据流，需要背压 |
| LLM 流式 token | 10-50 token/秒 | **Stream** | 连续数据流，前端逐字渲染 |
| Extended Thinking 内容 | 10-50 token/秒 | **Stream** | 同 LLM token |
| 终端创建/关闭 | 偶发 | EventBus | 离散状态变更 |
| 命令开始/完成/失败 | 偶发 | EventBus | 离散状态变更 |
| 会话切换/创建/删除 | 偶发 | EventBus | 离散状态变更 |
| 模型切换/离线/上线 | 偶发 | EventBus | 离散状态变更 |
| LSP 索引进度 | 每文件1次 | EventBus | 低频进度通知 |
| 下载进度 | 每百分比1次 | EventBus | 低频进度通知 |
| Agent 任务进度 | 偶发 | EventBus | 状态变更通知 |
| Task Sidechain 子任务进度 | 偶发 | EventBus | 状态变更通知 |

### 2.3 原则总结

```
EventBus = "发生了什么事"（状态变更、生命周期事件、错误通知）
Stream   = "这是数据的一段"（连续文本、终端输出、逐 token 推送）
```

---

## 三、统一 Stream 模块设计

### 3.1 模块定位

stream 模块是 Navis Go 所有流式数据通信的唯一出口。
其他模块（Terminal/Agent/Gateway/Extension）统一通过此模块接入流，不各自实现。

### 3.2 架构

```
foundation/stream/
├── mod.rs        # 统一入口，重导出所有公开类型
├── types.rs      # StreamSource（开放结构体）、StreamChunk、stream_kind 常量
├── sender.rs     # StreamSender/StreamReceiver（后端进程内，tokio mpsc）
├── channel.rs    # StreamChannel + Builder（跨进程，Tauri Channel + ThrottledEmitter）
├── emitter.rs    # ThrottledEmitter（通用节流组件）
└── index.rs      # StreamIndex（活跃流索引）
```

**整体数据流**：

```
┌─ 数据生产者（后端 Rust）─────────────────────────────────────┐
│                                                               │
│  Gateway  → stream::mpsc_pair("gateway", 64) → StreamSender  │
│  Terminal → StreamChannel::builder(ch).source(...).build()    │
│  Agent    → StreamReceiver 接收 + StreamChannel 推送到前端     │
│  Extension   → StreamChannel::builder(ch).source(...).build()    │
│  自定义   → StreamChannel::builder(ch).source(StreamSource    │
│              ::new("xxx", "id")).build()                       │
│                                                               │
└───────────────────────────────────────────────────────────────┘
                                                        │
                                             channel.send()
                                                        ▼
┌── IPC 层（Tauri 原生 / tokio mpsc）─────────────────────────┐
│                                                               │
│  Tauri Channel<T>  ── 跨进程，二进制直传，内建序列号和背压     │
│  tokio mpsc        ── 后端进程内，原生背压                     │
│                                                               │
└─────────────────────────────┬─────────────────────────────────┘
                              │ onmessage / recv()
                              ▼
┌─ 数据消费者（前端 Solid.js / 后端模块）─────────────────────┐
│                                                               │
│  前端：channel.onmessage → Solid Signal → View 渲染           │
│  ├── Terminal View → 渲染终端输出                             │
│  ├── Chat View → 渲染 LLM 流式回复                            │
│  └── Thinking View → 渲染思考过程                              │
│                                                               │
│  后端：StreamReceiver.recv() → 处理（上下文压缩等）            │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

### 3.3 三种使用场景

**场景 1：后端进程内（Gateway → Agent）**

```rust
// Gateway 侧：创建发送端
let (sender, receiver) = stream::mpsc_pair("gateway", 64);
// sender: StreamSender — Gateway 持有，发送 StreamChunk
// receiver: StreamReceiver — 传给 Agent，接收 StreamChunk

// Gateway 发送
sender.send(StreamChunk { ... }).await?;

// Agent 接收
while let Some(chunk) = receiver.recv().await {
    // 处理 chunk，可选择转发到前端或内部消费
}
```

**场景 2：跨进程推送（Agent/Terminal → 前端）**

```rust
// Tauri Command handler 中
#[tauri::command]
async fn terminal_create_pty(
    options: PtyOptions,
    channel: Channel<Value>,
) -> Result<String> {
    let pty_id = create_pty(options).await?;

    let mut stream_channel = StreamChannel::builder(channel)
        .source(StreamSource::new(stream_kind::TERMINAL, &pty_id))
        .label("PTY output")
        .throttle(Duration::from_millis(50))
        .build();

    // 推送 PTY 输出
    while let Some(output) = pty.recv().await {
        stream_channel.send(json!({ "data": output.data, "stream": output.stream })).await?;
    }
    Ok(pty_id)
}
```

**场景 3：扩展自定义流**

```rust
// 扩展通过 Extension runtime API 创建的流
let mut stream_channel = StreamChannel::builder(channel)
    .source(StreamSource::new("jira-feed", "123"))
    .label("Jira Issue 实时更新")
    .throttle(Duration::from_millis(100))
    .build();

// 扩展发送数据
stream_channel.send(json!({ "issue_id": "123", "status": "in_progress" })).await?;
```

### 3.4 StreamSource 开放结构体

```rust
/// 流来源标识（开放结构体）
///
/// 已知类型用 stream_kind 常量保证拼写正确，
/// 未知类型直接写字符串，扩展不需要改 Stream 模块源码。
/// 任意模块都能创建自己的流类型。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamSource {
    pub kind: String,   // 流类型（如 "terminal", "agent", "extension", 或任意字符串）
    pub id: String,     // 来源实例 ID（如 pty_id, session_id, extension_id）
}

impl StreamSource {
    pub fn new(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Self { kind: kind.into(), id: id.into() }
    }
}

/// 已知流类型常量（编译期拼写检查）
pub mod stream_kind {
    pub const TERMINAL: &str = "terminal";
    pub const AGENT: &str = "agent";
    pub const GATEWAY: &str = "gateway";
    pub const EXTENSION: &str = "extension";
    pub const THINKING: &str = "thinking";
}

/// 流数据块
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamChunk {
    pub source: StreamSource,
    pub data: Value,
    pub sequence: u64,
    pub is_final: bool,
}
```

**设计要点**：
- `kind` 和 `id` 都是 `String`，不使用枚举，因为扩展可以创建任意类型
- `stream_kind` 模块提供常量，内置模块使用常量保证类型安全
- 扩展直接传字符串 `"jira-feed"` 即可，无需修改 Stream 模块源码

### 3.5 Builder 模式

```rust
StreamChannel::builder(channel)
    .source(StreamSource::new(kind, id))
    .label("描述")
    .throttle(Duration::from_millis(50))
    .build()
```

**Builder 参数说明**：

| 参数 | 必填 | 说明 |
|------|------|------|
| `channel` | 是 | `Channel<Value>` — 由 Tauri Command handler 传入 |
| `source` | 是 | `StreamSource` — 流来源标识 |
| `label` | 否 | 可读描述，用于调试和 StreamIndex 展示 |
| `throttle` | 否 | 节流间隔，默认 50ms；设为 `Duration::ZERO` 可禁用节流 |

**实现**：

```rust
pub struct StreamChannelBuilder {
    channel: Channel<Value>,
    source: Option<StreamSource>,
    label: Option<String>,
    throttle: Option<Duration>,
}

impl StreamChannel {
    pub fn builder(channel: Channel<Value>) -> StreamChannelBuilder {
        StreamChannelBuilder {
            channel,
            source: None,
            label: None,
            throttle: None,
        }
    }
}

impl StreamChannelBuilder {
    pub fn source(mut self, source: StreamSource) -> Self {
        self.source = Some(source);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn throttle(mut self, interval: Duration) -> Self {
        self.throttle = Some(interval);
        self
    }

    pub fn build(self) -> StreamChannel {
        let source = self.source.expect("source is required");
        let throttle = self.throttle.unwrap_or(Duration::from_millis(50));
        StreamChannel::new(self.channel, source, self.label, throttle)
    }
}
```

### 3.6 各模块接入方式

| 模块 | 接入方式 | Source Kind |
|------|---------|-------------|
| Gateway → Agent | `stream::mpsc_pair("gateway", 64)` | `stream_kind::GATEWAY` |
| Agent → 前端 | `StreamChannel::builder(ch).source(...).build()` | `stream_kind::AGENT` |
| Terminal → 前端 | `StreamChannel::builder(ch).source(...).build()` | `stream_kind::TERMINAL` |
| Extension → 前端 | `StreamChannel::builder(ch).source(...).build()` | `stream_kind::EXTENSION` |
| 自定义 → 前端 | `StreamChannel::builder(ch).source(StreamSource::new("xxx", "id")).build()` | 任意字符串 |

**各模块接入示例**：

```
Gateway（发送 LLM 流式数据）：
├── agent_router_stream() 内部创建 stream::mpsc_pair() 获取 StreamSender + StreamReceiver
├── SSE 解析后的每个 chunk 通过 StreamSender::send(chunk) 发出
├── Agent 通过 StreamReceiver 接收
└── ipc_router_stream() 仅在前端 IPC 场景把内部 StreamReceiver 转发到 Tauri Channel

Agent（接收 + 转发到前端）：
├── 通过 stream::StreamReceiver 接收 Gateway 的 StreamChunk
├── 创建 StreamChannel::builder(ch)
│     .source(StreamSource::new(stream_kind::AGENT, session_id))
│     .build()
└── 将接收到的 chunk 通过 StreamChannel 转发到前端

Terminal（PTY 输出推送）：
├── StreamChannel::builder(ch)
│     .source(StreamSource::new(stream_kind::TERMINAL, pty_id))
│     .throttle(Duration::from_millis(50))
│     .build()
├── PTY stdout/stderr 数据通过 StreamChannel.send() 推送
└── ThrottledEmitter 内部自动节流

Extension（扩展流）：
├── 通过 Extension runtime API 创建的 StreamChannel
├── StreamChannel::builder(ch)
│     .source(StreamSource::new("custom-data-feed", extension_id))
│     .build()
└── 扩展 MCP Server 通过授权的 extension stream API 推送数据
```

**Gateway 的两种流模式**：

```
模式 A：跨进程推送到前端（ipc_router_stream）：
├── 签名：ipc_router_stream(&self, request, channel: Channel) -> Result<()>
├── 用途：前端 IPC 场景，逐 token 推送到前端 Chat View
├── 内部调用 agent_router_stream() 获取 StreamReceiver
├── 节流：默认关闭（对话场景逐 token 不节流）
├── 数据格式：{ "data": "<delta>" } 进行中 / { "done": true } 结束 / { "error": "<msg>" } 错误
└── 路径：Gateway → StreamSender → StreamReceiver → channel.send() → 前端 onmessage

模式 B：后端进程内（agent_router_stream）：
├── 签名：agent_router_stream(&self, request) -> Result<StreamReceiver<StreamChunk>>
├── 用途：Agent 或后端内部模块间流式传递，无需前端参与
├── 内部创建 stream::mpsc_pair()
├── 无需跨进程 IPC，tokio mpsc 原生背压已够用
└── 路径：Gateway StreamSender (mpsc) → 调用方 StreamReceiver
```

### 3.7 StreamIndex 流索引

全局活跃流管理，用于调试、监控、查询。StreamChannel 创建时自动加入活动索引，Drop 时自动移除。

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};

pub struct StreamIndex {
    streams: Arc<Mutex<HashMap<String, StreamInfo>>>,
}

#[derive(Clone, Debug)]
pub struct StreamInfo {
    pub stream_id: String,          // 流标识（如 "terminal:term_001"）
    pub source: StreamSource,       // 来源标识
    pub label: Option<String>,      // 可读描述
    pub started_at: DateTime<Utc>,
    pub chunks_sent: u64,
    pub bytes_sent: u64,
    pub is_active: bool,
}

impl StreamIndex {
    pub fn new() -> Self {
        Self { streams: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// 注册一个活跃流
    pub fn register(&self, info: StreamInfo) {
        self.streams.lock().unwrap().insert(info.stream_id.clone(), info);
    }

    /// 注销流
    pub fn unregister(&self, stream_id: &str) {
        self.streams.lock().unwrap().remove(stream_id);
    }

    /// 更新统计
    pub fn record_send(&self, stream_id: &str, bytes: u64) {
        if let Some(info) = self.streams.lock().unwrap().get_mut(stream_id) {
            info.chunks_sent += 1;
            info.bytes_sent += bytes;
        }
    }

    /// 按 source.kind 查询
    pub fn list_by_kind(&self, kind: &str) -> Vec<StreamInfo> {
        self.streams.lock().unwrap().values()
            .filter(|s| s.source.kind == kind && s.is_active)
            .cloned()
            .collect()
    }

    /// 按 session_id 查询（source.id 匹配）
    pub fn list_by_source_id(&self, id: &str) -> Vec<StreamInfo> {
        self.streams.lock().unwrap().values()
            .filter(|s| s.source.id == id && s.is_active)
            .cloned()
            .collect()
    }

    /// 按 stream_id 查询
    pub fn get(&self, stream_id: &str) -> Option<StreamInfo> {
        self.streams.lock().unwrap().get(stream_id).cloned()
    }

    /// 列出所有活跃流（调试用）
    pub fn list_active(&self) -> Vec<StreamInfo> {
        self.streams.lock().unwrap().values()
            .filter(|s| s.is_active)
            .cloned()
            .collect()
    }
}
```

**使用示例**：

```rust
// StreamChannel 创建时自动加入活动索引
let mut stream_channel = StreamChannel::builder(channel)
    .source(StreamSource::new(stream_kind::TERMINAL, &pty_id))
    .label("Terminal PTY output")
    .index(stream_index.clone())
    .build();
// stream_channel 内部自动调用 stream_index.track(...)

// 查询所有终端流
let terminal_streams = stream_index.list_by_kind(stream_kind::TERMINAL);

// 查询某个会话的所有流
let session_streams = stream_index.list_by_session(&session_id);

// stream_channel Drop 时自动从活动索引移除
```

### 3.8 扩展 Stream 支持

扩展如何接入 Stream 体系，通过 Extension runtime API 实现流式数据的生产与消费。

#### 3.8.1 接入方式

```
扩展 Stream 接入方式：
├── 1. 扩展 MCP Server 产出流式数据
│   ├── 通过授权的 extension stream API 创建流式通道
│   ├── extension.stream.create(sessionId?) → streamId
│   ├── extension.stream.send(streamId, data)
│   └── extension.stream.close(streamId)
│
├── 2. 扩展 UI 组件消费流式数据
│   ├── 通过 UI runtime 的 Tauri Channel / event outlet 订阅流
│   ├── extension.stream.subscribe(streamId, onData) → subscriptionId
│   └── extension.stream.unsubscribe(subscriptionId)
│
└── 3. 扩展声明自定义 Stream 通道
    └── extension.json contributes 中声明 stream_channels
```

#### 3.8.2 extension.json 声明示例

```json
{
  "contributes": {
    "stream_channels": [
      {
        "id": "custom-data-feed",
        "description": "实时数据流",
        "dataSchema": { "type": "object", "properties": { "value": { "type": "number" } } }
      }
    ]
  }
}
```

#### 3.8.3 Extension Stream API

扩展通过宿主注入的 `extension.stream` 命名空间访问 Stream 能力。它是沙箱运行时的能力 API，不是后端 EventBus 事件出口，也不是第二套事件系统：

```typescript
extension.stream.create(sessionId?: string): Promise<string>           // 创建流，返回 streamId
extension.stream.send(streamId: string, data: any): Promise<void>      // 发送数据块
extension.stream.close(streamId: string): Promise<void>                // 关闭流
extension.stream.subscribe(streamId: string, onData: (chunk: StreamChunk) => void): string  // 订阅流
extension.stream.subscribeSource(filter: StreamFilter, onData: (chunk: StreamChunk) => void): string // 按来源订阅
extension.stream.unsubscribe(subscriptionId: string): void             // 取消订阅
extension.stream.list(filter?: StreamFilter): Promise<StreamInfo[]>    // 列出活跃流
```

> 注：扩展 Stream API 内部实现基于 Stream 模块。`extension.stream.create()` 在后端创建一个 `StreamChannel::builder(channel).source(StreamSource::new(stream_kind::EXTENSION, extension_id)).build()`，扩展 MCP Server 通过 `send()` 写入数据，前端通过 `subscribe()` 注册 `onmessage` 回调。

`subscribeSource()` 用于扩展视图在不知道 `streamId` 时按来源订阅，例如 Agent 可视化面板可以使用 `{ kind: 'agent', sessionId }` 订阅当前会话的 Agent 流。

#### 3.8.4 扩展 Stream 权限模型

| 权限维度 | 规则 | 说明 |
|---------|------|------|
| 读权限（subscribe） | 宽松 | 扩展默认可以订阅任何 stream |
| 写权限（create/send） | 严格 | 创建 stream 需在 extension.json 中声明 `stream_channels`；扩展只能发送到自己声明的通道 |
| 超时回收 | 自动 | 5 秒无活动的扩展 stream 自动关闭（防止资源泄漏） |

#### 3.8.5 扩展 Stream 通道清单（动态扩展）

§3.6 中的通道清单是内置通道。扩展通过 `contributes.stream_channels` 声明可创建的流式通道类型；启用后由宿主把真实运行中的流加入 StreamIndex 活动索引，禁用/卸载时停止创建新流并清理活动流。StreamIndex 只维护活跃流查询、取消和调试投影，不是 Kernel Registry。

---

## 四、ThrottledEmitter 节流模式

### 4.1 适用场景

高频输出源（终端、编译日志、测试输出）直接通过 Channel 推送会导致 IPC 调用频率过高。ThrottledEmitter 在时间窗口内合并多次输出为单次发送。

### 4.2 通用接口

```rust
/// 节流发射器（所有高频输出源共用）
pub struct ThrottledEmitter<T> {
    interval: Duration,           // 节流窗口（默认 50ms）
    buffer: Vec<T>,               // 缓冲区
    last_flush: Instant,          // 上次刷新时间
    channel: Channel<Value>,      // Tauri Channel
    merge_fn: fn(&[T]) -> Value,  // 合并函数（将多个 chunk 合并为单个 Value）
    source: StreamSource,         // 流来源标识（用于 StreamIndex）
}

impl<T> ThrottledEmitter<T> {
    /// 创建新的节流发射器
    pub fn new(
        channel: Channel<Value>,
        source: StreamSource,
        interval: Duration,
        merge_fn: fn(&[T]) -> Value,
    ) -> Self { ... }

    /// 推送数据到缓冲区，超过间隔自动 flush
    pub fn push(&mut self, item: T) { ... }

    /// 显式刷新（确保数据发出）
    pub fn flush(&mut self) { ... }

    /// 获取关联的 StreamSource（用于调试）
    pub fn source(&self) -> &StreamSource { ... }
}
```

**代码示例**：

```rust
use tauri::ipc::Channel;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub struct ThrottledEmitter {
    interval: Duration,
    buffer: Vec<String>,
    last_flush: Instant,
    channel: Channel<Value>,
    source: StreamSource,
}

impl ThrottledEmitter {
    pub fn new(channel: Channel<Value>, source: StreamSource, interval_ms: u64) -> Self {
        Self {
            interval: Duration::from_millis(interval_ms),
            buffer: Vec::new(),
            last_flush: Instant::now(),
            channel,
            source,
        }
    }

    /// 推送数据到缓冲区，超过间隔自动 flush
    pub fn push(&mut self, data: String) {
        self.buffer.push(data);
        if self.last_flush.elapsed() >= self.interval {
            self.flush();
        }
    }

    /// 显式刷新（确保数据发出，如终端关闭前）
    pub fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        let merged: String = self.buffer.join("");
        let _ = self.channel.send(json!({
            "data": merged,
            "source": {
                "kind": self.source.kind,
                "id": self.source.id,
            },
        }));
        self.buffer.clear();
        self.last_flush = Instant::now();
    }
}
```

### 4.3 节流效果

| 场景 | 无节流 | 50ms 节流 | 降低 |
|------|--------|-----------|------|
| npm install（100行/秒） | 100 send/秒 | ~20 send/秒 | 5x |
| cargo build（500行/秒） | 500 send/秒 | ~20 send/秒 | 25x |
| 大型编译（2000行/秒） | 2000 send/秒 | ~20 send/秒 | 100x |

### 4.4 生命周期管理

```
StreamChannel::builder(channel).source(...).throttle(50ms).build()
    → 内部创建 ThrottledEmitter::new(channel, source, ...)

push(data)
    → ThrottledEmitter::push() → 自动/显式 flush → channel.send()

StreamChannel Drop
    → ThrottledEmitter Drop → 显式 flush 残留数据 → 前端 onmessage 流自动结束
```

---

## 五、前端 Stream 消费

前端消费流式数据有两种方式：底层 Channel 直接使用和 Solid Signal 封装。

### 5.1 方式 1：命令级 Channel（底层直接使用）

```typescript
import { invoke, Channel } from '@tauri-apps/api/core';

const channel = new Channel();
channel.onmessage = (msg) => render(msg);
await invoke('terminal_create_pty', { options, channel });
// invoke 返回后流自动结束（后端 Drop channel）
```

### 5.2 方式 2：Solid Signal 封装（推荐）

```typescript
import { createSignal, onMount, onCleanup } from 'solid-js';
import { invoke, Channel } from '@tauri-apps/api/core';

function useTauriStream<T>(command: string, args: Record<string, unknown>) {
    const [data, setData] = createSignal<T[]>([]);
    const [isStreaming, setIsStreaming] = createSignal(false);
    let channel: Channel<T> | null = null;

    onMount(async () => {
        channel = new Channel<T>();
        channel.onmessage = (msg) => setData(prev => [...prev, msg]);
        setIsStreaming(true);
        try {
            await invoke(command, { ...args, channel });
        } finally {
            setIsStreaming(false);
            channel = null;
        }
    });

    onCleanup(() => {
        channel = null;  // 丢弃引用 → 后端 Channel Drop → 流自动结束
    });

    return { data, isStreaming };
}

// 使用示例
function TerminalPanel({ ptyId }: { ptyId: string }) {
    const { data, isStreaming } = useTauriStream('terminal_subscribe', { ptyId });

    return (
        <div>
            <Show when={isStreaming()}>
                <span>接收中...</span>
            </Show>
            {data().map(chunk => <pre>{chunk.data}</pre>)}
        </div>
    );
}
```

### 5.3 useChannel — 通用 Channel Hook（进阶封装）

封装 Tauri `Channel<T>` 的生命周期，提供两种消费模式：

```typescript
import { useChannel } from '@/lib/stream';
import type { StreamTermination } from '@/lib/stream';

const channel = useChannel<string, { streamId: string }>({
  command: 'ui_stream_example',
  args: { payload: { topic: 'updates' } },
  mode: 'callback',
  completion: 'channel',
  onChunk: (chunk) => consumeChunk(chunk),
  onCreated: (resource) => registerResource(resource),
  disposeLateResource: (resource) => disposeResource(resource),
  onTermination: (termination: StreamTermination) => {
    if (termination.kind === 'error' || termination.kind === 'creation_error') {
      reportError(termination.error);
    }
  },
});

void channel.start();
// 用户主动离开时：channel.stop('view disposed')
// 仅 manual 模式需要：channel.complete()
```

`useChannel`、`runChannelStream` 都复用 `channel-lifecycle.ts`。业务只关心数据、资源所有权和一个终止结果；不得在业务组件内重新实现 `Channel`、`invoke` 的竞态处理。

### 5.4 useEvent — Tauri Event Projection Hook

封装 Tauri `app.listen()`，提供 Solid.js 响应式事件订阅。它只消费后端发布到 Tauri 的只读事件，不是前端 EventBus：

```typescript
// src/lib/stream/useEvent.ts
import { onCleanup } from 'solid-js';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/**
 * 订阅 Tauri 只读事件
 * @param eventName - 事件名称
 * @param handler - 事件处理函数
 *
 * @example
 * useEvent('terminal.command.completed', (e) => {
 *   console.log('命令完成:', e.payload);
 * });
 */
function useEvent<T = unknown>(
  eventName: string,
  handler: (event: { payload: T }) => void,
): void {
  let unlisten: UnlistenFn | null = null;

  // Tauri listen 返回 Promise<UnlistenFn>
  listen<T>(eventName, handler).then(fn => {
    unlisten = fn;
  });

  onCleanup(() => {
    unlisten?.();
  });
}

export { useEvent };
```

### 5.5 统一流数据类型

```typescript
// src/lib/stream/types.ts

export interface ChatAgentTimelinePart {
  partId: string;
  turnId: string;
  messageId: string;
  sequence: number;
  kind: 'reasoning' | 'thinking' | 'tool' | 'permission' | 'text' | 'error' | 'summary' | string;
  status?: 'pending' | 'running' | 'waiting_permission' | 'completed' | 'error' | 'denied' | string | null;
  callId?: string | null;
  tool?: string | null;
  gatewayTool?: string | null;
  title?: string | null;
  summary?: string | null;
  detail?: string | null;
  text?: string | null;
  source?: string | null;
  input?: Record<string, unknown> | null;
  output?: Record<string, unknown> | null;
  metadata?: Record<string, unknown> | null;
  progress?: Record<string, unknown> | null;
  createdAt: string;
  updatedAt?: string | null;
  startedAt?: string | null;
  completedAt?: string | null;
  durationMs?: number | null;
}

export type SessionMessageStreamChunk =
  | { type: 'agentTimelinePart'; part: ChatAgentTimelinePart }
  | {
      type: 'agentTimelinePartDelta';
      messageId: string;
      turnId: string;
      partId: string;
      field: 'text' | 'detail' | 'summary';
      delta: string;
    }
  | { type: 'toolApproval'; request: ToolApprovalRequest }
  | { type: 'messages'; messages: ChatMessageStreamItem[]; total: number };

/** Extension 自定义流数据（泛型，由扩展定义 Schema） */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type ExtensionStreamChunk = Record<string, any>;
```

### 5.6 使用示例

#### Terminal Panel（callback 模式 — 直接写 xterm.js）

```typescript
import { onMount } from 'solid-js';
import { useChannel } from '@/lib/stream';
import type { StreamTermination } from '@/lib/stream';

function TerminalPanel() {
  let term!: Terminal;
  let ptyId: string | null = null;

  const { start, stop, isActive } = useChannel<string, { ptyId: string; sessionId: string }>({
    command: 'ui_terminal_create_pty',
    args: { options: { shell: 'powershell' } },
    mode: 'callback',
    onChunk: (chunk) => {
      term.write(chunk);  // 直接写入 xterm.js，不经过 Signal
    },
    onCreated: (result) => {
      ptyId = result.ptyId;    // 获取 ptyId，供 writePty/resizePty 使用
    },
    disposeLateResource: (result) => {
      void invoke('ui_terminal_close_pty', { payload: result });
    },
    onTermination: (termination: StreamTermination) => {
      ptyId = null;
      if (termination.kind === 'error' || termination.kind === 'creation_error') {
        term.writeln(`\r\n\x1b[31m${termination.error.message}\x1b[0m`);
      }
    },
  });

  onMount(() => start());

  return <div ref={(el) => { term = new Terminal(); term.open(el); }} />;
}
```

#### Agent Chat（callback 模式 — 消费 Turn Timeline）

```typescript
import { runChannelStream } from '@/lib/stream';
import type { SessionMessageStreamChunk } from '@/lib/stream';

function sendChatMessage(sessionId: string, content: string) {
  return runChannelStream<SessionMessageStreamChunk>({
    command: 'ui_stream_session_message',
    args: { payload: { sessionId, content } },
    onChunk: (chunk) => {
      if (chunk.type === 'agentTimelinePart') {
        upsertAgentTimelinePart(chunk.part); // 按 messageId + part.partId 更新 Turn Timeline；不得用 callId 兜底合并不同 partId
      }
      if (chunk.type === 'agentTimelinePartDelta') {
        appendAgentTimelinePartField(chunk); // 只追加到已存在 AgentTimelinePart；缺失视为后端协议错误
      }
      if (chunk.type === 'toolApproval') {
        showInlineApproval(chunk.request);
      }
      if (chunk.type === 'messages') {
        replaceSessionMessages(chunk.messages);
      }
    },
  });
}
```

#### Extension 面板（callback 模式 — 自定义数据消费）

```typescript
import { useChannel } from '@/lib/stream';
import type { ExtensionStreamChunk } from '@/lib/stream';

function ExtensionDataPanel({ extensionId }) {
  const [chartData, setChartData] = createSignal<number[]>([]);

  useChannel<ExtensionStreamChunk>({
    command: 'extension.streamSubscribe',
    args: { extensionId },
    mode: 'callback',
    onChunk: (chunk) => {
      setChartData(prev => [...prev.slice(-99), chunk.value]);  // 保留最近 100 个点
    },
  });

  return <Chart data={chartData()} />;
}
```

### 5.7 与 Kernel EventBus 只读事件出口的配合

流式数据通过 Stream 模块推送；流的生命周期事件由后端发布到 Kernel EventBus，再由 UI runtime 发布为前端可监听的只读 Tauri event：

```typescript
import { useEvent } from '@/lib/stream';

// 监听终端生命周期事件
useEvent('terminal.command.completed', (e) => {
  toast.success(`命令完成: ${e.payload.command} (${e.payload.duration}ms)`);
});

useEvent('agent.task.failed', (e) => {
  toast.error(`任务失败: ${e.payload.error}`);
});
```

---

## 六、架构总览事件定义修正

以下事件从 EventBus 事件表中移出，标记为 Stream 模块通道：

### 6.1 移出 EventBus 的事件

```typescript
// ═══════════════════════════════════════════════════════════
// 以下事件不再通过 EventBus 发送，改走 Stream 模块
// ═══════════════════════════════════════════════════════════

// Agent 事件 - 移出
// 'agent.message.stream'    → 改走 Stream 模块（Command: ui_stream_session_message）
// 'agent.thinking.chunk'    → 改走 Stream 模块（Command: ui_stream_session_message）

// Gateway 事件 - 移出
// 'gateway.stream.chunk'    → 后端内部用 tokio mpsc，不走 EventBus
// 'gateway.stream.done'     → 同上
// 'gateway.stream.error'    → 同上

// Terminal 事件 - 移出
// 'terminal.output'         → 改走 Stream 模块（Command: terminal.createPty）
```

### 6.2 保留在 EventBus 的事件

```typescript
// Agent 事件 - 保留（离散状态）
type AgentStateEvents = {
  'agent.state.changed':     { sessionId, previous, current }
  'agent.task.started':      { sessionId, taskId }
  'agent.task.progress':     { sessionId, taskId, progress, message }
  'agent.task.completed':    { sessionId, taskId, duration }
  'agent.task.failed':       { sessionId, taskId, error }
  'agent.task.cancelled':    { sessionId, taskId }
  'agent.message.complete':  { sessionId, messageId, content }
}

// Gateway 事件 - 保留（离散状态）
type GatewayStateEvents = {
  'gateway.request.started':   { sessionId, requestId, model }
  'gateway.request.completed': { sessionId, requestId, duration, usage }
  'gateway.request.failed':    { sessionId, requestId, error }
  'gateway.request.retry':     { sessionId, requestId, attempt }
  'gateway.image.processed':   { sessionId, requestId, ... }
  'gateway.image.rejected':    { sessionId, requestId, ... }
  'gateway.model.switched':    { from, to, reason }
  'gateway.quota.warning':     { model, usage, limit }
  'gateway.quota.exceeded':    { model }
  'gateway.cost.updated':      { model, cost, ... }
  'gateway.provider.connected':    { providerId, providerType }
  'gateway.provider.disconnected': { providerId, providerType, reason }
  'gateway.provider.reconnecting': { providerId, providerType, attempt }
  'gateway.offline':           { reason, fallbackModel, timestamp }
  'gateway.online':            { fallbackModel, restoredModel }
}

// Terminal 事件 - 保留（离散状态）
type TerminalStateEvents = {
  'terminal.created':            { sessionId, terminalId, shell }
  'terminal.closed':             { sessionId, terminalId }
  'terminal.exit':               { sessionId, terminalId, code }
  'terminal.error':              { sessionId, terminalId, error }
  'terminal.command.started':    { sessionId, terminalId, command }
  'terminal.command.completed':  { sessionId, terminalId, command, exitCode, duration }
  'terminal.command.failed':     { sessionId, terminalId, command, error }
  'terminal.cwd.changed':       { sessionId, terminalId, newPath }
}
```

---

## 七、性能指标

| 指标 | 要求 | 说明 |
|------|------|------|
| StreamChannel 创建 | < 0.5ms | `StreamChannel::builder().build()`，含 StreamIndex 活动跟踪 |
| StreamSource 创建 | < 0.01ms | 开放结构体，零开销 |
| Chunk 发送延迟（Tauri Channel） | < 2ms | `channel.send()` → 前端 `onmessage` |
| Chunk 发送延迟（tokio mpsc） | < 0.1ms | `sender.send()` → `receiver.recv()` |
| 节流窗口 | 50ms | ThrottledEmitter 默认间隔 |
| 序列号保证 | 原生有序 | Tauri Channel 内建有序传输 |
| 背压行为 | 原生内建 | 慢消费者自动阻塞生产者，不丢数据 |
| IPC 效率 | 二进制直传 | 优于 JSON + emit 广播模式 |
| StreamIndex 查询 | < 1ms | 100 条以内 |
| mpsc 通道容量 | 64 | 默认 buffer 大小，可按需调整 |

---

## 八、测试策略

```
单元测试：
├── StreamSource 开放结构体（kind/id 赋值、序列化/反序列化）
├── stream_kind 常量正确性
├── StreamChannel Builder 模式（必填/选填参数、source 缺失 panic）
├── ThrottledEmitter 缓冲与刷新行为
├── ThrottledEmitter 合并函数正确性
├── StreamSender/StreamReceiver tokio mpsc 配对通信
├── StreamIndex 跟踪/移除/查询
├── StreamIndex 按 kind/source_id/stream_id 查询
└── 数据有序传输验证

集成测试：
├── 场景 1：Gateway → Agent 后端进程内流（mpsc_pair 全链路）
├── 场景 2：Terminal → 前端跨进程推送（StreamChannel + Tauri Channel）
├── 场景 3：Extension 自定义流（StreamChannel + Extension runtime API）
├── Terminal StreamChannel → ThrottledEmitter → Channel → 前端 onmessage 完整链路
├── Agent StreamChannel → Channel → 前端 onmessage 完整链路
├── 节流效果验证（高频 push 合并为低频 send）
├── 终端关闭时自动 flush 残留数据（Drop 前显式 flush）
├── 背压验证（前端处理慢时后端不丢数据）
└── StreamIndex 生命周期（Channel 创建自动加入活动索引，Drop 自动移除）
```

