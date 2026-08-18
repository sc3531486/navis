# 历史审计快照

> 本文记录 2026-08-17 之前的重构盘点，文中的旧路径仅用于解释当时的迁移来源，不代表当前目录结构或待办状态。当前架构以根目录 `ARCHITECTURE_REVIEW.md`、`MIGRATION-PLAN.md` 和 `AGENTS.md` 为准。

# Context

用户要求对全项目状态模型进行一次性全面治理：能统一的统一、能提取的提取为公共模块，并删除已验证不再使用的代码或文件，目标是高内聚、低耦合。此前已完成 Agent Timeline 的运行态动画和三份前端验证脚本修复，后续重构不得回退这些结果。用户明确要求暂不处理 Gateway SSRF/API Key 外泄；该项不纳入本计划。

全量盘点确认：项目存在多个合理但彼此漂移的领域状态体系（Task、Workflow、Tool、Timeline、Extension、Kernel、Terminal、Session、Todo、Stream）。问题不是 enum 数量，而是跨边界裸字符串、重复的活跃/终态判断、重复的状态文案/动效、两套 Channel 生命周期、无所有者的 singleton polling、Extension/Kernel 双状态持久化、以及重复的 Circuit Breaker。建议采用“领域状态保留事实 + 公共分类与展示投影统一行为”的架构，拒绝用万能 enum 抹平领域语义。

## Recommended approach

### 1. 建立 Rust 公共状态分类与可靠性基础设施

**新增：**
- `src-tauri/src/foundation/status/mod.rs`
- `src-tauri/src/foundation/status/classification.rs`
- `src-tauri/src/foundation/status/timeline.rs`
- `src-tauri/src/foundation/reliability.rs`

**修改：**
- `src-tauri/src/foundation/mod.rs`
- `src-tauri/src/ai/gateway/retry.rs`
- `src-tauri/src/tool/mcp/reliability.rs`
- 各消费模块的 import 与单元测试

**实施：**
- 定义不反向控制领域迁移的公共分类：`StatusPhase`（queued/active/waiting/succeeded/failed/cancelled/skipped/inactive）、`StatusOutcome`、`StatusAttention`、`StatusPresentation` 以及 `StatusClassify` trait。
- 为 Task、Workflow、Workflow Step、Skills Step、Tool Call、Agent Tool、Confirm、Preview、Extension、Kernel 状态分别实现分类/展示投影；保留各自 enum 与合法转移，不强行合并领域状态。
- Timeline 建立强类型 `TimelineStatus` 与 legacy parser：新写入拒绝裸字符串，历史数据库和扩展输入继续从字符串容错解析；未知值原样保留并投影为安全 fallback，绝不默认完成/成功。
- 将 Gateway 与 MCP 共用的 `CircuitState`、CircuitBreaker、退避/重试基础能力抽到 `foundation/reliability`。保留 Gateway 的 HTTP 可重试状态码策略和 MCP 的领域超时语义为适配层。先用回归测试锁定两边的现有行为，再删除旧重复实现。

### 2. 收紧 Rust 执行状态与 DTO/存储边界

**修改：**
- `src-tauri/src/ai/agent/task_manager.rs`
- `src-tauri/src/ai/agent/workflow.rs`
- `src-tauri/src/extension/skills/mod.rs`
- `src-tauri/src/tool/agent/result.rs`
- `src-tauri/src/tool/agent/runtime/events.rs`
- `src-tauri/src/foundation/storage/session_store/models.rs`
- `src-tauri/src/foundation/storage/session_store.rs`
- `src-tauri/src/ui/agent_timeline_part.rs`
- `src-tauri/src/ui/timeline.rs`
- `src-tauri/src/ui/tasks/task_projection.rs`
- `src-tauri/src/ui/dto.rs`
- `src-tauri/src/ui/extensions.rs`
- `src-tauri/src/extension/extension/models.rs`
- `src-tauri/src/kernel/registry/mod.rs`
- `src-tauri/src/project/session/manager.rs`

**实施：**
- 保留 Task/Workflow/Extension/Kernel 的领域状态事实，但输出 `statusPresentation` 给 UI DTO；保留原始 `status` 字段以兼容 IPC、历史记录和扩展协议。
- 将 ToolCall 与 AgentTool 的 `executing/running`、`waiting_confirm/waiting_permission`、`failed/error`、`cancelled/aborted` 的跨层转换集中为命名 mapper，替换散落字符串匹配；事件内部尽量使用强类型 phase/status，序列化保持既有 wire value。
- Timeline 的 active/terminal 逻辑全部改从 TimelineStatus / StatusPresentation 获取，移除 Rust UI、timeline、SQL 收口中的重复字符串集合；`denied`、`reused`、`compacted` 具备明确展示投影。
- `SessionStatus`、`SessionChange.status` 改为显式解析失败或 Unknown 投影，禁止未知状态静默归类为 Active/Success。
- ExtensionStatus 与 Kernel LifecycleState 保持两层事实，但通过唯一 typed projection 输出，不再在 metadata 保存可漂移的第二个 `kernel_status` 字符串源。
- 审查 `AgentStateMachine`：若接入真实 turn runner，则用它驱动 Task/Timeline 投影；若无法证明生产需要，则删除未接线状态机及只为其存在的 API/测试，避免空转的状态事实源。
- 对 `ThinkingStatus::Completed` 与 LSP 的 `IndexStatus + indexed bool` 进行消费者核对：删除不可达 variant 或冗余 bool，并补 transition/projection 测试。

