# 10 - Project / Worktree 项目与工作树管理 详细设计

> 模块编号：10 | 层级：能力层
> 依赖：01-Logger, 02-Event+IPC, 03-Config
> 被依赖：08-Session, 16-Agent, 09-File, 11-Terminal, 14-LSP, 21-Git
>
> 对标：Claude Code 的 project / worktree / session 主轴

---

## 一、模块概述

### 1.1 定位

Project 是轻量级的上下文容器，负责项目身份、自定义提示（navis.md）、知识文件、最近项目和项目信任状态。

Worktree 是当前 Session 绑定的真实工作目录或 Git checkout，负责文件、终端、Git、LSP 和 Sandbox 的路径边界。一个 Project 可以有多个 Worktree；一个 Session 在任意时刻只绑定一个 Worktree。

### 1.2 设计哲学

```
Claude Code 的做法：
  Project = 项目身份和指令
  Worktree = 当前工作目录 / git checkout
  Session = 对话和执行事实

Navis Go 对齐：
  Project = navis.md（自定义指令）+ 知识路径 + 信任状态 + Session 列表
  Worktree = worktreeRoot + git 信息 + 最近目录 + 路径边界
  Session = 绑定 projectId + worktreeRoot，保存消息、Timeline 和任务事实
```

### 1.3 职责边界

```
负责：
├── 项目发现（从 CWD 向上查找 navis.md）
├── 项目配置加载（navis.md 解析）
├── 项目知识文件管理（rag_sources 路径列表）
├── 会话列表管理（1 Project : N Session）
├── Worktree 发现和绑定（当前真实工作目录 / Git checkout）
├── 最近 Worktree 列表
├── 最近项目列表
└── 项目信任状态

不负责：
├── 全局搜索 → 各模块自行提供（File/Grep MCP 工具）
├── 技术栈自动识别 → 用户在 navis.md 中手动声明
├── 七模块联动 → 各模块自行监听配置变更事件
├── LSP 管理 → LSP 模块
├── Terminal 管理 → Terminal 模块
└── 安全策略 → Sandbox 模块
```

---

## 二、架构设计

```
project/
├── mod.rs              # 模块入口
├── discovery.rs        # 项目发现（从 CWD 向上查找 navis.md）
├── worktree.rs         # Worktree 发现、绑定、最近目录和 Git checkout 信息
├── config.rs           # navis.md 解析
├── knowledge.rs        # 知识文件路径管理
└── recent.rs           # 最近项目列表
```

> 没有 search.rs、metadata.rs、trust.rs —— 全局搜索交给 MCP 工具，元数据由用户声明，信任管理复用 Sandbox 模块。

---

## 三、数据模型

```rust
/// 项目实体（轻量级，不持有运行时状态）
struct Project {
    id: String,                         // 项目唯一标识（基于路径哈希）
    root: PathBuf,                      // 项目根目录（含 .git 或 navis.md 的最近目录）
    name: String,                       // 项目名称（来自 navis.md 或目录名）
    config: Option<NavisConfig>,      // navis.md 解析结果
    knowledge_paths: Vec<PathBuf>,      // 知识文件路径列表（来自 navis.md rag_sources）
    session_ids: Vec<String>,           // 关联的会话 ID 列表
    trust: ProjectTrust,                // 信任状态
    last_opened: DateTime<Utc>,         // 最后打开时间
}

/// Worktree 实体：Session 的真实工作目录边界。
struct Worktree {
    id: String,                         // 基于路径和 git checkout 派生的稳定 ID
    project_id: String,                 // 归属 Project
    root: PathBuf,                      // 当前工作目录 / Git worktree 根路径
    display_name: String,               // UI 显示名，默认目录名
    git_repository_root: Option<PathBuf>,
    git_branch: Option<String>,
    last_opened: DateTime<Utc>,
}

/// 项目信任状态（Sandbox 按 Project / Worktree 根路径评估）
enum ProjectTrust {
    Trusted,            // 完全信任
    Untrusted,          // 不信任（限制权限）
    AskEachTime,        // 每次询问（默认）
    SessionScoped,      // 仅本次会话信任
}

/// navis.md 配置（对标 Claude Desktop Custom Instructions）
struct NavisConfig {
    // === 项目信息 ===
    project_name: Option<String>,       // 项目名称
    description: Option<String>,        // 项目描述（注入 Agent system prompt）

    // === Agent 行为配置（对标 Claude Desktop Custom Instructions）===
    instructions: Option<String>,       // 自定义指令（Markdown 正文，注入 system prompt）
    default_model: Option<String>,      // 默认模型
    temperature: Option<f32>,           // 推理温度
    default_role: Option<String>,       // 默认角色（引用 RoleDefinition.id）

    // === 知识文件（对标 Claude Desktop Knowledge Files）===
    rag_sources: Vec<String>,           // 知识路径列表（目录或文件）

    // === 工具权限 ===
    tool_permissions: Option<ToolPermissions>,

    // === 排除模式 ===
    exclude_patterns: Vec<String>,      // 排除的文件/目录模式
}

/// 工具权限配置
struct ToolPermissions {
    allow: Vec<String>,                 // 允许列表
    deny: Vec<String>,                  // 拒绝列表
}
```

