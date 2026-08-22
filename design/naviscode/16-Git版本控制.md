# 21 - Git 版本控制 详细设计

> 模块编号：21 | 层级：高级能力层
> 依赖：01-Logger, 02-Event+IPC, 05-Auth, 06-Sandbox, 09-File, 11-Terminal
> 被依赖：16-Agent

---

## 一、模块概述

### 1.1 定位

Git 封装常用 Git 操作，提供状态检测、差异查看、提交推送、凭证管理能力。

### 1.2 职责边界

```
负责：
├── Git 状态检测（变更文件列表、分支信息）
├── Git 差异查看（diff、staged diff）
├── Git 操作封装（commit、push、pull、branch、merge）
├── 凭证管理（联动 Auth）
├── 冲突检测与提示
└── Git 日志查询

不负责：
├── 终端直接执行 Git 命令 → Terminal
├── 文件操作 → File
├── 凭证存储 → Auth
└── 安全校验 → Sandbox（高危 Git 操作需确认）
```

---

## 二、架构设计

```
git/
├── mod.rs              # 模块入口
├── operations.rs       # Git 操作封装
├── status.rs           # 状态检测
├── diff.rs             # 差异查看
├── branch.rs           # 分支管理
├── credential.rs       # 凭证管理
└── log.rs              # 日志查询
```

---

## 三、数据模型

```rust
struct GitStatus {
    repo_path: PathBuf,
    branch: String,
    upstream: Option<String>,
    ahead: usize,
    behind: usize,
    changes: Vec<GitChange>,
    is_clean: bool,
}

struct GitChange {
    path: PathBuf,
    status: ChangeStatus,
    staged: bool,
}

enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed { from: PathBuf },
    Untracked,
}

struct GitCommit {
    sha: String,
    author: String,
    email: String,
    message: String,
    timestamp: DateTime<Utc>,
}

struct GitBranch {
    name: String,
    is_current: bool,
    is_remote: bool,
    upstream: Option<String>,
    last_commit: GitCommit,
}

struct MergeResult {
    success: bool,
    conflicts: Vec<String>,       // 冲突文件路径列表
    merged_files: Vec<String>,    // 成功合并的文件路径列表
}
```

---

## 四、接口定义

```typescript
// 状态
git.status(repoPath: string): Promise<GitStatus>
git.isRepo(path: string): Promise<boolean>

// 差异
git.diff(repoPath: string, staged?: boolean): Promise<string>
git.diffFile(repoPath: string, filePath: string): Promise<string>

// 提交
git.commit(repoPath: string, message: string): Promise<string>  // sha
git.stage(repoPath: string, files: string[]): Promise<void>
git.unstage(repoPath: string, files: string[]): Promise<void>
git.stageAll(repoPath: string): Promise<void>

// 推送/拉取
git.push(repoPath: string, remote?: string, branch?: string): Promise<void>
git.pull(repoPath: string, remote?: string, branch?: string): Promise<void>

// 分支
git.branches(repoPath: string): Promise<GitBranch[]>
git.currentBranch(repoPath: string): Promise<string>
git.createBranch(repoPath: string, name: string): Promise<void>
git.switchBranch(repoPath: string, name: string): Promise<void>
git.deleteBranch(repoPath: string, name: string): Promise<void>

// 日志
git.log(repoPath: string, limit?: number): Promise<GitCommit[]>

// 合并
git.merge(repoPath: string, branch: string): Promise<MergeResult>

// Worktree 管理（供 Agent 并行任务和 Task Sidechain 使用）
git.worktree.add(branch: string, path?: string): Promise<WorktreeInfo>
git.worktree.remove(path: string): Promise<void>
git.worktree.list(): Promise<WorktreeInfo[]>
git.worktree.merge(path: string): Promise<MergeResult>
```

```typescript
// 合并结果
interface MergeResult {
  success: boolean
  conflicts: string[]       // 冲突文件列表，无冲突时为空数组
  mergedFiles: string[]     // 成功合并的文件列表
}
```

**Git 操作执行路径分工：**