### 3. 建立前端公共状态展示与运行时所有权模块

**新增：**
- `src/lib/status/types.ts`
- `src/lib/status/presentation.ts`
- `src/lib/status/time.ts`
- `src/lib/status/polling.ts`
- `src/lib/status/index.ts`

**修改：**
- `src/lib/stream/types.ts`
- `src/lib/agent-timeline/timeline-order.ts`
- `src/lib/agent-timeline/index.ts`
- `src/components/AgentTimeline/tool-kind.ts`
- `src/components/AgentTimeline/tool-label.ts`
- `src/components/AgentTimeline/AgentTimelineView.tsx`
- `src/components/AgentTimeline/GenericToolStep.tsx`
- `src/stores/agent-runtime.ts`
- `src/stores/task-projection.ts`
- `src/stores/session-todos.ts`
- `src/components/Chat/AgentBatchSummary.tsx`
- `src/components/WorkspacePanel/shared.tsx`
- `src/components/WorkspacePanel/BackgroundTasksPanel.tsx`
- `src/components/WorkspacePanel/PlanPanel.tsx`
- `src/components/Composer/Composer.tsx`
- `src/components/Settings/ExtensionsManager.tsx`
- `src/components/Terminal/TerminalPanel.tsx`（仅在确认需实际 terminal store 时）

**实施：**
- 对齐 Rust DTO 定义 `StatusPresentation` 前端类型，集中提供 `statusLabel`、`statusClass`、`isStatusLive`、`isStatusTerminal`、`statusMotion`、ARIA 文案和时长格式化。
- Timeline 只保留工具类别、详情和 renderer 决策；将 `isLiveTimelineStatus` / `isActiveTimelineStatus` 收敛为 `lib/agent-timeline` 单一 API，避免 UI 层重复生命周期集合。
- Task/Todo/Extension/Timeline 的完整/紧凑文案改经领域 presentation mapper 输出，消除 “Waiting/Waiting confirmation”“Stopped/Cancelled” 等漂移；保留少数经过确认的上下文文案差异作为 formatter option，而非重复 switch。
- Agent runtime 以 `isTimelineActionPart(part) && isTimelineLiveStatus(part.status)` 推导 tool_calling，不再仅识别 `kind === 'tool' && status === 'running'`。
- 用引用计数的 `subscribe...` polling API 取代模块级 start/stop singleton：每个 Chat、Composer、PlanPanel、BackgroundTasksPanel mount 时 acquire，cleanup 时 release；同 session/资源只有一个 interval；最后订阅者离开立即停止；使用 generation token 防止旧 session 响应覆盖新状态。
- 默认保持原轮询周期作为行为基线，测试通过后再考虑按活跃度降频。
- 删除 Terminal 未接入类型，或只有在建立实际 terminal session store 后保留：不可保留“未来可能用”的死类型。

### 4. 统一 CSS 状态 token、动效与可访问性

**修改：**
- `src/styles/shared/animations.css`
- `src/styles/chatMessages/agent-timeline.css`
- `src/styles/rightWorkspace/tasks-panel.css`
- `src/styles/rightWorkspace/plan-panel.css`
- `src/styles/composer/run-status.css`
- `src/styles/settings/extensions.css`
- `src/styles/statusbar/statusbar.css`
- `src/styles/shared/shell-window.css`
- 相关组件 class 输出

**实施：**
- 使用公共 presentation class/token 表达 active/waiting/succeeded/failed/cancelled/skipped/inactive；在过渡期保留 `is-running`、`is-waiting_permission` 等兼容 class，避免同时大规模重命名 CSS 和状态协议。
- 维持已实现的 Timeline group spinner、步骤 blink/pulse、shimmer 及 reduced-motion 行为。
- 将 `navis-status-ready-pulse` 移至共享 animations，解除 shared shell window 对 statusbar CSS 的隐式依赖。
- 删除确认无引用的 `navis-gateway-pulse`、`navis-thinking-icon-active` keyframes。
- 所有持续动效统一受 `prefers-reduced-motion` 控制；保留状态文字、颜色和 aria-live，不以动画作为唯一状态信号。

### 5. 合并两套 Channel 生命周期实现

**新增：**
- `src/lib/stream/channel-run.ts`

**修改：**
- `src/lib/stream/runChannelStream.ts`
- `src/lib/stream/useChannel.ts`
- `src/components/Terminal/TerminalPanel.tsx`
- `src/stores/chat-turn-stream.ts`
- `src/lib/stream/types.ts`

