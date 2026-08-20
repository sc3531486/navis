# navis-agent-core

核心 Agent 能力扩展（骨架占位）。

## 定位

在 Navis 通用运行时外壳上提供 Agent 对话、时间线与工具运行时能力。

## 贡献点

| 类型 | 键 | 说明 |
| --- | --- | --- |
| slot | `navis-code.viewport.main` → `Composer` | 对话 Composer |
| slot | `navis-code.sidebar.left` → `SessionList` | 会话列表 |
| providesSlots | `navis-agent.timeline` | 时间线子插槽 |
| tool | `agent.run` / `agent.status` | Agent 执行/状态 |
| pipelineHook | `beforeToolExecute` → `toolGuard` | 工具执行护栏 |

## 结构

```text
ExtensionUI/       # 前端（占位，未渲染具体 UI）
ExtensionBackend/  # 后端（占位）
extension.json     # 清单
```

## 下一步

- 绑定 `Composer`/`SessionList` 具名组件并注入子插槽
- 提供 stdio JSON-RPC 后端进程，暴露 `agent.*` 工具