| 操作类型 | 推荐路径 | 原因 |
|---------|---------|------|
| status / diff / log / branch list | Git 模块（结构化 API） | 需要结构化数据供 UI 和 Agent 消费 |
| worktree create / remove / list | Git 模块（结构化 API） | 并行任务和 Task Sidechain 依赖 |
| commit / merge / rebase | Git 模块（结构化 API） | 需要 Sandbox 权限校验 |
| push / pull（需凭证注入） | Terminal MCP 工具 | 依赖 shell 环境的 credential helper |
| rebase -i（交互式） | Terminal MCP 工具 | 需要交互式 shell |

Agent 在选择执行路径时应遵循上表，避免同一操作通过两条路径执行导致行为不一致。

---

## 五、事件定义

```typescript
type GitEvents = {
  'git.status.changed':    { sessionId: string; repoPath: string; branch: string; ahead: number; behind: number; isClean: boolean; changes: GitChange[] }
  'git.branch.switched':   { sessionId: string; repoPath: string; from: string; to: string }
  'git.branch.created':    { sessionId: string; repoPath: string; name: string }
  'git.branch.deleted':    { sessionId: string; repoPath: string; name: string }
  'git.commit.created':    { sessionId: string; repoPath: string; sha: string; message: string }
  'git.push.started':      { sessionId: string; repoPath: string; remote: string; branch: string }
  'git.push.completed':    { sessionId: string; repoPath: string; remote: string; branch: string }
  'git.push.failed':       { sessionId: string; repoPath: string; error: string }
  'git.pull.started':      { sessionId: string; repoPath: string; remote: string; branch: string }
  'git.pull.completed':    { sessionId: string; repoPath: string; remote: string; branch: string }
  'git.pull.failed':       { sessionId: string; repoPath: string; error: string }
  'git.conflict.detected': { sessionId: string; repoPath: string; files: string[] }
  'git.conflict.resolved': { sessionId: string; repoPath: string; files: string[] }
  'git.merge.completed':   { sessionId: string; repoPath: string; branch: string }
  'git.merge.failed':      { sessionId: string; repoPath: string; branch: string; error: string }
}
```

---

## 六、凭证管理实现

`git.push`、`git.pull`、`git.merge` 等涉及远端操作的命令，在执行前自动获取凭证。

```
git.push(repoPath, remote, branch)
     │
     ▼
Auth.getGitCredential(repoUrl)
     │
     ├── SSH Key 认证
     │   └── 设置 GIT_SSH_COMMAND 环境变量：
     │       GIT_SSH_COMMAND="ssh -i /path/to/key -o StrictHostKeyChecking=no"
     │
     └── Token 认证
         └── 通过 git credential helper 注入：
             git -c credential.helper='!f() { echo "username=token"; echo "password=<token>"; }; f'
```

**实现要点：**
- 凭证获取失败时，操作返回明确错误而非暴露认证细节
- SSH Key 路径由 Auth 模块统一管理，Git 模块不直接存储
- Token 认证仅在内存中临时使用，不写入 `.gitconfig`

---

## 七、Git 状态检测机制

Git 模块依赖 File 模块的 watcher 监听 `.git` 目录变更，实现状态自动刷新。

```
File Watcher 监听 .git/ 目录
     │
     ├── HEAD 变更（分支切换、commit）
     ├── index 变更（stage/unstage）
     ├── refs/ 变更（远程更新）
     │
     ▼
触发 git.status(repoPath) 重新查询
     │
     ▼
发出 git.status.changed 事件
```

**实现要点：**
- debounce 间隔 300ms，避免频繁触发
- 仅在有订阅者时激活 watcher，无监听时自动停止
- 合并 .git 目录内的多个文件变更事件为一次 status 刷新

---

## 八、跨平台路径标准化

IPC 传输层统一使用正斜杠（`/`）路径格式，后端根据宿主平台转换为本地路径。

```
IPC 传输（正斜杠）
"C:/Users/dev/project/src/main.rs"
     │
     ▼
后端路径转换
     ├── Windows → "C:\Users\dev\project\src\main.rs"
     └── macOS/Linux → "/Users/dev/project/src/main.rs"
```

**实现要点：**
- 前端发送路径时统一转为正斜杠格式
- 后端接收后立即根据 `std::path::Path` 转换为平台原生路径
- Git 命令参数中的路径由后端统一处理，前端无需感知

---

## 九、测试策略

```
单元测试：状态解析、差异格式化、分支管理
集成测试：提交/推送/拉取流程、冲突处理、凭证管理
```
