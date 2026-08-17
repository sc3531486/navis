# 11 - Terminal 终端管理 详细设计

> 模块编号：11 | 层级：基础能力层
> 依赖：01-Logger, 02-Event+IPC, Kernel Policy, 06-Sandbox
> 被依赖：13-MCP, 16-Agent, 21-Git

---

## 一、模块概述

### 1.1 定位

Terminal 模块提供两方面能力：

1. **命令执行引擎** — 为 Agent 和 MCP 工具提供 Shell 命令执行，运行中通过 AgentTimelinePart progress 推送可见进度，结束后把结构化结果返回给模型
2. **交互式终端面板** — 提供 PTY 伪终端进程，通过 Tauri Channel 流式推送到前端 xterm.js 面板（对标 Claude Desktop 底部终端）

### 1.2 设计说明：双通道输出

Navis Go 采用 Claude Desktop 的双通道输出模式：

```
通道一：对话气泡代码块（Agent 驱动）
─────────────────────────────────────────────
Agent 调用 MCP terminal.run_command
      ↓
后端通过 Executor 执行命令，stdout/stderr 读取线程持续生成 progress snapshot
      ↓
AgentTimelinePart 更新同一 tool 行，展示尾部输出、耗时、行数和字节数
      ↓
命令结束后返回 CommandResult 给 Agent，并把完整结果写入 output/detail

适用：Agent 自动执行的命令、构建/测试/脚本运行、需要与对话上下文关联且可审计的输出
```

```
通道二：交互式终端面板（用户驱动）
─────────────────────────────────────────────
用户打开底部终端面板
      ↓
后端创建 PTY 伪终端进程
      ↓
Tauri Channel 实时推送 PTY stdout/stderr
      ↓
前端 xterm.js 渲染终端界面

适用：用户手动交互、需要 Tab 补全、Ctrl+C 中断、长时间运行的命令
```

两种通道共用同一套 Shell 执行基础设施，区别仅在于输出路由。

---

## 二、架构设计

```
terminal/
├── mod.rs              # 模块入口，导出公共 API
├── executor.rs         # 命令执行器（同步/异步，对话气泡通道）
├── shell.rs            # Shell 进程封装（跨平台 Shell 选择与启动）
├── pty.rs              # PTY 伪终端管理（交互式终端面板通道）
├── stream.rs           # PTY 输出桥接（内部 sender → 专属转发线程 → StreamChannel → Tauri Channel）
├── history.rs          # 命令历史记录（持久化与查询）
└── env.rs              # 环境变量管理（项目级 / 全局）
```

---

## 三、数据模型

### 3.1 命令执行模型（通道一）

```rust
/// 命令执行结果 —— 返回给 Agent 的结构化数据
struct CommandResult {
    command: String,                // 执行的命令
    shell: String,                  // 实际 shell：pwsh / powershell / bash / cmd / sh
    working_dir: PathBuf,           // 实际工作目录
    exit_code: i32,                 // 进程退出码
    timed_out: bool,                // 是否超时
    stdout: String,                 // 标准输出（完整文本）
    stderr: String,                 // 标准错误（完整文本）
    total_lines: usize,             // stdout + stderr 行数
    total_bytes: usize,             // stdout + stderr 字节数
    duration: Duration,             // 执行耗时
}

/// 命令退出码语义
struct CommandExitSemantics {
    is_error: bool,
    message: Option<String>,
}

/// 运行中进度快照 —— 写入 AgentTimelinePart.progress，不直接进入模型上下文
struct CommandProgress {
    command: String,
    shell: String,
    working_dir: PathBuf,
    output: String,                 // 最近 5 行输出
    full_output: String,            // 有上限的当前输出快照
    full_output_truncated: bool,
    stdout: String,                 // 有上限的 stdout 快照
    stderr: String,                 // 有上限的 stderr 快照
    elapsed_time_seconds: u64,
    total_lines: usize,
    total_bytes: usize,
    timeout_ms: u64,
}

/// 命令执行选项
struct ExecOptions {
    cwd: Option<PathBuf>,           // 工作目录
    timeout: Option<Duration>,      // 超时时间
    env: Option<HashMap<String, String>>, // 额外环境变量
}
```

### 3.2 PTY 模型（通道二）

```rust
/// PTY 会话资源。PTY 资源只由终端域持有，不进入 Kernel。
struct PtySession {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
}

/// 创建后的 PTY 资源集合。
struct SpawnedPty {
    session: PtySession,
    reader: Box<dyn Read + Send>,
    child: Box<dyn Child + Send + Sync>,
    pid: u32,
}

/// 终端域的 PTY 协调服务。
struct PtyService {
    sessions: Mutex<HashMap<String, Arc<PtySession>>>,
    streams: Mutex<PtyStreamManager>,
}

/// `PtyService` 与 `TerminalManager` 必须满足 `Send + Sync`，才能作为共享 Tauri State。
/// Tauri `Channel` 不存入该 State，而是在 `PtyStreamManager` 创建的转发线程中使用。
```

### 3.3 命令历史模型

