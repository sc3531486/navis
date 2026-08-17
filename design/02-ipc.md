# 02 - IPC 通信层详细设计

> 模块编号：02 | 大域：foundation/ipc
> 依赖：Kernel EventBus, Stream
> 被依赖：UI, Session, Agent, Extension, MCP

---

## 一、模块概述

IPC 模块只负责前端和 Rust 后端之间的命令边界：命令解码、参数校验、状态注入、错误编码和响应返回。

Navis Go 不在 IPC 层实现第二套事件总线。离散事件统一通过 `crate::kernel::EventBus` 发布；UI runtime 在启动时订阅唯一 Kernel EventBus，并按 topic 发布为 Tauri event，供前端 `listen()` / `useEvent()` 消费。高频数据统一通过 `foundation::stream` 的 Tauri Channel 推送；持久事实统一写入 Storage。IPC 只把这些能力暴露给前端，不拥有事实源，也不保存事件状态。

## 二、职责边界

负责：

- Tauri command 注册和参数反序列化。
- IPC 错误结构化返回。
- 调用后端业务服务并返回结果。
- 将 Kernel EventBus 的离散事件发布为前端只读 Tauri event。

不负责：

- 不定义独立应用事件总线；事件通知只接入 Kernel EventBus。
- 不保存会话、任务、AgentTimelinePart 或扩展状态。
- 不用事件传递工具结果或模型流式文本。
- 不绕过 Kernel Policy 执行高风险能力。

## 三、通信规则

| 场景 | 通道 | 事实源 |
|------|------|--------|
| 前端执行命令 | Tauri command | 对应业务模块 |
| 离散状态通知 | Kernel EventBus -> UI Tauri event publisher | Storage / Registry / Config |
| 流式文本和终端输出 | Stream Channel | Session / AgentTimelinePart |
| 可恢复执行过程 | SessionEvent / AgentTimelinePart | SQLite |

命令成功并不代表事件一定投递成功。业务模块在事实写入成功后发布内核事件；事件发布失败只记录 `tracing::warn!`，不回滚事实源。Tauri event 只是 UI 侧只读事件出口，不命名为 bridge，也不能作为后端业务事件入口。

## 四、接口形态

Rust 后端：

```rust
#[tauri::command]
async fn ui_stream_session_message(...) -> Result<StreamHandle, UiError> {
    // 1. 校验 session / project / policy
    // 2. 创建 stream channel
    // 3. 启动 Agent run
    // 4. 返回 stream id
}
```

前端：

```ts
const result = await invoke<T>("ui_command_name", payload)
```

所有字段使用完整含义命名：`sessionId`、`turnId`、`messageId`、`partId`、`providerId`、`modelId`。不新增历史缩写和兼容字段。

## 五、测试策略

- IPC command 单元测试覆盖参数缺失、业务错误、成功响应。
- EventBus 订阅相关测试必须运行在 Tokio runtime 内。
- Stream 测试覆盖取消、背压、结束事件和错误事件。
