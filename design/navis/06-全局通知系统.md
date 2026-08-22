# 25 - Notification 通知系统 详细设计

> 模块编号：25 | 层级：UI 层
> 依赖：22-UI-Framework, 02-IPC, Kernel EventBus
> 被依赖：无（顶层展示组件）

---

## 一、模块概述

### 1.1 定位

Notification 管理应用内所有通知展示，包括 Toast 即时通知、通知中心持久化通知、系统级通知。

### 1.2 职责边界

```
负责：
├── Toast 即时通知（自动消失/手动关闭）
├── 通知中心（持久化、可查看历史）
├── 系统通知（操作系统级通知）
├── 通知级别管理（info/success/warning/error）
└── 通知去重（相同消息不重复显示）

不负责：
├── 通知触发逻辑 → 各模块通过 Kernel EventBus 触发
└── 声音提醒 → 可选扩展
```

---

## 二、架构设计

```
src/components/Notification/
├── Toast.tsx               # Toast 组件
├── ToastContainer.tsx      # Toast 容器（堆叠管理）
├── NotificationCenter.tsx  # 通知中心面板
├── NotificationItem.tsx    # 单条通知
├── store.ts                # 通知状态管理
└── hooks.ts                # 通知相关 Hooks
```

---

## 三、数据模型

```typescript
interface Notification {
  id: string
  level: 'info' | 'success' | 'warning' | 'error'
  title: string
  message?: string
  timestamp: number
  read: boolean
  persistent: boolean      // 是否持久化到通知中心
  duration?: number        // Toast 持续时间
  actions?: NotificationAction[]
  source?: string          // 来源模块
}

interface NotificationAction {
  label: string
  handler: () => void
}
```

---

## 四、接口定义

```typescript
// Toast 通知
notification.info(title: string, message?: string): string
notification.success(title: string, message?: string): string
notification.warning(title: string, message?: string): string
notification.error(title: string, message?: string): string

// 管理
notification.dismiss(id: string): void
notification.clearAll(): void
notification.markRead(id: string): void
notification.markAllRead(): void
notification.getUnread(): Notification[]
notification.getAll(): Notification[]

// 订阅（响应 Kernel EventBus 的 UI Tauri event publisher 自动产生通知）
notification.subscribe(): void
```

---

## 五、自动通知触发规则

```
Kernel EventBus -> UI Tauri event publisher -> 自动产生通知

任务完成        → Toast success（后台任务时）
扩展资源就绪    → Toast success + 通知中心
网络断开        → Toast warning
网络恢复        → Toast success
模型调用失败    → Toast error + 通知中心
存储空间不足    → Toast warning + 通知中心
新版本可用      → 通知中心 info
安全操作被拦截  → Toast warning
MCP 连接失败   → Toast error + 通知中心
RAG 索引完成   → Toast success
```

---

## 六、Toast 样式

```
┌──────────────────────────────────────┐
│  ✅ 操作成功                          │
│  文件已保存到 ./output/result.ts      │
│                               [关闭] │
└──────────────────────────────────────┘

┌──────────────────────────────────────┐
│  ⚠️ 网络断开                          │
│  模型请求将等待网络恢复或切换可用 Provider │
│                               [关闭] │
└──────────────────────────────────────┘

┌──────────────────────────────────────┐
│  ❌ 模型调用失败                       │
│  API Key 无效，请检查配置              │
│                     [去设置] [关闭]   │
└──────────────────────────────────────┘
```

---

## 七、事件定义

```typescript
type NotificationEvents = {
  'notification.show':    { id: string; level: 'info' | 'warning' | 'error' | 'success'; title: string; message?: string; persistent: boolean; source: string; actions?: Array<{ id: string; label: string }> }
  'notification.dismiss': { id: string }
  'notification.clear':   {}
  'notification.unread_count.changed': { count: number }
}
```

---

## 八、测试策略

```
单元测试：Toast 显示/消失、通知中心管理、去重逻辑
集成测试：Kernel EventBus 发布到 UI 的只读 Tauri event 自动触发通知、系统通知调用
```

---

## 扩展扩展支持

通知系统支持通过扩展注册自定义通知渠道，将通知分发到外部服务（Webhook、Slack、邮件等）。

### contributes.notification_channels

```json
{
  "contributes": {
    "notification_channels": [
      {
        "id": "slack",
        "name": "Slack",
        "description": "Send notifications to Slack channel",
        "configSchema": {
          "type": "object",
          "properties": {
            "webhookUrl": { "type": "string", "title": "Webhook URL" },
            "channel": { "type": "string", "title": "Channel", "default": "#general" }
          },
          "required": ["webhookUrl"]
        }
      }
    ]
  }
}
```

> **注意**：`notification_channels` 只承载**声明式配置**（id/name/configSchema）。扩展禁止提供可执行 JS 模块路径（`module` 字段），宿主不加载扩展 JS 运行时（见 07-extension.md、33-extension-gateway-review.md）；渠道的发送实现由宿主 Notification 域按渠道类型（webhook/http）提供，扩展通过配置声明目标端点。

### 渠道接口

扩展需实现 `NotificationChannel` 接口：

```typescript
interface NotificationChannel {
  id: string
  name: string

  // 初始化（扩展启用时调用，传入用户配置）
  initialize(config: Record<string, any>): Promise<void>

  // 发送通知
  send(notification: NotificationPayload): Promise<void>

  // 清理（扩展禁用时调用）
  dispose(): Promise<void>
}

interface NotificationPayload {
  level: 'info' | 'warning' | 'error' | 'success'
  title: string
  message: string
  source: string        // 来源模块名
  timestamp: number
  actions?: Array<{ id: string; label: string }>
}
```

### 分发机制

Kernel EventBus 只负责业务离散事件；Notification 组件通过 UI Tauri event publisher 订阅这些事件并创建 UI 通知。扩展通知渠道发送失败只在前端生成本地 warning Toast，不回写 Kernel EventBus，避免通知分发形成事件循环。

```
任意模块发出通知
     │
     ▼
Notification Dispatcher
     │
     ├── 内置渠道：Toast（即时显示）
     ├── 内置渠道：Notification Center（持久化存储）
     ├── 内置渠道：System Notification（系统原生通知）
     │
     └── 扩展渠道（并行分发）：
         ├── Slack → channel.send(payload)
         ├── Webhook → channel.send(payload)
         ├── 邮件 → channel.send(payload)
         └── ...（任何已注册的渠道）
```

### 渠道配置

- 渠道配置通过 Extension 框架的 `contributes.configuration` 声明
- 用户在设置页面的"通知"分类下配置每个渠道的参数
- 配置存储在 Config 模块中，扩展启用时由宿主配置 API 读取并注入运行上下文

### 渠道过滤

用户可在设置中配置每个渠道的通知级别过滤：
```
Slack 渠道：只接收 warning + error
Webhook 渠道：接收所有级别
邮件渠道：只接收 error
```

### 新增事件

```typescript
type NotificationChannelEvents = {
  'notification.channel.registered':   { channelId: string; channelName: string }
  'notification.channel.unregistered': { channelId: string }
  'notification.channel.sent':         { channelId: string; level: string; title: string }
  'notification.channel.failed':       { channelId: string; error: string }
}
```
