# 19 - Skills 技能管理 详细设计

> 模块编号：19 | 层级：AI 核心层
> 依赖：01-Logger, 02-IPC, 03-Config, 09-File, 22-UI-Framework（Slash TriggerIndex 投影）, kernel::EventBus
> 被依赖：16-Agent

---

## 一、模块概述

### 1.1 定位

Skills 是 Agent 的技能配置层，管理 SKILL.md 文件的加载、解析、校验、启停和激活计划生成，支持标准模式和增强流程。同时管理轻量命令（Commands），提供纯 Markdown 提示词模板的快速复用能力。

Skills 不是 Tool，不进入 Tool Projection，也不作为可执行能力注册到 Kernel Registry。它只提供提示词、参数、工具白名单、步骤建议和角色绑定；真正的工具发现、权限判断、执行链和事件写出，仍由 Agent / Tool Runtime 通过 Kernel Registry / Policy / Pipeline / EventBus 完成。

### 1.2 三层边界

```
MCP      = 原子工具（read / bash）
Skills   = 规则模板（提示词 + 参数 + 工具白名单 + 步骤建议）
Commands = 轻量命令（纯提示词模板，无元数据头）
Agent    = 流程决策（调用哪些工具、什么顺序）

禁止：Skills 内部直接执行工具
禁止：Skills 自建能力 Registry / Pipeline / EventBus / Policy
```

### 1.3 两种 Skills 模式 + 一种轻量命令

> Commands 不是 Skill 的子类型，而是一种独立的轻量机制。

```
标准模式（Skill）：纯 SKILL.md，遵循通用 Skill 文档格式
├── 人设设定 + 行为规范 + 工具白名单 + 示例
├── 导入即用
└── 触发：/skill-name 或 Agent 自动匹配

增强模式（Skill）：支持步骤/工作流/状态机
├── 步骤定义、条件分支、循环
├── 团队固化流程
└── 激活后生成 Agent Pipeline 的步骤计划，由 Agent 按步骤调度

轻量命令（Command，非 Skill 子类型）：纯 Markdown 提示词模板（对标 Claude Code .claude/commands/*.md）
├── 无 YAML 元数据头，文件内容即提示词
├── 支持 $ARGUMENTS 占位符，运行时由用户输入替换
├── 存放位置：.navis/commands/（项目级）或 ~/.navis/commands/（用户级）
└── 触发：/command-name
```

---

## 二、架构设计

```
skills/
├── mod.rs              # 模块入口
├── loader.rs           # Skill 文件加载器
├── parser.rs           # SKILL.md 解析器
├── activation.rs       # 生成 Agent 激活计划，不执行工具
├── role.rs             # 角色模板管理
├── validator.rs        # Skill 格式校验
├── store.rs            # SkillStore：领域提示词包索引，不是 Kernel Registry
├── commands.rs         # 轻量命令加载与管理
└── builtin/            # 内置 Skills
    ├── commit.md
    ├── review.md
    ├── explain.md
    └── refactor.md
```

### 轻量命令目录结构

```
.navis/
└── commands/
    ├── explain.md          # 项目级命令示例
    ├── test.md
    └── deploy-check.md

~/.navis/
└── commands/
    └── summarize.md        # 用户级命令示例
```

轻量命令是纯 Markdown 文件，文件名即命令名（去掉 `.md` 后缀），文件内容即提示词模板。

### 2.1 与 Kernel 四原语的边界

| 事项 | Skills 负责 | Kernel / 宿主负责 |
|------|-------------|-------------------|
| Skill/Command 文件 | 加载、解析、校验、启停、来源记录 | 不进入 Kernel |
| Skill 查询 | `SkillStore` 按 id/trigger/source 查询领域状态 | 不作为能力 Registry |
| Skill 激活 | 生成 `SkillActivationPlan`，注入提示词、参数和工具白名单 | Agent Pipeline 消费计划并推进 turn |
| 工具白名单 | 声明允许使用哪些工具或通配符 | Kernel Registry 查找真实工具；Policy 裁剪可用集合 |
| 增强流程步骤 | 声明步骤、依赖、失败策略和提示片段 | Agent Pipeline 逐步执行，Tool Runtime 执行真实工具 |
| `/` 入口 | 输出候选项 DTO | UI TriggerIndex 只做前端投影和搜索交互 |
| 事件 | 通过 Kernel EventBus 发布加载/启停/激活通知 | EventBus 是唯一事件总线；AgentTimelinePart 是执行事实 |

