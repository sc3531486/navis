# 15 - Edit 代码编辑引擎 详细设计

> 模块编号：15 | 层级：能力层
> 依赖：01-Logger, 02-Event+IPC, 06-Sandbox, 09-File
> 被依赖：16-Agent

---

## 一、模块概述

### 1.1 定位

Edit 是代码精确编辑引擎，处理 AI 生成的代码修改，生成 Diff 预览，支持批量编辑、撤销重做。

Agent 工具链路中的文件变更事实不放在 Edit 模块私有状态里。`edit/write` 成功写盘后由 `tool/agent` 和 `project/session` change recorder 记录 `SessionChange`：Edit 负责生成和执行精确修改，Session 负责保存“本轮会话改了哪个文件、before/after/diff 是什么”。这样右侧 Review、Diff、Revert 都读取同一事实源，不把 UI 面板状态误用成文件回滚语义。

### 1.2 职责边界

```
负责：
├── 精确文本替换（基于行号/内容匹配）
├── Diff 生成（Unified/Side-by-side 格式）
├── 批量编辑（多文件多处修改）
├── 编辑预览（不立即写入，等待确认）
├── 撤销/重做
├── 编辑冲突检测
├── 编辑校验（语法检查）
└── 为调用方返回 before/after/diff/stat 所需的结构化结果

不负责：
├── 文件读写 → File
├── 安全校验 → Sandbox
├── 代码智能（补全/诊断）→ LSP
├── 编辑器渲染 → Editor
└── 会话级 Review/Revert 事实保存 → SessionChange
```

---

## 二、架构设计

```
edit/
├── mod.rs              # 模块入口
├── parser.rs           # 编辑指令解析
├── executor.rs         # 编辑执行器
├── diff_generator.rs   # Diff 生成器
├── preview.rs          # 编辑预览管理
├── batch.rs            # 批量编辑
├── undo.rs             # 撤销/重做栈
└── validator.rs        # 编辑校验
```

---

## 三、数据模型

```rust
// 编辑操作
struct EditOperation {
    file_path: PathBuf,
    edits: Vec<Edit>,
}

struct Edit {
    id: String,
    edit_type: EditType,
    range: Option<Range>,        // 行范围
    old_text: Option<String>,    // 旧文本（替换/删除时）
    new_text: Option<String>,    // 新文本（替换/插入时）
}

enum EditType {
    Replace,    // 替换指定行
    Insert,     // 在指定行后插入
    Delete,     // 删除指定行
}

struct Range {
    start_line: usize,
    start_col: Option<usize>,
    end_line: usize,
    end_col: Option<usize>,
}

// Diff 结果
struct DiffResult {
    edit_id: String,
    file_path: String,
    hunks: Vec<DiffHunk>,
    stats: DiffStats,
}

struct DiffHunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    changes: Vec<DiffChange>,
}

struct DiffChange {
    change_type: ChangeType,  // add / delete / context
    line_number: usize,
    content: String,
}

struct DiffStats {
    additions: usize,
    deletions: usize,
    files_changed: usize,
}

// 编辑预览
struct EditPreview {
    id: String,
    session_id: String,
    operations: Vec<EditOperation>,
    diffs: Vec<DiffResult>,
    status: PreviewStatus,
    created_at: DateTime<Utc>,
}

**定位方式说明：**

| 方式 | 字段 | 适用场景 | 优先级 |
|------|------|---------|--------|
| 文本匹配 | old_text / new_text | AI 生成的代码修改（推荐） | 优先使用 |
| 行号定位 | start_line / end_line | 已知精确行号的编辑 | fallback |

当 old_text 和 start_line 同时存在时，优先使用文本匹配。文本匹配在文件被其他编辑修改后仍能正确定位，而行号可能错位。

enum PreviewStatus {
    Pending,     // 等待用户确认
    Applied,     // 已确认并应用（确认即应用，不再区分 Confirmed 和 Applied）
    Rejected,    // 已拒绝
}
```

---

## 四、接口定义

```typescript
// 创建编辑预览
edit.preview(sessionId: string, operations: EditOperation[]): Promise<EditPreview>

// 确认/拒绝
edit.confirm(editId: string): Promise<void>
edit.reject(editId: string): Promise<void>

// 撤销/重做
edit.undo(editId?: string): Promise<void>
edit.redo(editId?: string): Promise<void>

// 查询
edit.getPreview(editId: string): Promise<EditPreview>
edit.listPreviews(sessionId: string): Promise<EditPreview[]>
edit.canUndo(): Promise<boolean>
edit.canRedo(): Promise<boolean>
```

---

## 五、事件定义

```typescript
type EditEvents = {
  'edit.preview.created':   { sessionId: string; editId: string; fileCount: number; files: string[] }
  'edit.preview.confirmed': { sessionId: string; editId: string }
  'edit.preview.rejected':  { sessionId: string; editId: string }
  'edit.applied':           { sessionId: string; editId: string; files: string[] }
  'edit.undone':            { sessionId: string; editId: string }
  'edit.redone':            { sessionId: string; editId: string }
  'edit.failed':            { sessionId: string; editId: string; error: string }
  'edit.conflict.detected': { sessionId: string; editId: string; filePath: string; conflictType: string }
}
```

`edit.applied` 事件不是 Review 的事实源。`tool/agent` 必须在同一 `callId` 的 completed `AgentTimelinePart` 落库后，经 `project/session` change recorder 写入 `session.change.recorded`，并用 `agentTimelinePartId` 关联展示步骤。文件恢复时写入 `session.change.reverted`，真实恢复文件内容后再更新状态；如果缺少 `beforeContent`，必须报错。

---

## 六、Edit 与 Sandbox 审批模式的关系

Code Edit 的预览与写入行为受 Sandbox 审批模式（ApprovalMode）控制，具体对应关系如下：

### 6.1 各模式下的行为

| 审批模式 | Preview 行为 | 写入行为 | 说明 |
|----------|-------------|----------|------|
| Suggest | 必须展示 preview | 用户确认后才写入 | Agent 只生成编辑建议，等待用户逐条确认 |
| AutoEdit | 仍然展示 preview | Agent 可自动写入（跳过 preview） | preview 作为参考展示，但不阻塞写入流程 |
| FullAuto | 不阻塞写入 | Agent 自动写入已授权路径 | 仍受 Sandbox 路径白名单、Project Trust 和危险操作 denylist 约束 |

### 6.2 批量编辑的特殊规则

无论当前处于哪种审批模式，当单次编辑涉及 **超过 5 个文件** 时，始终需要用户确认：

```
Agent 发起批量编辑
     │
     ▼
涉及文件数 > 5？
     │
     ├── 是 → 弹出确认框，列出所有待编辑文件，等待用户确认
     │
     └── 否 → 按当前 ApprovalMode 的规则执行
```

### 6.3 实现要点

```rust
impl EditExecutor {
    fn should_require_preview(&self, operations: &[EditOperation]) -> bool {
        let mode = self.sandbox.get_approval_mode();
        let file_count: usize = operations.iter()
            .map(|op| op.file_path.clone())
            .collect::<HashSet<_>>()
            .len();

        // 批量编辑（>5 文件）始终需要确认
        if file_count > 5 {
            return true;
        }

        match mode {
            ApprovalMode::Suggest => true,   // 必须展示 preview
            ApprovalMode::AutoEdit => true,  // 展示 preview 但不阻塞
            ApprovalMode::FullAuto => false,  // 不展示 preview
        }
    }
}
```

---

## 七、测试策略

```
单元测试：精确替换、Diff 生成、撤销重做
集成测试：多文件批量编辑、预览确认流程、冲突检测
```
