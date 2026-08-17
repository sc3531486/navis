# 27 - Hotkey 全局快捷键 详细设计

> 模块编号：27 | 层级：UI 层
> 依赖：01-Logger, 02-Event+IPC, 03-Config
> 被依赖：前端 UI 层

---

## 一、模块概述

### 1.1 定位

Hotkey 管理系统级全局快捷键，提供注册、冲突检测、配置、触发分发能力。

### 1.2 职责边界

```
负责：
├── 全局快捷键注册（系统级，应用不在前台也能触发）
├── 应用内快捷键注册（仅应用内生效）
├── 快捷键冲突检测
├── 快捷键配置（用户自定义）
├── 快捷键触发后的本地通知分发
└── 快捷键帮助文档

不负责：
├── 快捷键绑定的具体操作 → 各模块/前端
└── 快捷键 UI 展示 → 设置页面
```

---

## 二、架构设计

```
hotkey/
├── index.ts            # 模块入口
├── notifier.ts         # Hotkey 模块本地通知器
├── store.ts            # 快捷键存储
├── dispatcher.ts       # 触发分发器
├── conflict.ts         # 冲突检测
└── defaults.ts         # 默认快捷键配置
```

---

## 三、数据模型

```rust
struct HotkeyBinding {
    id: String,
    keybinding: String,          // 如 "Ctrl+Shift+A"
    scope: HotkeyScope,         // global / app
    command: String,            // 触发的命令
    description: String,
    category: String,           // 分类（如 "Agent", "Editor", "Terminal"）
    is_custom: bool,            // 是否用户自定义
}

enum HotkeyScope {
    Global,     // 系统级全局
    App,        // 仅应用内
}
```

---

## 四、默认快捷键

```toml
[[hotkeys]]
keybinding = "Ctrl+Shift+P"
command = "commandPalette.open"
description = "打开命令面板"
scope = "app"
category = "General"

[[hotkeys]]
keybinding = "Ctrl+Shift+N"
command = "session.create"
description = "新建会话"
scope = "app"
category = "Session"

[[hotkeys]]
keybinding = "Ctrl+Shift+A"
command = "agent.abort"
description = "终止当前任务"
scope = "app"
category = "Agent"

[[hotkeys]]
keybinding = "Ctrl+`"
command = "terminal.toggle"
description = "切换终端"
scope = "app"
category = "Terminal"

[[hotkeys]]
keybinding = "Ctrl+B"
command = "sidebar.toggle"
description = "切换侧边栏"
scope = "app"
category = "General"

[[hotkeys]]
keybinding = "Ctrl+Shift+Enter"
command = "agent.sendAndExecute"
description = "发送消息并执行"
scope = "app"
category = "Agent"

[[hotkeys]]
keybinding = "Ctrl+O"
command = "file.open"
description = "打开文件"
scope = "app"
category = "General"

**平台差异说明：**
- macOS 上 `Ctrl` 对应 `Cmd`（Meta key）
- `Ctrl+O`（打开文件）在 macOS 上为 `Cmd+O`，可能与系统级"Open File"快捷键冲突
- 跨平台快捷键如遇系统冲突，优先让位给系统快捷键，Navis Go 使用备选绑定
- 具体平台适配由 `hotkey/platform.ts` 处理

[[hotkeys]]
keybinding = "Ctrl+J"
command = "panel.toggle"
description = "切换面板显示/隐藏"
scope = "app"
category = "General"
```

---

## 五、接口定义

```typescript
hotkey.list(): Promise<HotkeyBinding[]>
hotkey.register(binding: HotkeyBinding): Promise<void>
hotkey.unregister(id: string): Promise<void>
hotkey.update(id: string, keybinding: string): Promise<void>
hotkey.checkConflict(keybinding: string): Promise<HotkeyBinding | null>
hotkey.reset(): Promise<void>  // 恢复默认
```

当前实现为前端 TypeScript `HotkeyManager`，首次调用 `getHotkeyManager()` 时加载 `DEFAULT_HOTKEYS` 并启动浏览器 `keydown` dispatcher。`App` 作用域快捷键在前端处理；`Global` 作用域仍保留给后续 Tauri 系统级热键，不在当前默认 UI 中启用。

当前已接入真实 handler 的默认命令：

- `commandPalette.open`：通过顶部栏挂载的 `useHotkeyCommand` 打开全局 Command Palette。
- `sidebar.toggle`：通过顶部栏挂载的 `useHotkeyCommand` 切换左侧栏。

其他默认快捷键只有注册定义，必须在对应业务能力形成真实闭环后再绑定 handler，不能绑定假动作。

---

## 六、本地通知定义

Hotkey 的 `HotkeyNotifier` 只服务前端 Hotkey 模块内部的 UI/hooks 状态同步，不是应用级 EventBus，也不替代后端 Kernel EventBus。业务事实、后端状态变化和跨域通知仍统一通过 Kernel EventBus 的 UI 投影进入前端。

```typescript
type HotkeyNotificationPayloads = {
  'hotkey.triggered':    { id: string; keybinding: string; command: string }
  'hotkey.registered':   { id: string; keybinding: string }
  'hotkey.unregistered': { id: string }
  'hotkey.conflict':     { id: string; keybinding: string; conflictWith: string }
}
```

---

## 七、Extension 快捷键注册

扩展通过 `extension.json` 中的 `contributes.keybindings` 声明快捷键。当前真实机制：**后端投影公共模型**——`ui_list_extension_keybindings`（见 07-extension.md L1224-1235、22-ui-framework.md L273）在后端投影已启用的扩展 keybinding，前端 Hotkey 模块消费投影注册；**不存在** `hotkey.register()` 后端直接注册链路，本节旧描述已废弃。

### 7.1 注册约束

- 扩展只能注册 `App` 范围的快捷键，不能注册 `Global` 范围
- `Global` 范围需要操作系统级权限申请（如 Windows 全局热键 API），仅核心模块可使用
- 扩展 keybinding 的动作统一为动作族分发（`OpenView/ToggleView/OpenDialog/RunScript/SendMessage`，见 34-extension-ui-open-architecture §4.2.1），不再限于 `OpenView/ToggleView`

### 7.2 冲突处理策略

当扩展注册的快捷键与已有快捷键冲突时：

1. **后注册被拒绝**：不自动覆盖已有绑定
2. **提示用户**：返回冲突信息，告知用户哪个快捷键已被哪个命令占用
3. 用户可手动修改冲突方的快捷键后再重新注册

```
扩展注册 Ctrl+K → 检测冲突 → 无冲突 → 注册成功
扩展注册 Ctrl+B → 检测冲突 → 与 sidebar.toggle 冲突 → 拒绝注册，返回冲突信息
```

---

## 八、测试策略

```
单元测试：冲突检测、配置解析、触发分发
集成测试：系统级快捷键注册、用户自定义覆盖
```