`SkillStore` 是业务状态库，类似配置索引；它不能提供可执行 trait、运行时 handler 或工具调用入口。任何看起来像“执行 Skill”的 API，实际都必须返回 Agent 可消费的计划或上下文补丁。

---

## 三、数据模型

```rust
struct SkillDefinition {
    id: String,
    name: String,
    description: String,
    mode: SkillMode,             // standard / enhanced
    version: String,
    source: SkillSource,         // builtin / user / project / extension
    file_path: PathBuf,
    trigger: Option<String>,     // 触发命令（如 "/commit"）
    tools_whitelist: Vec<String>,// 允许使用的工具
    parameters: Vec<SkillParameter>,
    content: String,             // 提示词内容
    enabled: bool,
}

enum SkillMode {
    Standard,    // 纯提示词模板
    Enhanced,    // 支持步骤/工作流
}

enum SkillSource {
    Builtin,
    User,        // ~/.navis/skills/
    Project,     // .navis/skills/
    Extension,
}

struct SkillParameter {
    name: String,
    description: String,
    required: bool,
    default: Option<String>,
    param_type: String,          // string / number / boolean
}

// 增强模式步骤
struct SkillStep {
    name: String,
    description: String,
    prompt: String,
    tools: Vec<String>,
    depends_on: Vec<String>,     // 依赖的步骤
    condition: Option<String>,   // 条件表达式
    on_failure: OnFailureAction, // 失败处理策略
    max_retries: u32,            // 最大重试次数（配合 on_failure = Retry）
    timeout: Option<Duration>,   // 步骤超时时间
}

enum OnFailureAction {
    Retry,   // 重试（受 max_retries 限制）
    Fail,    // 终止整个 Skill 执行
    Skip,    // 跳过当前步骤，继续下一步
}

// 增强模式步骤执行状态
enum StepStatus {
    Pending,     // 等待执行
    Running,     // 正在执行
    Completed,   // 执行完成
    Failed,      // 执行失败
    Skipped,     // 被跳过（条件不满足或 on_failure = Skip）
}

// 角色模板
struct RoleDefinition {
    id: String,
    name: String,
    description: String,
    system_prompt: String,
    guidance: Option<String>,        // 角色行为指导（Task Sidechain 派发时自动注入，对标 Codex worker/explorer 角色指导）
    skills: Vec<String>,             // 绑定的 Skills
    commands: Vec<String>,           // 绑定的轻量命令名
    model_preference: Option<String>,
    temperature: Option<f32>,
}

// 轻量命令模板（区别于 SkillDefinition）
struct CommandTemplate {
    name: String,                // 命令名（文件名去 .md）
    file_path: PathBuf,          // .md 文件路径
    source: CommandSource,       // 项目级 / 用户级
    content: String,             // 原始 Markdown 内容（即提示词模板）
    has_arguments: bool,         // 是否包含 $ARGUMENTS 占位符
}

enum CommandSource {
    Project,     // .navis/commands/
    User,        // ~/.navis/commands/
}

// Skill 激活计划：传给 Agent，不直接执行
struct SkillActivationPlan {
    skill_id: String,
    prompt_patch: String,
    parameters: HashMap<String, String>,
    tools_allowlist: Vec<String>,
    steps: Vec<SkillStep>,
    role_overlay: Option<RoleDefinition>,
}

// "/" 输入候选投影：传给 UI，不是注册表 entry
struct SkillTriggerCandidate {
    name: String,
    candidate_type: SkillCandidateType,
    source: SkillSource,
    description: String,
    trigger: String,
    extension_id: Option<String>,
}

enum SkillCandidateType {
    Command,
    Skill,
    EnhancedSkill,
}
```

