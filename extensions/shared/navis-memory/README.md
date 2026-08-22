# navis-memory

记忆扩展（骨架占位）：项目记忆与工具调用记忆的持久化。

## 贡献点

| 类型 | 键 | 说明 |
| --- | --- | --- |
| slot | `navis-code.sidebar.left` → `MemoryPanel` | 记忆面板 |
| providesSlots | `navis-memory.panel` | 面板子插槽 |
| tool | `memory.get` / `memory.set` | 记忆读写 |
| pipelineHook | `assembleContext` → `injectMemory` | 上下文记忆注入 |

## 下一步

- 提供后端进程暴露 `memory.*` 工具
- 绑定 `MemoryPanel` 组件