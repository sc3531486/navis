# 26 - Editor 编辑器渲染详细设计

> 模块编号：26 | 层级：UI/工具能力层
> 依赖：CodeMirror 6、LSP capability port、Extension lifecycle、02-IPC
> 被依赖：Editor UI、Project/Worktree、HostView

---

## 一、定位

Editor 负责 CodeMirror 文档编辑、标签页、Diff、诊断、语言支持、主题和编辑器交互。Editor 是 UI/runtime 宿主，不是 Extension 生命周期管理器，也不是 Kernel Registry 的业务实现。

Editor 的职责：

- 文档、Selection、Cursor、Undo/Redo、Diff 和多文件标签页。
- CodeMirror language、theme、editor extension 的受控激活。
- LSP 诊断、补全、跳转等能力的 UI 投影。
- 图片/文件输入的展示和与 Agent 消息的 typed DTO 连接。

Editor 不负责：

- Extension 安装、启用、禁用、权限和回滚。
- LSP 进程、server 生命周期和 transport 实现。
- Gateway Provider/Model/协议。
- 任意扩展代码的直接执行或 DOM 访问。

---

## 二、运行时结构

```text
Extension manifest
    -> Extension lifecycle
    -> LspCapabilityPort / UI projection
    -> validated language/theme/editor contribution
    -> Editor activation cache
    -> CodeMirror runtime
```

Editor 只接收已校验的 projection 或 runtime-safe module contract。Editor runtime cache 保存 CodeMirror extension 实例和 activation 状态，不承担 Extension discovery、生命周期、权限裁决或跨域 Registry。

LSP 由 tool/lsp 负责进程、协议和语言服务注册；Editor 只订阅 LSP projection 和事件。新增语言通过 Extension language contribution 和 LspCapabilityPort 接入，不修改 Editor 主流程。

---

## 三、Editor extension contribution

Extension 可以声明：

- contributes.themes：id、name、type、受控主题资源。
- contributes.editorLanguages：id、name、extensions、受控语言模块。
- contributes.editorExtensions：id、name、activationEvents、受控 CodeMirror extension。

声明使用严格 schema，未知字段拒绝。module/entry 路径必须位于 Extension 安装目录，并经过 loader 白名单、资源完整性和沙箱策略校验。manifest 字符串不能绕过 renderer 或 module 白名单。

激活流程：安装 -> loader 解析 manifest -> schema/security 校验 -> Extension lifecycle 注册 projection -> Editor 根据文件类型或 activation event 加载 runtime-safe contribution -> 缓存实例。

停用流程：停止新激活 -> 销毁当前 session 的 extension instance -> 清理 cache -> 注销 projection。失败时不得留下半激活实例。

---

## 四、LSP 边界

LSP 语言声明属于 Extension contribution；LSP server 配置、进程权限、工作目录、资源限制和生命周期属于 LSP host。Extension lifecycle 只依赖 LspCapabilityPort。

Editor 只消费：language id、file extensions、diagnostics、completion、hover、definition、references 和 server status projection。Editor 不持有 LSP manager 具体类型，也不直接启动外部进程。

---

## 五、文档、Diff 与多模态

文档编辑状态属于 Editor runtime；Project/Worktree 是文件事实源，Session 是消息和可恢复执行事实源。Editor 不把临时 UI state 写入 Session transcript。

Diff view 只接收 typed original/modified document、hunks 和 selection 状态；写回必须经过 edit tool 的 validator、write guard 和审计链路。

图片和文件输入流程：选择/拖拽/粘贴 -> 前端预览与大小提示 -> typed IPC DTO -> Gateway/Agent request boundary 处理。前端不负责 Provider 具体图片协议；Adapter 根据 Model capability 负责目标协议转换。

文件必须限制类型、数量、大小和总请求体积；二进制或超限数据 fail-closed。错误投影不包含 secret 和完整敏感内容。

---

## 六、事件与 projection

Editor 事件使用 Kernel EventBus 作为后端离散事实出口，Tauri event/useEvent 只用于前端同步。事件包括 document opened/changed/saved、language activated、theme activated、editor extension activated/deactivated、LSP diagnostic updated 和 diff changed。

UI store 只保存当前窗口的 projection；Editor runtime 不成为后端事实源。事件失败必须 tracing::warn 并保留可诊断信息。

---

## 七、扩展性合同

新增主题、语言或 CodeMirror extension 不修改 Editor 组件分支，只新增 Extension contribution 和受控资源。新增 LSP server 不修改 Editor，只通过 LspCapabilityPort 和 LSP projection 接入。新增模型协议也不进入 Editor，由 Gateway Adapter/Provider/Model 负责。

如果未来需要新的 renderer 或 placement，必须在 UI HostView 域增加版本化 contract、registry 和安全评审；不能在 Editor 中增加通用 Extension runtime 执行入口，也不能让 manifest 字符串直接操作 DOM。

---

## 八、测试与验收

必须覆盖 CodeMirror activation cache、unknown field、路径穿越、module 白名单、Extension enable/disable rollback、language matching、theme switching、LSP projection、Diff write guard、图片/文件大小限制、移动/桌面布局和事件同步。

完成标准：新增 Editor contribution、LSP language 或主题只增加 Extension manifest/受控 Adapter；Editor 主流程、Kernel、Gateway 和其他 UI 模块不增加固定分支。
