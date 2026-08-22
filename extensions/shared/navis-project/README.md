# navis-project

项目与工作树扩展（骨架占位）：项目目录、工作树、文件系统访问。

## 贡献点

| 类型 | 键 | 说明 |
| --- | --- | --- |
| slot | `navis-code.sidebar.left` → `ProjectTree` | 项目目录树 |
| providesSlots | `navis-project.overview` | 项目概览子插槽 |
| tool | `project.open` / `project.worktree` | 打开项目/查询工作树 |

## 下一步

- 提供后端进程暴露 `project.*` 工具
- 绑定 `ProjectTree` 组件