# 14 - LSP 语言服务协议 详细设计

> 模块编号：14 | 大域：tool/lsp
> 依赖：foundation/logger, foundation/ipc, kernel/EventBus, security/sandbox, tool/file, project
> 被依赖：26-Editor, 16-Agent

---

## 一、模块概述

### 1.1 定位

LSP 是代码智能服务层，管理多个语言 Server 的生命周期，为 Editor 和 Agent 提供代码补全、诊断、跳转、重构等能力。

### 1.2 职责边界

```
负责：
├── LSP Client（与单个 Server 的 JSON-RPC 通信；读取 Content-Length 帧，按 request id 匹配响应，并处理 publishDiagnostics 通知）
├── LSP Manager（多 Server 生命周期管理）
├── LanguageRegistry 查询 facade（语言→Server 配置映射；底层由 Kernel Registry 承载）
├── 能力探测（Server 能力发现）
├── 诊断聚合（多 Server 诊断合并）
├── 文档同步（打开/修改/保存通知）
├── 项目索引管理
└── LSP 事件发布与 UI 投影 DTO

不负责：
├── 代码编辑 → Code Edit
├── 编辑器渲染 → Editor
├── 文件操作 → File
└── LSP Server 进程（外部依赖）
```

---

## 二、架构设计

```
tool/lsp/
├── mod.rs              # 模块入口
├── manager.rs          # LSP Server 管理器
├── client.rs           # 单个 Server 客户端
├── commands.rs         # Tauri IPC 命令：lsp_completion / lsp_hover / lsp_definition / lsp_diagnostics / lsp_format
├── projection.rs        # Kernel EventBus 事件到 UI DTO 的只读投影
├── registry.rs         # LanguageRegistry 查询 facade；不保存第二套能力目录
├── capabilities.rs     # Server 能力探测
├── diagnostics.rs      # 诊断聚合
├── sync.rs             # 文档同步
└── indexer.rs          # 项目索引管理
```

语言注册表边界：

- `LanguageRegistry` 是 LSP 领域查询 facade，底层能力注册、注销和生命周期由 Kernel `Registry` 承载。
- 内置语言配置是 LSP 启动基础能力，注册或启用失败时 `LanguageRegistry::new()` / `LSPManager::new()` 必须 fail-closed，不允许 warn 后继续形成半初始化状态。
- 扩展语言仍走 `LanguageRegistry::register()`，禁止覆盖内置语言；扩展重复注册同一 language_id 直接返回错误。

---

## 三、数据模型

```rust
struct LSPServerConfig {
    language_id: String,
    language_names: Vec<String>,
    file_extensions: Vec<String>,
    server_command: String,
    server_args: Vec<String>,
    initialization_options: Option<Value>,
    capabilities_required: Vec<String>,
}

struct LSPServerStatus {
    server_id: String,
    language_id: String,
    status: ServerStatus,
    capabilities: LSPCapabilities,
    indexed: bool,
    index_progress: Option<f32>,
}

struct LSPCapabilities {
    completion: bool,
    hover: bool,
    definition: bool,
    references: bool,
    diagnostics: bool,
    formatting: bool,
    rename: bool,
    code_action: bool,
    document_symbol: bool,
    workspace_symbol: bool,
}

struct CompletionItem {
    label: String,
    kind: CompletionKind,
    detail: Option<String>,
    documentation: Option<String>,
    insert_text: String,
    sort_text: Option<String>,
}

struct Diagnostic {
    range: Range,
    severity: DiagnosticSeverity,
    message: String,
    code: Option<String>,
    source: Option<String>,
    related_information: Option<Vec<DiagnosticRelated>>,
}

enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

struct HoverResult {
    contents: String,
    range: Option<Range>,
}

struct Location {
    file_path: String,
    range: Range,
}
```

---

## 四、接口定义