```rust
/// 命令历史条目（通道一、通道二共用）
struct CommandHistoryEntry {
    id: u64,                        // 自增 ID
    command: String,                // 命令内容
    working_dir: PathBuf,           // 执行时的工作目录
    exit_code: i32,                 // 退出码
    duration: Duration,             // 执行耗时
    timestamp: DateTime<Utc>,       // 执行时间
    triggered_by: TriggerSource,    // 触发来源
}

enum TriggerSource {
    Agent,                          // Agent 自主执行
    McpTool(String),                // MCP 工具调用（附带工具名）
    PtyInteractive,                 // 交互式终端面板（通道二）
}
```

---

## 四、接口定义

### 4.1 IPC 命令（通道一：Agent 命令执行）

```typescript
// 命令执行 —— Agent 通过 IPC 调用后端执行 Shell 命令，返回完整结果
terminal.exec(command: string, options?: {
    cwd?: string;                   // 工作目录，不传则使用项目根目录
    timeout?: number;               // 超时毫秒数，默认 30000
    env?: Record<string, string>;   // 额外环境变量
}): Promise<CommandResult>

// 同步命令执行 —— 阻塞等待命令完成
terminal.execSync(command: string, options?: {
    cwd?: string;
    timeout?: number;
    env?: Record<string, string>;
}): Promise<CommandResult>
```

### 4.2 IPC 命令（通道二：交互式终端面板）

```typescript
// 创建 PTY。Channel 作为 command 参数传入，返回资源标识。
terminal.createPty(options?: {
    sessionId: string;
    shell?: string;                 // bash / zsh / powershell，默认系统默认 shell
    cwd?: string;                   // 工作目录，默认项目根目录
}, channel: Channel<StandardStreamChunk<string>>): Promise<{ ptyId: string; sessionId: string }>

// 向 PTY 进程写入输入（键盘输入）
terminal.writePty(ptyId: string, data: string): Promise<void>

// 调整 PTY 窗口大小（xterm.js resize 时同步调用）
terminal.resizePty(ptyId: string, cols: number, rows: number): Promise<void>

// 销毁 PTY 进程并关闭面板
terminal.closePty(sessionId: string, ptyId: string): Promise<void>
```

### 4.3 MCP 工具（供 Agent 调用）

```
terminal.exec(command, cwd?, timeout?, env?) → CommandResult
terminal.execSync(command, cwd?, timeout?, env?) → CommandResult
terminal.getHistory(limit?) → CommandHistoryEntry[]
```

### 4.4 Rust API（供其他后端模块调用）

```rust
impl TerminalManager {
    // ── 通道一：命令执行 ──
    pub fn exec_sync(&self, command: &str, options: ExecOptions) -> Result<CommandResult>;
    pub async fn exec(&self, command: &str, options: ExecOptions) -> Result<CommandResult>;

    // ── 通道二：PTY 交互 ──
    pub async fn create_pty(&self, session_id: &str, shell: Option<&str>, cwd: Option<PathBuf>, channel: Channel<StandardStreamChunk<String>>) -> Result<String>;
    pub async fn write_pty(&self, pty_id: &str, data: &str) -> Result<()>;
    pub fn resize_terminal(&self, pty_id: &str, cols: u32, rows: u32) -> Result<()>;
    pub async fn close_pty(&self, session_id: &str, pty_id: &str) -> Result<()>;

    // ── 公共 ──
    pub fn get_history(&self, limit: Option<usize>) -> Result<Vec<CommandHistoryEntry>>;
    pub fn clear_history(&self) -> Result<()>;
}
```

### 4.5 命令语义与退出码归一化

Agent 执行命令前必须把实际执行 shell 写入 Kernel Policy 输入，并由 Sandbox constraint 按该 shell 语义校验：

```rust
evaluate_command_request_for_shell(sandbox, operation_request, actual_shell)
```

终端执行路径不得直接把 Sandbox 当最终裁决入口；必须通过 Kernel Policy constraint 得到 Allow / Ask / Deny。这样 PowerShell/cmd/bash 的只读命令不会互相串用。`Allow` 才能进入进程执行；`Ask` 在当前非交互 Terminal 执行入口中必须 fail-closed 返回 `Command requires approval`，由 UI 审批通道重新授予后再发起执行；`Deny` 直接拒绝。命令执行器结束后会保留原始 stdout/stderr，同时按命令语义解释退出码：

- `findstr` / `grep` / `rg`：exit code `1` 表示无匹配，不视为执行错误；`>=2` 才是错误。
- `robocopy`：exit code `0..=7` 是成功/信息性位标志，归一化为成功；`>=8` 才是失败。
- 其他命令：非零退出码按失败处理。

归一化只影响 `CommandResult.exit_code` 的错误判断，AgentTimelinePart 仍应展示原始输出和耗时，避免用户看不到真实命令行为。

---

## 五、PTY 流式推送（通道二）

### 5.1 数据流

```
PTY reader
      │
      ▼
PtyStreamManager sender
      │
      ▼
专属 forwarder thread
  StreamChannel::builder(channel)
  .source(StreamSource::new("terminal", pty_id))
  .build()  ← 统一 Stream 模块
      │  （通过统一 StreamChannel 的节流/序列化能力）
      ▼
Tauri Channel<StandardStreamChunk<string>>::send()  ← 专用通道，不经过 EventBus
      │
      ▼
前端 channel.onmessage → xterm.js term.write(data)
```

