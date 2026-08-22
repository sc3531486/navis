# navis-session

会话管理扩展（骨架占位）：会话列表、历史、快照与工作树绑定。

## 贡献点

| 类型 | 键 | 说明 |
| --- | --- | --- |
| slot | `navis-code.sidebar.left` → `SessionList` | 会话列表 |
| providesSlots | `navis-session.list` | 列表子插槽 |
| tool | `session.create` / `session.load` | 创建/加载会话 |

## 下一步

- 提供后端进程暴露 `session.*` 工具
- 绑定 `SessionList` 组件