```typescript
// Server 管理
lsp.startServer(language: string, rootPath: string): Promise<string>
lsp.stopServer(serverId: string): Promise<void>
lsp.listServers(): Promise<LSPServerStatus[]>
lsp.restartServer(serverId: string): Promise<void>

// 代码智能（Tauri command 名称使用 snake 命名，payload 使用 camelCase）
lsp_completion(payload: { filePath: string, line: number, character: number }): Promise<CompletionItem[]>
lsp_hover(payload: { filePath: string, line: number, character: number }): Promise<HoverResult | null>
lsp_definition(payload: { filePath: string, line: number, character: number }): Promise<Location[]>
lsp_diagnostics(payload: { filePath: string }): Promise<Diagnostic[]>
lsp_format(payload: { filePath: string }): Promise<string | null>

// 后续扩展能力
lsp.completion(filePath: string, line: number, character: number): Promise<CompletionItem[]>
lsp.hover(filePath: string, line: number, character: number): Promise<HoverResult | null>
lsp.definition(filePath: string, line: number, character: number): Promise<Location[]>
lsp.references(filePath: string, line: number, character: number): Promise<Location[]>
lsp.diagnostics(filePath: string): Promise<Diagnostic[]>
lsp.format(filePath: string): Promise<TextEdit[]>
lsp.rename(filePath: string, line: number, character: number, newName: string): Promise<WorkspaceEdit>
lsp.codeAction(filePath: string, range: Range, diagnostics: Diagnostic[]): Promise<CodeAction[]>
lsp.documentSymbols(filePath: string): Promise<SymbolInformation[]>
lsp.workspaceSymbols(query: string): Promise<SymbolInformation[]>
lsp.highlight(filePath: string, line: number, character: number): Promise<DocumentHighlight[]>
lsp.signatureHelp(filePath: string, line: number, character: number): Promise<SignatureHelp | null>

// 文档同步（通知 LSP Server 文件状态变更）
lsp.didOpen(filePath: string, languageId: string, version: number, content: string): Promise<void>
lsp.didChange(filePath: string, version: number, changes: TextDocumentContentChangeEvent[]): Promise<void>
lsp.didSave(filePath: string): Promise<void>

// 配置
lsp.listLanguages(): Promise<LSPServerConfig[]>
lsp.addLanguage(config: LSPServerConfig): Promise<void>
lsp.removeLanguage(languageId: string): Promise<void>
```

---

## 五、预置语言

```toml
[[languages]]
language_id = "typescript"
language_names = ["typescript", "typescriptreact"]
file_extensions = [".ts", ".tsx"]
server_command = "typescript-language-server"
server_args = ["--stdio"]

[[languages]]
language_id = "python"
language_names = ["python"]
file_extensions = [".py"]
server_command = "pyright-langserver"
server_args = ["--stdio"]

[[languages]]
language_id = "rust"
language_names = ["rust"]
file_extensions = [".rs"]
server_command = "rust-analyzer"

[[languages]]
language_id = "go"
language_names = ["go"]
file_extensions = [".go"]
server_command = "gopls"

[[languages]]
language_id = "java"
language_names = ["java"]
file_extensions = [".java"]
server_command = "jdtls"
```

---

## 六、事件定义

```typescript
type LSPEvents = {
  'lsp.server.started':        { serverId: string; language: string }
  'lsp.server.stopped':        { serverId: string; language: string }
  'lsp.server.error':          { serverId: string; error: string }
  'lsp.server.capabilities':   { serverId: string; capabilities: LSPCapabilities }
  'lsp.server.restarting':     { serverId: string; language: string; attempt: number; maxAttempts: number }
  'lsp.server.crashed':        { serverId: string; language: string; error: string }
  'lsp.diagnostics.published': { filePath: string; diagnostics: Diagnostic[] }
  'lsp.diagnostics.cleared':   { filePath: string }
  'lsp.indexing.started':      { serverId: string }
  'lsp.indexing.progress':     { serverId: string; progress: number; total: number }  // progress: 已索引数量; total: 总文件数量
  'lsp.indexing.completed':    { serverId: string; duration: number }
}
```

上述事件只发布到唯一的 `crate::kernel::EventBus`。前端通过 UI Tauri event publisher 订阅这些离散状态；LSP 模块不得实现独立 EventBus，也不得把 Tauri event 当作后端事实源。

补充约束：

- Editor 前端不得返回空数组或 `null` 作为成功路径的兼容占位；补全、悬停、定义、诊断和格式化必须调用真实 Tauri IPC，失败时只作为错误降级返回空结果。
- Agent 通过内建 MCP 工具 `lsp.query` 暴露同一套后端能力。Tool Projection 对模型侧固定投影为 `lsp`，运行时注入 `worktreeRoot`，模型只提供 worktree-relative `file_path`、`operation` 和位置参数。
- `format` 返回格式化后的完整文本供调用方应用，不在 LSP 模块内直接写文件。

---

## 七、项目切换响应

LSP 模块自行监听 `project.switched` 事件，实现 LSP Server 的生命周期管理：