### 3.1 角色行为指导（guidance）

`RoleDefinition.guidance` 是角色的行为指导文本，在 Task Sidechain 派发时由 Context Manager 自动注入 sidechain session 的 system prompt，保障子任务行为一致性。

**对标竞品**：Codex CLI 的 worker/explorer 角色 TOML 配置中包含行为指导，框架自动注入。

**内置角色 guidance 示例：**

```markdown
# developer 角色 guidance

你是一个独立工作的 Task Sidechain 开发执行者。
- 你不是代码库中唯一的工作者，不要撤销其他人的编辑
- 明确分配文件所有权：只修改与你任务直接相关的文件
- 修改代码前先阅读现有实现，理解上下文
- 遵循项目的代码规范（参见 project_summary 中的 code_standards）
- 每次修改后运行相关测试验证
```

```markdown
# technical-writer 角色 guidance

你是一个独立工作的 Task Sidechain 文档执行者。
- 引用代码时附带文件路径和行号
- 文档格式统一使用 Markdown
- 代码示例必须可运行，不要编造不存在的 API
- 优先更新现有文档，而非创建新文件
```

**用户自定义角色：**

`navis_{mode}.md` 中的 `## 角色` 字段仅引用 `RoleDefinition.id`（如 `developer`），不直接定义角色内容。如需自定义角色，通过 `roles.update()` 接口修改 RoleDefinition。

```

---

## 四、接口定义

### 4.1 IPC 命令

```typescript
// Skill 管理
skills.list(filter?: { mode?: string; source?: string; enabled?: boolean }): Promise<SkillDefinition[]>
skills.get(id: string): Promise<SkillDefinition | null>
skills.enable(id: string): Promise<void>
skills.disable(id: string): Promise<void>
skills.install(path: string): Promise<void>
skills.uninstall(id: string): Promise<void>
skills.validate(path: string): Promise<{ valid: boolean; errors: string[] }>

// 角色管理
roles.list(): Promise<RoleDefinition[]>
roles.get(id: string): Promise<RoleDefinition | null>
roles.create(role: RoleDefinition): Promise<void>
roles.update(id: string, updates: Partial<RoleDefinition>): Promise<void>
roles.delete(id: string): Promise<void>

// 轻量命令管理
commands.list(filter?: { source?: string }): Promise<CommandTemplate[]>
commands.get(name: string): Promise<CommandTemplate | null>
commands.create(name: string, content: string): Promise<void>
commands.delete(name: string): Promise<void>
commands.enable(name: string): Promise<void>
commands.disable(name: string): Promise<void>
```

### 4.2 Rust API

```rust
Skills::with_event_bus(config: Arc<Mutex<Config>>, event_bus: Arc<dyn EventBus>) -> Result<Skills>
Skills::load_all(&mut self) -> Result<()>   // 加载并写入 SkillStore（格式不合格的记录警告但不阻断其他文件）
Skills::get(&self, id: &str) -> Option<&SkillDefinition>
Skills::find_by_trigger(&self, trigger: &str) -> Option<&SkillDefinition>
Skills::get_context(&self, skill_id: &str, params: HashMap<String, String>) -> Result<String>
Skills::activate(&self, skill_id: &str, params: HashMap<String, String>) -> Result<SkillActivationPlan>  // 生成 Agent 激活计划，不执行工具
Skills::list_trigger_candidates(&self) -> Vec<SkillTriggerCandidate>  // "/" UI 投影数据
Skills::list_roles(&self) -> Vec<RoleDefinition>
Skills::get_role(&self, id: &str) -> Option<&RoleDefinition>
Skills::list_commands(&self) -> Vec<&CommandTemplate>
Skills::get_command(&self, name: &str) -> Option<&CommandTemplate>
Skills::load_commands(&mut self) -> Result<()>
```

应用运行时只托管一份共享 `Arc<Mutex<Skills>>` 状态。Slash / Command Palette 候选投影读取这份状态；Agent Tool Pipeline 的 `SkillMatchStage` 也读取同一份状态来生成 `SkillActivationPlan`。这两个入口都不能把 Skill 当作工具执行：Slash 只是前端候选，Pipeline 也只产出提示词补丁、步骤建议和工具白名单，后续真实工具调用必须继续经过 Tool Projection、Kernel Policy、Agent Tool Pipeline 和 MCP / builtin executor。

---

## 五、SKILL.md 格式

```markdown
---
name: code-review
description: 代码审查
mode: standard
trigger: /review
tools: [read, lsp.diagnostics, lsp.references]
parameters:
  - name: focus
    description: 审查重点
    required: false
    default: "all"