---

## 四、接口定义

```typescript
// === 项目发现 ===
// 从指定路径向上查找 navis.md，返回最近的项目根目录
project.discover(fromPath: string): Promise<Project | null>

// === 项目打开 ===
// 打开一个项目目录（自动发现 navis.md，加载配置）
project.open(path: string): Promise<Project>

// === 当前项目 ===
project.getCurrent(): Promise<Project | null>

// === 项目切换 ===
// 切换到另一个项目（触发 project.switched 事件）
project.switchTo(targetPath: string): Promise<void>

// === 最近目录（当前 UI IPC 实现） ===
ui_list_recent_worktrees(payload: { limit?: number }): Promise<Worktree[]>
ui_record_recent_worktree(payload: { path: string; limit?: number }): Promise<Worktree[]>
ui_remove_recent_worktree(payload: { path: string; limit?: number }): Promise<Worktree[]>

// === Worktree ===
project.discoverWorktree(path: string): Promise<Worktree>
project.listRecentWorktrees(projectId?: string, limit?: number): Promise<Worktree[]>
project.recordRecentWorktree(payload: { path: string; projectId?: string; limit?: number }): Promise<Worktree[]>

// === 项目配置 ===
project.getConfig(): Promise<NavisConfig | null>
project.reloadConfig(): Promise<NavisConfig | null>  // 重新加载 navis.md

// === 知识文件 ===
// 返回当前项目的知识文件路径列表（供 Context Manager / RAG 使用）
project.getKnowledgePaths(): Promise<string[]>

// === 会话列表 ===
project.getSessions(): Promise<Session[]>
```

当前 UI 映射：

- `composer.workspace` 的当前目录按钮和文件夹 `+` 打开最近目录菜单，前端通过 `ui_list_recent_worktrees` 读取后端 `ProjectManager` 中最近绑定过的 Worktree 目录，界面只展示最近 10 个。
- `选择新的工作目录` 通过系统目录选择器拿到目录后，先解析 Project / Worktree，再写入当前 `Session.worktree_root`，并把目录放到最近 Worktree 列表首位。
- 最近目录持久化在用户配置 `project.recentWorktrees`，运行时由 `ProjectManager` 复用 `RecentWorktreesManager` 维护；该入口不强制执行完整 `project.open` 发现流程，因此普通目录也可以作为会话 Worktree 进入最近列表。

---

## 五、项目发现机制

从 CWD 向上遍历，查找项目根目录。对标 Claude Code 的 `.claude/` 发现机制和 Codex 的 `.git` 发现机制。

```
从指定路径开始
    │
    ├── 检查当前目录是否存在 navis.md → 是 → 确认为项目根
    ├── 检查当前目录是否存在 .navis/ → 是 → 确认为项目根
    ├── 检查当前目录是否存在 .git/ → 是 → 确认为项目根
    │
    └── 不存在 → 向上一级目录，重复检查
        │
        └── 到达文件系统根目录 → 返回 null（未找到项目）
```

**优先级：** `navis.md` > `.navis/` > `.git/`

### 5.1 子目录级 navis.md（对标 Claude Code subdir/CLAUDE.md）

```
my-project/
├── navis.md                  # 项目级配置（全项目生效）
├── src/
│   ├── navis.md              # src 目录配置（覆盖项目级，仅 src 及其子树生效）
│   └── api/
│       └── navis.md          # api 目录配置（最高优先级，仅 api 及其子树生效）
└── tests/
    └── navis.md              # 测试目录配置（覆盖项目级，仅 tests 及其子树生效）
```

