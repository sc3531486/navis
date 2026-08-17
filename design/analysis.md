# Navis Go 竞品深度分析与对照报告

> 基于 claude-code-main、hermes-agent-main、opencode-dev 三个项目的架构解剖，
> 对照 Navis Go 现有设计，识别优势与差距，指导后续开发方向。

---

## 一、核心技术栈对照

| 维度 | Claude Code | Hermes Agent | OpenCode | Navis Go（当前） |
|------|------------|--------------|----------|--------------|
| 语言 | TypeScript + Bun | Python 3.11+ | TypeScript + Bun/Node | **Rust + TypeScript** |
| UI 框架 | React/Ink (终端) | 无 TUI | SolidJS + Electron | **SolidJS + Tauri 2** |
| 构建系统 | Bun bundler + Vite | setuptools + uv | Turborepo + Bun workspaces | Vite + cargo |
| 运行时 | Bun (兼 Node) | CPython | Bun (兼 Node) | **Tokio + WebView** |
| 数据库 | 文件系统 | SQLite + FTS5 | SQLite (Drizzle ORM) | SQLite (rusqlite) |
| 日志 | console + Sentry | Python logging | Effect tracing | tracing |
| 配置格式 | JSON + Zod | YAML | JSONC | TOML/JSON |

---

## 二、架构分层对照

| 层级 | Claude Code | Hermes Agent | OpenCode | Navis Go |
|------|------------|--------------|----------|------|
| 入口层 | cli.tsx → main.tsx (Commander.js) | cli.py → run_agent.py | index.ts (yargs CLI) | lib.rs → ui/mod.rs (Tauri IPC) |
| UI 层 | React/Ink 终端 TUI | 无（纯 CLI 输出） | SolidJS Web + Electron | SolidJS Web + Tauri WebView |
| 引擎层 | QueryEngine + query.ts | AIAgent + conversation_loop.py | SessionProcessor | Agent 状态机 + AgentEngine |
| 工具层 | Tool 接口 + runTools() | registry.py + model_tools.py | ToolRegistry + tools.ts | MCP 引擎 + ToolExecutor |
| 适配层 | API client (Anthropic SDK) | ProviderProfile + adapters | Provider (AI SDK) | Gateway + ProviderAdapter |
| 状态层 | AppState (React store) | SQLite SessionDB | Effect InstanceState + SQLite | Storage (SQLite) + Kernel EventBus 通知 |
| 扩展层 | Extension + MCP + Skills + Hooks | Extensions + Skills + MCP | Extension + MCP + Skills + Agents | Extension (16 types) + MCP + Skills |

---

## 三、Agent 决策引擎对照