> **节流策略**：高频输出（如 `npm install`）可能每秒数千行，StreamChannel 通过 Builder 的 `.throttle(Duration::from_millis(50))` 参数配置 ThrottledEmitter，在 50ms 窗口内合并所有 chunk，IPC 调用频率控制在 ~20 次/秒，用户感知无延迟。详见 02b-stream.md §四。

### 5.2 ThrottledEmitter 实现

Terminal 的 `stream.rs` 使用统一 Stream 模块的 `StreamChannel`，不自行实现第二套 Channel 生命周期：

```rust
// 仅在专属转发线程中创建并持有 Tauri Channel
let mut stream_channel = StreamChannel::builder(channel)
    .source(StreamSource::new(stream_kind::TERMINAL, &pty_id))
    .label("PTY output")
    .throttle(Duration::from_millis(50))
    .build();

// 推送 PTY 输出（内部自动节流）
while let Ok(output) = receiver.recv() {
    stream_channel.push(&output.data, output.metadata);
}
```

---

## 六、命令历史

### 6.1 存储策略

- 内存中保留最近 1000 条记录（环形缓冲区）
- 当前实现以 `HistoryManager` 内存环形缓冲区为主
- SQLite 持久化与历史查询是后续扩展，不作为当前 PTY State 的隐式副作用

### 6.2 查询能力

```
按时间范围查询：get_history_by_time(start, end)
按命令模糊搜索：search_history(pattern)
按退出码过滤：get_history_by_exit_code(code)
```

---

## 七、环境变量管理

```
优先级（从高到低）：
├── 命令级 env 参数（ExecOptions.env）
├── 项目级环境变量（.env 文件 + 项目配置）
├── 用户级环境变量（全局配置）
└── 系统级环境变量（OS 环境继承）
```

**实现要点：**
- 环境变量在命令执行前按优先级合并，相同 key 高优先级覆盖低优先级
- 敏感变量（API_KEY、TOKEN 等）不出现在命令历史记录中
- Sandbox 可通过规则限制特定环境变量的传递

---

## 八、事件定义

> 以下为通过 **EventBus** 发送的离散状态事件。
> PTY 交互式输出走 **Tauri Channel**，不经过 EventBus。

```typescript
type TerminalEvents = {
  // ── 通道一：命令执行事件 ──
  'terminal.command.started':    { sessionId: string; command: string; requestId: string }
  'terminal.command.completed':  { sessionId: string; command: string; requestId: string; exitCode: number; duration: number }
  'terminal.command.failed':     { sessionId: string; command: string; requestId: string; error: string; exitCode?: number }

  // ── 通道二：PTY 面板生命周期（输出本身不经过 EventBus） ──
  'terminal.created':            { sessionId: string; ptyId: string; shell: string }
  'terminal.closed':             { sessionId: string; ptyId: string }
}
```

---

## 九、项目切换响应

终端管理器监听 `project.switched` 事件，切换所有 PTY 进程的工作目录。

```
EventBus: project.switched { sessionId, newPath }
      │
      ▼
Terminal Manager
      ├── 遍历所有活跃 PTY 进程
      ├── 更新终端实例的项目绑定
      ├── 后续命令使用新工作目录
      └── 已运行中的进程不受影响（进程自身维护 cwd）
```

---

## 十、性能指标

| 指标 | 要求 | 说明 |
|------|------|------|
| 命令启动延迟 | < 10ms | 从 IPC 调用到 Shell 进程 fork |
| PTY 创建延迟 | < 200ms | 含伪终端设备分配与 Shell 启动 |
| PTY 输出延迟 | < 5ms | 从 PTY stdout 到 Channel.send() |
| 同步命令返回 | < 5ms | 命令完成后序列化与 IPC 返回 |
| 命令历史查询 | < 5ms | 1000 条以内 |
| 大输出处理 | < 100ms | stdout/stderr 1MB 以内 |

---

## 十一、测试策略

```
单元测试：
├── 命令安全校验（Kernel Policy + Sandbox constraint）
├── 命令历史记录（增删查、持久化、环形缓冲区）
├── 环境变量合并优先级
├── 跨平台 Shell 选择逻辑
├── CommandResult 序列化/反序列化
└── ThrottledEmitter 节流合并逻辑

集成测试：
├── 通道一：端到端命令执行（exec / execSync）
├── 通道二：PTY 生命周期（创建 → 输入 → 输出 → 关闭）
├── Tauri Channel 流式推送验证
├── PTY resize 同步验证
├── 命令超时处理
├── 环境变量注入验证
└── 事件触发正确性（command.started / completed / failed / pty.created / pty.exited）

边界测试：
├── 空命令处理
├── 特殊字符与注入防护（Kernel Policy + Sandbox constraint）
├── 工作目录不存在时的行为
├── 并发 PTY 实例隔离性
└── PTY 异常断开时的资源清理
```