---

你是一个资深代码审查员。

## 行为规范
- 审查代码时关注：安全性、性能、可读性、可维护性
- 使用 LSP 工具获取诊断信息和引用关系
- 输出格式：问题列表 + 严重程度 + 修复建议

## 示例
输入：审查 src/auth.ts
输出：
1. [HIGH] 第42行：密码明文存储，建议使用 bcrypt
2. [MED] 第58行：未处理的 Promise rejection
```

---

## 5.1 轻量命令格式

轻量命令是纯 Markdown 文件，不包含 YAML 元数据头（`---` 分隔的 frontmatter）。文件内容直接作为提示词模板，文件名去掉 `.md` 后缀即为命令名。

### 示例：.navis/commands/explain.md

```markdown
请详细解释以下代码或概念：

$ARGUMENTS

要求：
1. 用通俗易懂的语言解释
2. 给出关键代码片段的逐行说明
3. 指出可能的陷阱或注意事项
```

### 触发与参数替换

```
用户输入：/explain 这段 async/await 代码的作用
     ↓
匹配命令：commands/explain.md
     ↓
替换占位符：$ARGUMENTS → "这段 async/await 代码的作用"
     ↓
注入 Agent 上下文
```

---

## 5.2 轻量命令与 Skills 的区别

| 维度 | 轻量命令 (Command) | Skills (SkillDefinition) |
|------|-------------------|--------------------------|
| 文件格式 | 纯 Markdown，无元数据头 | YAML frontmatter + Markdown 正文 |
| 存放目录 | `.navis/commands/` 或 `~/.navis/commands/` | `.navis/skills/` 或 `~/.navis/skills/` |
| 能力声明 | 不支持（无工具白名单、无参数定义） | 支持（tools、parameters、mode 等） |
| 执行模式 | 仅提示词注入，由 Agent 自行决策 | 生成 Agent 激活计划；增强模式附带步骤计划 |
| 参数机制 | `$ARGUMENTS` 占位符，运行时字符串替换 | 命名参数，支持类型、默认值、必填校验 |
| 适用场景 | 简单的提示词复用（解释、翻译、格式化） | 团队标准化流程（审查、重构、发布） |
| 复杂度 | 低（一个 Markdown 文件） | 中高（元数据 + 工具约束 + 步骤编排） |
| 对标 | Claude Code `.claude/commands/*.md` | Claude Code `/skill` + SKILL.md |

设计原则：优先使用轻量命令满足简单需求；当需要工具白名单、参数校验、步骤编排时，升级为 Skill。

### 5.3 Commands 安全机制

轻量命令文件在加载时需进行安全校验，防止恶意或危险内容被误执行：

```
加载流程：
1. 读取 .md 文件内容
2. 扫描是否包含危险指令模式（正则匹配）
   ├── 删除所有文件 / rm -rf / format
   ├── 修改系统配置
   ├── 执行任意代码（eval / exec）
   └── 其他自定义危险模式（可配置）
3. 如匹配 → 标记该命令为"需审核"（needs_review = true）
4. 需审核的命令不出现在命令面板中，需管理员确认后方可启用
```

### 5.4 Commands 与 Skills 触发冲突处理

当用户输入 `/name` 时，匹配优先级如下：

```
/name 输入
  ├─ 1. 优先匹配 Commands（轻量命令）
  │     └─ 匹配到 → 使用该 Command
  └─ 2. 无 Command 匹配时 → 匹配 Skills
        └─ 匹配到 → 使用该 Skill
```

设计理由：Commands 作为高频轻量操作，优先匹配可减少误触发重型 Skill 的概率。

---

## 5.5 输入框 `/` 触发器集成

Skills 和 Commands 加载完成后，会生成 Chat 输入框 `/` 内置触发器的候选项投影。`/` 触发器属于 UI Framework 的 TriggerIndex，不是 Skills 自己的注册表，也不是 Kernel Registry。

### 注册流程

```
Skills Loader 加载完成
     │
     ├── 1. 扫描 Commands（.navis/commands/ + ~/.navis/commands/）
     │      └── 写入 CommandStore，并生成 "/" 候选投影
     │
     ├── 2. 扫描 Skills（.navis/skills/ + ~/.navis/skills/）
     │      └── 校验后写入 SkillStore，并投影到 "/" 数据源
     │
     ├── 3. 扩展启用时登记 contributes.skills
     │      └── ExtensionLifecycle 调用共享 Skills 状态逐个 upsert/remove
     │
     └── 4. 通过 Kernel EventBus 发出 skill.loaded / command.loaded 事件（含来源统计）
```

当前实现中，Slash commands 的后端候选集来自共享 `Skills` 状态：启动时 `Skills::load_all()` 负责内置 / 用户 / 项目级 Skills 与轻量命令，扩展启用/禁用时再由 `ExtensionLifecycle` 把 `contributes.skills` 写入或移出同一状态源。前端只消费候选投影，不单独扫描扩展 skill 清单。

### GitHub 下载的 Skill 自动生效

用户从 GitHub 下载 SKILL.md 放入 `.navis/skills/` 目录后：

```
1. File 模块检测到新文件 → 发出 file.changed 事件
2. Skills Loader 监听 file.changed → 解析 SKILL.md
3. 校验通过 → 写入 SkillStore
4. 自动出现在输入框 "/" 候选投影
5. 用户在输入框输入 "/" 即可搜索到该 Skill
```

**无需重启、无需手动注册、放入即用。**

### 候选项数据格式

投影到 `/` 触发器时，每个候选项的数据结构：

```typescript
interface SkillTriggerCandidate {
    name: string           // Skill/Command 名称（如 "review"）
    type: 'command' | 'skill' | 'enhanced'
    source: 'builtin' | 'user' | 'project' | 'extension'
    source_label: string   // 来源显示文本（如 "[项目]" "[扩展:github]"）
    description: string    // 描述（Skill.description 或 Command 文件首行）
    trigger: string        // 完整触发路径（如 "/review"）
    extension_id?: string     // 来源扩展 ID（source 为 extension 时）
}
```

---

## 六、事件定义

Skill 事件通过 Kernel EventBus 发布，只表达加载、启停和激活这类离散事实。增强 Skill 的步骤执行进度不由 Skills 模块自建事件链；它进入 Agent Pipeline 后，以 AgentTimelinePart / action / reasoning 等既有事件和 Stream payload 呈现。

```typescript
type SkillEvents = {
  'skill.loaded':        { skillId: string; name: string; source: string }
  'skill.unloaded':      { skillId: string }
  'skill.enabled':       { skillId: string }
  'skill.disabled':      { skillId: string }
  'skill.activated':     { sessionId: string; skillId: string; skillName: string }
  'skill.completed':     { sessionId: string; skillId: string; duration: number }
  'skill.failed':        { sessionId: string; skillId: string; error: string }
  'role.loaded':         { roleId: string }
  'role.activated':      { roleId: string; roleName: string }
  'role.changed':        { roleId: string; changeType: string }
  'command.loaded':      { count: number; sources: string[] }
  'command.activated':   { sessionId: string; commandName: string }
  'command.created':     { name: string; source: string }
  'command.modified':    { name: string; source: string }
  'command.deleted':     { name: string; source: string }
}
```

---

## 七、测试策略

```
单元测试：SKILL.md 解析、参数替换、工具白名单校验、SkillActivationPlan 生成
集成测试：Skill 加载/卸载、角色绑定、Agent Pipeline 消费 SkillActivationPlan
命令测试：轻量命令加载、$ARGUMENTS 占位符替换、命令名解析
```