| 设计点 | Claude Code | Hermes Agent | OpenCode | Navis Go |
|--------|------------|--------------|----------|------|
| 循环模式 | AsyncGenerator `queryLoop()` | 同步 while 循环 | Effect Stream `processor.ts` | **状态机驱动** `state_machine.rs` |
| 状态管理 | `State` 对象跨迭代 | `AIAgent` 实例属性 | Effect `InstanceState` | **7 种显式状态**枚举 |
| 状态种类 | 无显式状态机，隐式 continue/terminal | 无状态机，标志位控制 | busy/idle/retry 运行状态 | Idle/Thinking/Streaming/ToolCalling/WaitingPermission/Error/Recovering |
| 工具编排 | 分区并发（partitionToolCalls） | 串行为主，delegate 支持并行 | 逐事件流式处理 | Tool Catalog → Kernel Pipeline 串行主路径；并发只能作为 Pipeline 执行策略 |
| 并发上限 | MAX_CONCURRENCY=10 环境变量 | IterationBudget 90 次总限制 | Session 级互斥 | **Task Scheduler + Worktree 隔离** |
| 终止条件 | max_turns / aborted / no tool_use | budget 耗尽 / 用户中断 | blocked / error / no tool_use | 状态机终态（Idle） |
| 错误恢复 | 3 次恢复 + compact | fallback_model + credential 轮转 | Effect error channel | **Error → Recovering → 原状态** |
| Extended Thinking | 无独立模块 | 无 | reasoning 事件 | **独立 thinking.rs 模块** |
| 自我进化 | 记忆提取 | memory_manager (MEMORY/USER/SOUL.md) | 无 | **self_evolution/**（经验捕获+模式提取+反思） |
| 工作模式 | plan / default / auto | 由 toolset 控制 | build / plan / explore / scout | **Code / Cowork / Custom** |

---

## 四、工具系统对照

| 设计点 | Claude Code | Hermes Agent | OpenCode | Navis Go |
|--------|------------|--------------|----------|------|
| 工具数量 | 60+ 内置 | 50+ 内置 | 15+ 内置 | 17 MCP 内置工具 |
| 工具定义 | `Tool<I,O,P>` 泛型接口（30KB） | `ToolEntry` dataclass | `Tool.Def` + Effect Schema | MCP `ToolDefinition` / `MCPTool` + Tool Catalog 投影 |
| 注册方式 | `getAllBaseTools()` 静态数组 | 自注册 `registry.register()` | `ToolRegistry` 分类注册 | MCP ToolRegistry 的 Kernel-backed facade + Tool Catalog |
| 参数校验 | Zod schema + validateInput | JSON Schema | Effect Schema | JSON Schema (MCP 标准) |
| 权限声明 | isReadOnly / isDestructive / checkPermissions | 无显式声明 | Agent 级 permission 规则集 | Tool Catalog 风险元数据 + Kernel Policy/Sandbox constraint |
| 并发声明 | `isConcurrencySafe()` | 无 | 无（Session 互斥） | 默认串行；未来 readonly 并发必须进入 Kernel Pipeline 策略 |
| 执行生命周期 | 8 步（查找→解析→验证→preHook→权限→执行→后处理→postHook） | 直接调用 handler | extension.before → execute → extension.after | Policy → MCP Kernel Pipeline → EventBus → Audit → AgentTimelinePart |
| Hooks | PreToolUse / PostToolUse（可 block） | 无 | tool.execute.before / after | Host-owned hooks + Kernel EventBus 只读通知 |
| 输出截断 | 自动 compact 大输出 | max_result_size_chars | `Truncate.output()` 写磁盘 | **无** |
| MCP 集成 | MCPTool 包装 + 合并 | mcp_tool.py 适配 | MCP 独立模块 | **完整 MCP 协议引擎**（7 内置 Server + 5 传输层） |
| 自定义工具 | .claude/tool/*.ts | Python 模块自注册 | .opencode/tool/*.ts | Extension contributes + MCP Server |
| 工具集分组 | 无显式分组 | TOOLSETS 定义 | 无显式分组 | Agent 工作模式决定工具集 |

---

## 五、Provider / 模型适配对照

| 设计点 | Claude Code | Hermes Agent | OpenCode | Navis Go |
|--------|------------|--------------|----------|------|
| 适配层设计 | 直接调用 Anthropic SDK | ProviderProfile dataclass + adapters | Provider + AI SDK 25+ bundled | **ProviderAdapter trait + Gateway** |
| 支持数量 | 1（Anthropic） | 6+ | **25+**（全部 AI SDK 提供商） | 2（Anthropic + OpenAI） |
| 协议适配 | 无（直接 Messages API） | 4 个适配器 | 统一 AI SDK 接口 | **ChatCompletions + Responses + Custom** |
| 流式传输 | SSE 原生 | SSE 流式 | AI SDK streamText + native | **StreamSender/Receiver 内部通道** |
| 模型路由 | 无（单一模型） | Provider 自动检测 | models.dev API + 扩展 hook | **ModelRouter 按名称查找** |
| 重试策略 | 3 次恢复 | jittered_backoff 指数退避 | Effect retry 组合子 | **指数退避（1→30s）+ CircuitBreaker** |
| 凭证管理 | API Key / OAuth | **CredentialPool 多 key 轮转** | 环境变量 + 扩展 auth.loader | API Key 存储 |
| Quota/成本 | 无 | 无 | 无 | **QuotaManager + CostTracker** |
| 离线降级 | 无 | 无 | 无 | **OfflineDetector** |
| Provider 边界 middleware 定义 | 无 | 无 | 无 | **GatewayMiddlewareSet + Kernel Pipeline** |

---

## 六、权限系统对照

| 设计点 | Claude Code | Hermes Agent | OpenCode | Navis Go |
|--------|------------|--------------|----------|------|
| 权限模式 | 4 种（default/plan/bypass/auto） | 无显式模式 | Agent 级权限规则 | 审批模式（FullAuto 等） |
| 规则类型 | **allow / deny / ask** | 无 | allow / ask / deny | Sandbox 黑白名单 |
| 规则来源 | **9 层合并** | 无 | 4 层合并 | Sandbox 配置 |
| 自动判断 | **LLM 安全分类器**（52KB） | 无 | 无 | 无 |
| UI 交互 | 权限请求对话框（React Hook） | 无 | ctx.ask() 对话 | confirm_handler.rs |
| 文件系统权限 | 独立模块（62KB） | 无 | glob 模式匹配 | Sandbox 路径管控 |
| 扩展权限 | 无 | 无 | 无 | **ExtensionPermissions**（6 维管控） |
| Hook 驱动 | PreToolUse hook 可 block | 无 | 扩展 tool.execute.before | 无 |

---

## 七、扩展性设计对照

| 扩展类型 | Claude Code | Hermes Agent | OpenCode | Navis Go |
|---------|------------|--------------|----------|------|
| 工具扩展 | Tool 接口 + .claude/tool/ | Python 模块自注册 | .opencode/tool/*.ts | MCP Server + Extension contributes |
| 扩展系统 | LoadedExtension（4 种能力） | extensions/ 延迟导入 | npm 包 + 本地文件 + 内置 | **16 种 contributes 类型** |
| MCP 协议 | 完整客户端（独立包 122KB） | mcp_tool.py 适配 | MCP 独立模块 | **完整引擎**（7 Server + 5 传输） |
| 技能系统 | .claude/skills/ + DiscoverSkills | skills/ 21 分类 | SKILL.md 文件 | skills/ 双模式 |
| 命令扩展 | **100+ 斜杠命令** | 无 | command 配置 | 命令面板 |
| Hook 扩展 | settings.json hooks 配置 | 无 | 扩展 hooks | Kernel EventBus 只读订阅 |
| 自定义 Agent | AgentTool（委派独立工作体） | delegate_task 委派 | 配置文件 agent 字段 | Agent 工作模式 + Task Sidechain |
| Feature Flag | **30+ 编译时宏** | 无 | 无 | 无 |
| 主题扩展 | 无 | 无 | 无 | **Extension contributes themes** |
| 编辑器扩展 | 无（终端编辑） | 无 | 无 | **Extension contributes editor_extensions** |
| 传输适配 | stdio/SSE/HTTP | 无 | stdio/SSE | **5 种（stdio/SSE/WS/REST/gRPC）** |

---

## 八、会话 / 上下文管理对照

| 设计点 | Claude Code | Hermes Agent | OpenCode | Navis Go |
|--------|------------|--------------|----------|------|
| 会话存储 | 文件系统 JSON | SQLite + FTS5 | SQLite (Drizzle ORM) | SQLite (rusqlite) |
| 消息结构 | 6 种消息类型 | role + content 列表 | **Part 化**（8 种 Part 类型） | ChatMessage 结构 |
| 上下文压缩 | **4 种策略**（auto/reactive/snip/micro） | ContextCompressor（摘要 20%） | compaction Agent | context_compress.rs |
| 压缩触发 | token 接近窗口自动 | 迭代式摘要 | needsCompaction 标志 | 长会话自动触发 |
| 记忆系统 | **Memdir + extractMemories + teamSync** | MEMORY/USER/SOUL.md | 无 | memory.recall/store MCP 工具 |
| 会话恢复 | resume 命令 | SessionDB 持久化 | SQLite 持久化 | checkpoint + snapshot |
| 会话统计 | token 用量追踪 | 无 | **cost/tokens 累积** | 无 |
| 会话分享 | 无 | 无 | **share URL** | 无 |
| 会话导出 | 无 | 无 | 无 | **Markdown/JSON 导入导出** |

---

## 九、配置系统对照

| 设计点 | Claude Code | Hermes Agent | OpenCode | Navis Go |
|--------|------------|--------------|----------|------|
| 配置层次 | **9 层合并** | 4 层 | 8 层 | Config 模块 |
| 格式 | JSON + Zod 校验 | YAML | JSONC | TOML/JSON |
| 变量替换 | 无 | `${VARIABLE}` | `${VARIABLE}` | 无 |
| 配置热更新 | useSettingsChange 监听 | 无 | 无 | **支持** |
| 企业管控 | managed + MDM + 远程 | 无 | enterprise + MDM | 无 |
| 校验 | **Zod schema** | 无 | 无 | 无 |

---

## 十、设计模式对照

| 设计模式 | Claude Code | Hermes Agent | OpenCode | Navis Go |
|---------|------------|--------------|----------|------|
| 依赖注入 | React Context + 手动传参 | 构造函数参数注入 | Effect-TS Layer/Context | **Tauri app.manage() + State<T>** |
| 状态管理 | 自定义 store (类 Zustand) | 实例属性 + SQLite | Effect InstanceState | SolidJS createStore + Tauri State |
| 事件系统 | EventEmitter + Hooks | 回调函数注入 | EventV2Bridge (pub/sub) | `crate::kernel::EventBus`（离散状态通知） |
| 流式处理 | AsyncGenerator | 同步 + _run_async 桥接 | Effect Stream | **tokio::sync::mpsc channel** |
| 错误处理 | 分类 Error + Sentry | FailoverReason 枚举 | TaggedErrorClass | **Result<T,E> + tracing** |
| 并发模型 | Promise.allSettled 分区 | 线程 + asyncio 混合 | Effect fiber | **tokio async + spawn** |
| 工厂模式 | buildTool() 安全默认值 | registry.register() | Tool.Def + Effect Schema | MCP ToolDefinition + Capability facade |
| 策略模式 | 权限 allow/deny/ask | ProviderProfile 声明式 | Agent permission 规则 | Kernel Policy + Sandbox constraints |
| 观察者模式 | useAppState(selector) | 回调函数 | Effect subscription | Kernel EventBus + Tauri event |
| 管道模式 | toolOrchestration 分区 | 无 | pipeline() 函数 | **Kernel Pipeline；Gateway 仅保留边界 hook 链** |
| 桥接模式 | 无 | _run_async（同步↔异步） | EffectBridge（Effect↔Promise） | **StreamSender/Receiver（后端↔IPC）** |

---

## 十一、Navis Go 独有优势

| # | 优势 | 说明 | 三者均不具备 |
|---|------|------|-------------|
| 1 | **Rust 后端性能** | 内存安全 + 零成本抽象 + 无 GC 停顿 | ✅ |
| 2 | **桌面原生体验** | Tauri 2 原生窗口（~5MB vs Electron ~150MB） | ✅（OpenCode 用 Electron） |
| 3 | **完整 MCP 引擎** | 7 内置 Server + 17 工具 + 5 传输适配器 | ✅（CC 只是客户端，Hermes 简单适配） |
| 4 | **Gateway 生产级设计** | Quota/成本/离线/中间件/熔断 全链路 | ✅ |
| 5 | **Agent 显式状态机** | 7 种状态 + 状态转换守卫（编译期安全） | ✅（CC 隐式循环，Hermes 无状态机） |
| 6 | **16 种扩展扩展类型** | 覆盖 Gateway/Agent/MCP/UI/Editor 全链路 | ✅（CC 4 种，OpenCode 6 种 Hook） |
| 7 | **34 篇设计文档** | 全部完成，设计驱动开发 | ✅ |
| 8 | **自我进化系统** | 经验捕获 + 模式提取 + 反思分析 | ✅（仅 Hermes 有简单记忆） |
| 9 | **Provider 边界 hook 链** | Token 计数 / 前缀稳定 / 限流 | ✅ |
| 10 | **Extension 沙箱隔离** | 路径/命令/网络/资源配额 6 维管控 | ✅ |

---

## 十二、Navis Go 需要补齐的差距

| # | 差距 | 当前状态 | 参考来源 | 影响 |
|---|------|---------|---------|------|
| 1 | **Provider 数量不足** | 仅 2 个（Anthropic/OpenAI） | OpenCode 25+ | 用户覆盖面窄 |
| 2 | **凭证池轮转** | 无多 API key 轮转 | Hermes CredentialPool | 限流时无法自动切换 |
| 3 | **工具生命周期 Hooks** | 无 PreToolUse/PostToolUse | CC toolHooks.ts | 扩展无法拦截/修改工具行为 |
| 4 | **权限分级粗糙** | 仅 Sandbox 黑白名单 | CC 9 层规则合并 + LLM 分类器 | 安全粒度不足 |
| 5 | **配置层次单一** | 仅 Config 模块 | CC 9 层合并 | 无法支持企业管控/项目隔离 |
| 6 | **工具输出截断** | 无 | OpenCode Truncate.output() | 大输出可能撑爆上下文 |
| 7 | **Doom Loop 检测** | 无 | OpenCode processor.ts | 工具死循环风险 |
| 8 | **Feature Flag** | 无编译时条件编译 | CC feature() 宏 30+ | 无法灵活裁剪功能集 |
| 9 | **技能自改进** | 无自动优化 | Hermes skills 自改进 | 技能质量不会随使用提升 |
| 10 | **多平台终端** | 仅本地终端 | Hermes 6 种后端（docker/ssh/modal 等） | 无远程开发能力 |
| 11 | **会话统计** | 无 cost/tokens 累积 | OpenCode Session.Info | 用户无法感知成本 |
| 12 | **配置校验** | 无 schema 校验 | CC Zod schema | 配置错误无法提前发现 |

---

## 十三、复刻 / 优化优先级

### P0 — 底座夯实（直接影响可用性与安全性）

| # | 功能 | 来源 | 工作量 | 理由 |
|---|------|------|--------|------|
| 1 | 工具生命周期 Hooks | Claude Code | 中 | 扩展性核心，扩展拦截/修改工具行为的基础 |
| 2 | 多 Provider 扩展 + 凭证池轮转 | OpenCode + Hermes | 大 | 用户覆盖面，直接决定产品可用性 |
| 3 | 权限分级（deny/allow/ask 规则合并） | Claude Code | 中 | 安全性核心，现有 Sandbox 粒度不足 |
| 4 | 工具输出截断 + Doom Loop 检测 | OpenCode | 小 | 鲁棒性，防止工具失控撑爆上下文 |

### P1 — 核心收紧（提升工程质量与灵活性）

| # | 功能 | 来源 | 工作量 | 理由 |
|---|------|------|--------|------|
| 5 | 配置层次扩展（全局/用户/项目 3 层合并） | Claude Code | 小 | 配置灵活性，支持项目级隔离 |
| 6 | 会话 cost/tokens 统计 | OpenCode | 小 | 用户体验，成本感知 |
| 7 | 配置 schema 校验 | Claude Code | 小 | 防止配置错误 |

### P2 — 扩展留好（长期竞争力）

| # | 功能 | 来源 | 工作量 | 理由 |
|---|------|------|--------|------|
| 8 | Feature Flag 编译时宏 | Claude Code | 中 | 构建灵活性，代码瘦身 |
| 9 | 技能自改进循环 | Hermes | 大 | 长期竞争力 |
| 10 | 多平台终端后端 | Hermes | 大 | 远程开发场景 |
| 11 | 会话分享机制 | OpenCode | 中 | 协作场景 |

---

## 十四、核心设计思想提炼

### 从 Claude Code 学到的

1. **工具即契约**：Tool 接口定义了完整的生命周期（定义→注册→校验→权限→执行→Hook→渲染），每个环节可插拔
2. **权限是系统的一等公民**：9 层规则合并 + LLM 分类器，权限不是附加功能而是架构核心
3. **Hook 是扩展的通用语言**：PreToolUse/PostToolUse/Stop/Compact/SessionStart，通过配置文件即可扩展行为
4. **编译时裁剪**：30+ Feature Flag 实现同一代码库产出不同功能集的构建

### 从 Hermes Agent 学到的

1. **自注册工具**：模块导入即注册，零配置发现，降低工具开发门槛
2. **凭证池**：多 API key 自动轮转 + 冷却期管理，生产环境必备
3. **记忆三件套**：MEMORY（知识）+ USER（用户）+ SOUL（人格），形成完整的 Agent 记忆体系
4. **Toolsets 分组**：按场景定义工具集（web/research/full_stack），而非按技术分类

### 从 OpenCode 学到的

1. **Effect-TS 全面应用**：依赖注入 + 流式处理 + 并发控制 + 错误处理统一在一个范式下
2. **Part 化消息**：消息不再是纯文本，而是 text/tool/reasoning/file/step/patch/compaction 的组合
3. **6 层扩展点**：Provider → 工具 → Agent → Skill → MCP → Extension，每层独立扩展互不干扰
4. **输出截断**：大输出写磁盘返回预览，防止工具结果撑爆上下文窗口

### Navis Go 应坚持的

1. **Rust 为底座**：内存安全 + 性能优势是桌面工具的核心竞争力
2. **MCP 为统一协议**：工具、传输、Server 全部统一到 MCP 标准，不另造轮子
3. **状态机为决策核心**：显式状态比隐式循环更可维护、更可测试
4. **Gateway 为统一入口**：所有 LLM 调用走 Gateway，Provider 差异在此层屏蔽
5. **设计文档先行**：34 篇设计文档是项目最大资产，继续保持

---

## 附录：三项目关键路径速查

### Claude Code 关键文件

| 路径 | 说明 |
|------|------|
| `src/query.ts` | Agent 决策循环核心（AsyncGenerator） |
| `src/Tool.ts` | Tool 接口定义 + buildTool 工厂 |
| `src/tools.ts` | 工具注册和组装（唯一真相来源） |
| `src/services/tools/toolOrchestration.ts` | 工具编排（并发/串行分区） |
| `src/services/tools/toolExecution.ts` | 工具执行生命周期（8 步） |
| `src/services/tools/toolHooks.ts` | Pre/Post ToolUse Hooks |
| `src/utils/permissions/permissions.ts` | 权限检查核心（9 层规则合并） |
| `src/utils/permissions/yoloClassifier.ts` | LLM 安全分类器 |
| `src/utils/settings/settings.ts` | 多层 settings 合并 |
| `src/services/mcp/client.ts` | MCP 客户端集成 |

### Hermes Agent 关键文件

| 路径 | 说明 |
|------|------|
| `run_agent.py` | AIAgent 主类定义 |
| `agent/conversation_loop.py` | 对话循环核心（3900 行） |
| `tools/registry.py` | 工具注册中心（自注册模式） |
| `model_tools.py` | 工具编排层 + 异步桥接 |
| `providers/base.py` | ProviderProfile 声明式基类 |
| `agent/credential_pool.py` | 凭证池轮转 |
| `agent/context_compressor.py` | 上下文压缩 |
| `agent/memory_manager.py` | 记忆管理（MEMORY/USER/SOUL.md） |
| `toolsets.py` | 工具集分组定义 |
| `hermes_state.py` | SQLite 状态存储 + FTS5 |

### OpenCode 关键文件

| 路径 | 说明 |
|------|------|
| `packages/opencode/src/agent/agent.ts` | Agent 定义 + Info Schema |
| `packages/opencode/src/session/processor.ts` | Agent 循环处理器（流式事件） |
| `packages/opencode/src/tool/tool.ts` | Tool 接口定义 |
| `packages/opencode/src/tool/registry.ts` | 工具注册表（builtin + custom） |
| `packages/opencode/src/session/llm.ts` | LLM 流式调用（双运行时） |
| `packages/opencode/src/provider/provider.ts` | Provider 系统（25+ 提供商） |
| `packages/opencode/src/config/config.ts` | 配置系统（8 层合并） |
| `packages/opencode/src/extension/index.ts` | 扩展系统（Hook 接口） |
| `packages/opencode/src/permission/index.ts` | 权限系统 |
| `packages/opencode/src/session/session.ts` | 会话管理 |

---

> 文档生成时间：2026-06-06
> 基于 claude-code-main (v2.6.10)、hermes-agent-main、opencode-dev 源码分析
