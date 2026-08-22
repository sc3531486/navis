# navis-editor

编辑器扩展（骨架占位）：代码编辑、文件操作、Git、剪贴板。

## 贡献点

| 类型 | 键 | 说明 |
| --- | --- | --- |
| slot | `navis-code.viewport.main` → `Editor` | 编辑器主视图 |
| providesSlots | `navis-editor.tabs` / `navis-editor.diff` | 标签页 / Diff 子插槽 |
| tool | `file.read` / `file.write` / `git.status` / `git.commit` | 文件与 Git 能力 |

## 下一步

- 提供 stdio JSON-RPC 后端进程暴露 `file.*`/`git.*` 工具
- 绑定 `Editor` 组件（可选用 CodeMirror/Monaco）