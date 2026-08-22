# navis-terminal

终端扩展（骨架占位）：PTY 终端、Shell 执行与命令历史。

## 贡献点

| 类型 | 键 | 说明 |
| --- | --- | --- |
| slot | `navis-code.viewport.main` → `Terminal` | 终端面板 |
| providesSlots | `navis-terminal.panel` | 面板子插槽 |
| tool | `terminal.exec` | 执行 Shell 命令 |

## 下一步

- 提供后端进程暴露 `terminal.*` 工具（PTY 通过 stdio 双向流）
- 绑定 `Terminal` 组件（xterm.js）