**合并规则：**
- 子目录 navis.md **覆盖**项目级 navis.md 的同名字段
- 未覆盖的字段从项目级继承
- 更深层级的 navis.md 优先级更高（最具体 > 最宽泛）
- `instructions` 字段：子目录追加到项目级之后（不覆盖，而是补充）

---

## 六、navis.md 格式

```markdown
# 项目名称

## 项目描述
这是一个 React + TypeScript 的电商前端项目。

## 自定义指令
- 遵循 Airbnb 代码规范
- 使用 ESLint + Prettier 格式化
- 测试框架：Jest，新代码必须有测试
- commit message 使用 Conventional Commits 格式

## 模型配置
- 默认模型：claude-sonnet-4-6
- temperature: 0.3

## 知识文件
- ./docs
- ./src/types
- ./ARCHITECTURE.md

## 排除模式
- node_modules
- .git
- dist
- coverage
```

**与 Claude Desktop CLAUDE.md 的对比：**

| 维度 | CLAUDE.md | navis.md |
|------|-----------|------------|
| 格式 | 纯 Markdown 自由文本 | 结构化 Markdown（有标题分节） |
| 自定义指令 | 整个文件内容 | `## 自定义指令` 部分 |
| 知识文件 | 通过 UI 上传 | `## 知识文件` 路径列表 |
| 模型配置 | 不支持（固定 Claude） | `## 模型配置`（支持多模型） |
| 工具权限 | `.claude/settings.json` | `## 工具权限`（可选） |
| 子目录支持 | ✅ subdir/CLAUDE.md | ✅ subdir/navis.md |

---

## 七、事件定义

```typescript
type ProjectEvents = {
  // 项目生命周期
  'project.opened':          { projectId: string; rootPath: string }
  'project.switched':        { fromId: string; toId: string; fromPath: string; toPath: string }
  'project.closed':          { projectId: string }

  // 配置变更
  'project.config.loaded':   { projectId: string; config: NavisConfig }
  'project.config.reloaded': { projectId: string; config: NavisConfig }
  'project.config.error':     { projectId: string; error: string }

  // 信任状态
  'project.trust.changed':   { projectId: string; trust: ProjectTrust }
}
```

> **重要：** 其他模块（LSP、Terminal、Agent 等）**自行监听** `project.switched` 和 `project.config.loaded` 事件来响应项目切换，Project 模块不负责联动协调。每个模块自己决定在项目切换时做什么。

---

## 八、与其他模块的关系

Project / Worktree 模块是**被动的上下文提供者**，不是主动的联动协调者。

```
Project 模块的职责：
  1. 发现项目 → 加载 navis.md → 发出事件
  2. 发现 Worktree → 保存当前会话工作目录边界
  3. 提供配置查询接口（getConfig / getKnowledgePaths）
  4. 管理会话列表

其他模块自行响应：
  ├── Session：监听 project.opened / worktree.bound，自动创建/切换到项目的会话
  ├── Agent：监听 project.config.loaded，更新模型/角色/指令
  ├── Sandbox：监听 project.trust.changed，更新权限
  ├── LSP：监听 project.switched，重启语言服务器
  ├── Terminal：监听 worktree.bound，切换工作目录
  └── Context Manager：调用 project.getKnowledgePaths() 获取知识路径
```

设计约束：

- Project 只保存身份、配置、知识和信任状态，不直接执行文件、终端、Git 或 LSP 行为。
- Worktree 只保存真实目录边界和 Git checkout 信息，不保存对话历史。
- Session 是消息、Turn Timeline、任务摘要和恢复事实源；跨 Worktree 切换必须写回 Session，而不是只改前端临时状态。
- 文件搜索、全文搜索和符号搜索由 File / MCP / LSP 能力提供，Project 不维护全局搜索索引。

---

## 九、配置优先级

```
子目录 navis.md（subdir/navis.md）          ← 最高优先级
  > 项目 navis.md（<project>/navis.md）
    > 用户全局配置（~/.navis/config.toml）
      > 系统默认（内置）                            ← 最低优先级
```

---

## 十、测试策略

```
单元测试：navis.md 解析、项目发现（向上遍历）、子目录配置合并、最近项目管理
集成测试：项目打开/切换/关闭流程、配置热重载、事件发布
```
