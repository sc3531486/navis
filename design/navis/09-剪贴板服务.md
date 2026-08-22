# 32 - Clipboard 剪贴板 详细设计

> 模块编号：32 | 层级：基础能力层
> 依赖：01-Logger, Kernel EventBus, Kernel Policy, 02-IPC
> 被依赖：15-Code-Edit, 26-Editor

---

## 一、模块概述

### 1.1 定位

Clipboard 封装系统剪贴板，提供读写、格式处理、历史记录能力。

### 1.2 职责边界

```
负责：
├── 系统剪贴板读写（纯文本/富文本）
├── 代码块格式清洗（复制时保留缩进）
├── 剪贴板历史记录（最近 N 条）
├── 剪贴板监听（设备观察器，只负责检测外部复制）
├── 通过 Kernel EventBus 发布 `clipboard.changed`
├── 读写前消费 Kernel Policy（`tool.clipboard.read/write`）
└── 跨格式转换（Markdown → 纯文本等）

不负责：
├── 编辑器复制/粘贴操作 → Editor
├── 文件复制 → File
└── 自建权限系统或审批事实源 → 统一走 Kernel Policy / Audit
```

---

## 二、架构设计

```
clipboard/
├── mod.rs              # 模块入口
├── policy.rs           # Clipboard Policy constraint（消费 Kernel Policy）
├── provider.rs         # 系统剪贴板接口
├── formatter.rs        # 格式处理
├── history.rs          # 历史记录
└── watcher.rs          # 剪贴板监听
```

---

## 三、数据模型

```rust
struct ClipboardEntry {
    id: String,
    content: String,
    format: ClipboardFormat,
    source: Option<String>,     // 来源（如文件路径）
    timestamp: DateTime<Utc>,
}

enum ClipboardFormat {
    PlainText,
    RichText,
    Code { language: Option<String> },
    Image,
}
```

---

## 四、接口定义

### 4.1 IPC 命令

```typescript
clipboard.read(): Promise<string>
clipboard.write(content: string, format?: string): Promise<void>
clipboard.readRich(): Promise<{ text: string; html: string }>
clipboard.getHistory(limit?: number): Promise<ClipboardEntry[]>
clipboard.clearHistory(): Promise<void>
```

`clipboard.read()` / `clipboard.write()` 在访问系统剪贴板前必须评估 Kernel Policy：

| 操作 | Policy action | subject | 说明 |
|------|---------------|---------|------|
| 读取 | `tool.clipboard.read` | `user` / `mcp_executor` / `agent` | 默认允许，但必须留下统一决策点 |
| 写入 | `tool.clipboard.write` | `user` / `mcp_executor` / `agent` | 用户主动写入允许；Agent/MCP/扩展写入按当前审批模式 Ask/Allow |

前端监听剪贴板变化时消费 Kernel EventBus 的 Tauri 只读投影，不在 Clipboard 模块内建立 `subscribe/unsubscribe` 分发系统。

### 4.2 MCP 工具

```
clipboard.read() → string
clipboard.write(content) → void
clipboard.history(limit?) → ClipboardEntry[]
```

MCP 内置 `clipboard.get` / `clipboard.set` 不直接绕过安全边界。它们作为 MCP 工具注册进 Kernel-backed Tool Registry，执行时先进入 MCP Executor 的 Kernel Pipeline，再由 Clipboard Policy constraint 评估 `tool.clipboard.read/write`。需要确认时通过标准 `authorization.requested` 事件返回 UI，不能在工具实现内部私自放行。

---

## 五、事件定义

```typescript
type ClipboardEvents = {
  'clipboard.changed': { content: string; format: string; source?: string }
}
```

`clipboard.changed` 由 `ClipboardManager` 在写入历史成功后发布到 Kernel EventBus。`watcher.rs` 只是设备轮询观察器，不保存事实，也不向外暴露第二套事件总线。

---

## 六、性能指标

| 指标 | 要求 | 说明 |
|------|------|------|
| 读取 | < 5ms | 系统调用 |
| 写入 | < 5ms | 系统调用 |
| 历史查询 | < 1ms | 内存读取 |

---

## 七、测试策略

```
单元测试：格式转换、历史记录管理
集成测试：系统剪贴板读写、跨平台兼容
```