1. **停止旧 Server**：遍历旧项目关联的所有 LSP Server，逐个执行 `stopServer`
2. **启动新 Server**：根据新项目的文件类型扫描，自动启动对应语言的 LSP Server
3. **诊断清理**：清除旧项目的诊断缓存，避免跨项目诊断混淆

```
project.switched 事件到达（LSP 自行监听，非 Project 主动调用）
         │
         ▼
遍历旧项目关联的 Server 列表
         │
         ▼
逐个 stopServer（等待 graceful shutdown）
         │
         ▼
扫描新项目文件类型
         │
         ▼
自动启动匹配语言的 LSP Server
```

---

## 八、Server 崩溃自动重启策略

当 LSP Server 进程意外退出时，Manager 自动执行重启：

| 重试次数 | 间隔 | 说明 |
|----------|------|------|
| 第 1 次 | 2s | 立即重试，短暂等待 |
| 第 2 次 | 4s | 指数退避 |
| 第 3 次 | 8s | 指数退避 |

- 3 次重试全部失败后，记录错误日志，发出 `lsp.server.error` 事件，不再自动重试
- 用户可手动通过 `lsp.restartServer(serverId)` 再次尝试
- 重启期间该语言的代码智能功能暂时不可用，但不影响其他语言的 Server

---

## 九、多 Project / Worktree 下的 LSP 协议支持

一个 Project / Worktree 可能包含多种语言的源文件，因此可启动多个 LSP Server（按语言区分）。本节中的 `workspace_symbol`、`WorkspaceEdit`、`workspaceSymbols` 属于 LSP 协议命名，不表示 Navis Go 业务域：

- LSP Manager 为每个 Project / Worktree 维护一个 `Map<languageId, serverId>`
- 启动时根据 Worktree 内文件的扩展名匹配 LanguageRegistry facade，自动启动所需语言的 Server
- 诊断聚合模块合并同一 Project / Worktree 下所有 Server 的诊断结果
- 文件操作（如 `didOpen`）自动路由到对应语言的 Server

---

## 十、测试策略

```
单元测试：LanguageRegistry facade、诊断聚合、能力探测
集成测试：Server 生命周期、代码补全、跳转定义、重命名
```

---

## 扩展扩展支持

LSP 模块支持通过扩展注册自定义语言的 LSP Server 配置，无需修改核心代码。

### contributes.languages

扩展在 `extension.json` 中声明语言支持：

```json
{
  "contributes": {
    "languages": [
      {
        "languageId": "csharp",
        "displayName": "C#",
        "extensions": [".cs", ".csx", ".razor"],
        "serverCommand": "omnisharp",
        "serverArgs": ["-lsp"],
        "initializationOptions": {
          "enableEditorConfigSupport": true
        }
      }
    ]
  }
}
```

### 注册流程

```
扩展安装
     │
     ▼
ExtensionLifecycle 启用扩展
     │
     ▼
读取 manifest contributes.languages
     │
     ▼
遍历每个 LanguageRegistration
     │
     ├── 转换为 LSPServerConfig
     ├── 调用 LSPManager.registry().register(config, LanguageSource::Extension)
     └── LSP Registry 经 Kernel Registry 记录能力并发出 lsp.language.registered 事件
```

### 卸载流程

```
扩展禁用/卸载
     │
     ▼
遍历该扩展注册的所有语言
     │
     ├── 调用 LSPManager.registry().unregister(languageId)
     └── LSP Registry 经 Kernel Registry 注销能力并发出 lsp.language.unregistered 事件
```

Extension 模块只负责生命周期编排，不保存独立语言运行时注册表；LSP `LanguageRegistry` 是领域查询 facade，底层由 Kernel Registry 承载。扩展声明的 `languageId` 不能覆盖内置语言，也不能覆盖其他已启用扩展注册的语言。没有注入 LSP 宿主时，带 `contributes.languages` 的扩展启用必须 fail-closed。

### 新增事件

```typescript
type LSPLanguageEvents = {
  'lsp.language.registered':   { languageId: string; displayName: string; source: 'builtin' | 'extension' }
  'lsp.language.unregistered': { languageId: string; source: 'builtin' | 'extension' }
}
```

### 优先级规则

- 内置语言（TS/Python/Rust/Go/Java）优先级最高，扩展不能覆盖
- 多个扩展注册同一 languageId 时，先安装的优先
- 用户可在设置页面查看所有已注册的语言及其来源（内置/扩展）