**实施：**
- 提取标准 Stream envelope、run ownership/generation、streamId 捕获、exactly-once completion、远端 cancel、stale callback 丢弃的 transport core。
- `runChannelStream` 仅保留 Chat 业务回调；`useChannel` 仅负责 Solid signal 和 `onCleanup` 接入。
- 统一 `cancelled`、`done`、`error` 的 callback 时机；停止时固定当前 run 的 streamId，不让旧/新 run 互相影响。
- 为 Terminal 重连、组件卸载、延迟 invoke resolve/reject、旧 chunk 迟到增加回归测试；不改变 Terminal/Chat 的公开业务语义。

### 6. 删除确认无用代码和文件，并更新导出/测试

**删除：**
- `src/stores/session.ts`，以及 `src/stores/index.ts` 中对应 export/type export。
- `src/lib/stream/types.ts` 内未接入的 `PtyStatus`、`PtyProcess`、`PanelVisibility`（若全仓引用和终端回归确认零消费者）。
- 统一可靠性实现完成后，`src-tauri/src/tool/mcp/reliability.rs` 和/或 `src-tauri/src/ai/gateway/retry.rs` 中被迁移的重复类型与实现；仅保留领域适配器，若整文件无余留则删除文件和 mod export。
- 未接入 `AgentStateMachine`（若接线审计证实无生产用途），连同无效 public re-export。
- `.tmp/verify-*` 中遗留构建产物，不删除测试脚本的临时目录机制。
- `src-tauri/test_rb_size`、`src-tauri/test_rb_source` 仅在完整 Rust 测试、文件删除/recycle-bin 关联回归和全仓引用扫描均证明无消费者后删除。

**保留：**
- `StreamChunkKind` / `StreamCancelToken` 的独立语义。
- Session archive、Git diff、Preview、Terminal exit code 等领域事实状态，不纳入执行状态万能模型。
- 已修复的 Timeline 动画和 `test:stream`、`test:menus`、`test:tool-renderers` 三份验证逻辑。

### 7. 验证与迁移顺序

1. 先为 Rust 分类矩阵、Timeline legacy parser、Task/Workflow/Extension 迁移、AgentTool mapper、Circuit Breaker 并发 half-open 行为写单测。
2. 新增 DTO `statusPresentation`，前端双读原 status + presentation；迁移所有 Rust 生产者后，再删除前端的散落裸字符串判断。
3. 在每个 polling 消费者替换为 acquire/release 后，测试同 session 单 interval、最后 release 停止、session 切换 stale response 防护、Plan mode/面板卸载 cleanup。
4. 执行删除项前后各做全仓引用扫描，再运行：
   - `npm run test:stream`
   - `npm run test:menus`
   - `npm run test:tool-renderers`
   - `npm run build`
   - `cd src-tauri && cargo test`
   - `cd src-tauri && cargo clippy --all-targets -- -D warnings`（若当前基线可通过）
5. 手动验证：Timeline 执行/等待权限/重试/拒绝/失败/取消/完成、reduced-motion；Chat/Plan/Tasks 多面板和 session 切换无多余轮询；Terminal 重连与关闭；Extension 启禁和异常；MCP/Gateway 熔断恢复。

## Critical files

- `src-tauri/src/foundation/mod.rs`、新 `foundation/status/*`、新 `foundation/reliability.rs`
- `src-tauri/src/ai/agent/task_manager.rs`、`workflow.rs`、`state_machine.rs`
- `src-tauri/src/tool/agent/result.rs`、`runtime/events.rs`
- `src-tauri/src/ui/agent_timeline_part.rs`、`ui/timeline.rs`、`ui/tasks/task_projection.rs`
- `src-tauri/src/foundation/storage/session_store/models.rs`
- `src-tauri/src/extension/extension/models.rs`、`src-tauri/src/kernel/registry/mod.rs`
- `src/lib/status/*`、`src/lib/agent-timeline/*`、`src/lib/stream/channel-run.ts`
- `src/stores/task-projection.ts`、`src/stores/session-todos.ts`、`src/stores/agent-runtime.ts`
- `src/components/AgentTimeline/*`、`src/components/WorkspacePanel/*`、`src/components/Composer/Composer.tsx`
- `src/styles/shared/animations.css`、`agent-timeline.css`、`tasks-panel.css`、`plan-panel.css`
- `scripts/verify-chat-message-reducer.mjs`、`verify-menu-coverage.mjs`、`verify-tool-renderers.mjs`

## Verification

- Rust: status projection matrix、legacy status parser、Task/Workflow/Extension transition、AgentTool mapper、Circuit Breaker concurrency/recovery、Timeline DTO/storage compatibility、polling integration-related command tests。
- Frontend: pure state presentation/polling/channel-run tests plus the three existing scripts and production build。
- Runtime: execute every visible lifecycle path and verify label/tone/motion/ARIA, multiple panel subscriptions, session switching, terminal reconnect, extensions and stream cancellation.
- Removal gates: global reference search before deletion and all build/test checks after deletion.

