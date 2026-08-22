# navis-settings

设置扩展（骨架占位）：全局与扩展配置管理。

## 贡献点

| 类型 | 键 | 说明 |
| --- | --- | --- |
| slot | `overlay` → `SettingsDialog` | 设置对话框 |
| tool | `settings.get` / `settings.set` | 配置读写 |

## 下一步

- 提供后端进程暴露 `settings.*` 工具
- 绑定 `SettingsDialog` 组件（挂到 overlay 插槽）