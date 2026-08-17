# 06 - Sandbox 安全沙箱 详细设计

> 模块编号：06 | 层级：安全层
> 依赖：01-Logger, 02-Event+IPC, 03-Config, 05-Auth
> 被依赖：09-File, 11-Terminal, 12-MCP, 15-Code-Edit

---

## 一、模块概述

### 1.1 定位

Sandbox 是安全管控中心，所有文件/终端/网络操作必须经过 Sandbox 校验，提供访问控制、命令黑白名单、项目信任、资源限制、审计能力。

### 1.2 职责边界

```
负责：
├── 路径访问控制（白名单/黑名单）
├── 命令规则引擎（黑白名单、正则匹配、Shell 语义识别）
├── 项目信任机制（信任/不信任/仅本次）
├── 操作分级（Level 0-3）
├── 资源限制（CPU/内存/时间）
└── 操作审计（所有安全相关操作记录）

不负责：
├── 网络请求代理（只做域名校验）
├── 进程沙箱隔离（依赖操作系统能力）
└── 加密/解密 → Auth
```

---

## 二、架构设计

```
sandbox/
├── mod.rs              # 模块入口
├── gate.rs             # 权限门禁（统一入口）
├── access_control.rs   # 路径访问控制
├── command_rules.rs    # 命令规则引擎
├── project_trust.rs  # 项目信任
├── resource_limit.rs   # 资源限制
├── permission.rs       # 操作权限分级
├── audit_view.rs       # 近期审计视图缓存；结构化事实源走 Kernel Audit
└── policy.rs           # 策略配置
```

---

## 三、数据模型

```rust
// 操作分级
enum PermissionLevel {
    Unrestricted,   // Level 0: 无限制（只读、内存操作）
    LightCheck,     // Level 1: 轻量校验（文件读取、目录遍历）
    StrictCheck,    // Level 2: 严格校验（文件写入、命令执行）
    UserConfirm,    // Level 3: 用户确认（删除、系统命令、网络请求）
}

// 操作请求
struct OperationRequest {
    operation: OperationType,
    target: String,             // 文件路径 / 命令 / URL
    actor: String,              // "user" / "agent" / "extension:xxx"
    session_id: Option<String>,
    worktree_root: Option<String>, // 当前 Session Worktree 根路径；Sandbox 以此作为路径级信任目标
}

enum OperationType {
    FileRead,
    FileWrite,
    FileDelete,
    DirCreate,
    DirDelete,
    CommandExecute,
    NetworkRequest,
}

// 校验结果
struct CheckResult {
    allowed: bool,
    level: PermissionLevel,
    reason: Option<String>,     // 拒绝原因
    require_confirm: bool,      // 是否需要用户确认
    confirm_message: Option<String>, // 确认提示
}

// 命令规则
struct CommandRule {
    pattern: String,            // 正则表达式
    action: RuleAction,         // deny / confirm / allow
    description: String,
}

// Shell 语义
enum CommandShell {
    Bash,
    PowerShell,
    Cmd,
}

// 危险命令提示分类；提示用于强制确认，不等同于永久拒绝。
enum DangerousCommandCategory {
    DataLoss,
    RemoteHistoryRewrite,
    FileDeletion,
    DatabaseMutation,
    InfrastructureMutation,
    SafetyBypass,
    SystemDamage,
    CredentialOrConfigWrite,
}

enum RuleAction {
    Deny,       // 永久拒绝
    Confirm,    // 需要用户确认
    Allow,      // 允许
}

// 项目信任
enum ProjectTrust {
    Trusted,        // 完全信任
    Untrusted,      // 不信任（受限访问）
    AskEachTime,    // 每次询问
    SessionScoped,  // 仅本次会话信任（不持久化，会话结束自动清除）
}

// 注：ProjectTrust 是 Sandbox 内部沿用的路径级信任类型名；公开 IPC 和事件统一使用 worktreeRoot。

// 审批模式（三级审批，对标 Codex CLI）
enum ApprovalMode {
    Suggest,        // 建议模式：Agent 只建议，不执行，需用户确认所有操作
    AutoEdit,       // 自动编辑模式：Agent 可自动编辑文件，但执行命令需确认
    FullAuto,       // 完全自动模式：跳过普通确认，但仍受 ProjectTrust、权限白名单和危险操作 denylist 约束
}

UI 文案映射：

- `Bypass permissions` 是 `FullAuto` 的用户界面文案别名，不是第四种审批模式。
- 审计日志中的 `SandboxBypass` 表示 FullAuto 路径下自动放行的操作事件。
- IPC、配置和事件 payload 中的审批模式只接受规范小写 kebab-case：`suggest`、`auto-edit`、`full-auto`。不保留 `autoedit`、`auto_edit`、`fullauto`、`full_auto` 或大小写变体。

// ApprovalMode 与操作类型的联动规则
// ┌──────────────────────────┬─────────┬──────────┬─────────┐
// │ 操作类型                  │ Suggest │ AutoEdit │ FullAuto│
// ├──────────────────────────┼─────────┼──────────┼─────────┤
// │ FileRead                 │ 允许    │ 允许     │ 允许    │
// │ FileWrite                │ 确认    │ 允许     │ 允许    │
// │ FileDelete               │ 确认    │ 确认     │ 允许    │
// │ CommandExecute(只读)      │ 确认    │ 确认     │ 允许    │
// │ CommandExecute(写入)      │ 确认    │ 确认     │ 允许*   │
// │ NetworkRequest           │ 确认    │ 放行(黑名单拦截) │ 放行(黑名单拦截) │
// └──────────────────────────┴─────────┴──────────┴─────────┘
// 说明：
// * FullAuto 不弹普通确认框，但删除项目根、格式化磁盘、越权路径、未信任 Project / Worktree 等危险操作仍会被拒绝或强制确认。
// - "允许" 表示 Sandbox 直接放行，无需用户介入
// - "确认" 表示 Sandbox 返回 require_confirm=true，由 Dialog 弹出确认框
// - NetworkRequest 在 AutoEdit/FullAuto 模式下默认放行，仅拦截黑名单域名
// - Gateway 模型连接不经过 Sandbox 网络校验，由 Auth/Config 管控

// 网络策略（仅管控 Agent 工具调用产生的网络请求，不管控 Gateway 模型连接）
struct NetworkPolicy {
    blocked_domains: Vec<String>,       // 黑名单域名（从 sandbox.toml 加载，非硬编码）
    // 设计原则：默认放行，仅拦截已知恶意目标
    // Gateway 模型连接（Anthropic/OpenAI/Custom/扩展适配等）由 Auth/Config 管控，不经过 Sandbox
    // 加载来源：~/.navis/sandbox.toml（用户级）+ <project>/.navis/sandbox.toml（项目级），合并去重
}
```

---

## 四、接口定义

### 4.1 Rust API

```rust
// 统一校验入口
Sandbox::check(&self, request: &OperationRequest) -> Result<CheckResult>

// 路径校验
Sandbox::check_path(&self, path: &Path, operation: OperationType, worktree_root: &Path) -> CheckResult

// 命令校验
Sandbox::check_command(&self, command: &str, worktree_root: &Path) -> CheckResult
Sandbox::check_command_for_shell(&self, command: &str, worktree_root: &Path, shell: CommandShell) -> CheckResult

// 网络校验
Sandbox::check_network(&self, url: &str) -> CheckResult

// Project / Worktree 信任
Sandbox::get_trust(&self, worktree_root: &Path) -> ProjectTrust
Sandbox::set_trust(&self, worktree_root: &Path, trust: ProjectTrust) -> Result<()>

// 近期审计视图（非事实源）
Sandbox::get_recent_audit_view(&self, filter: SandboxAuditViewFilter) -> Vec<SandboxAuditViewEntry>

// 资源限制（resource_limit.rs）
Sandbox::set_resource_limit(&self, resource: ResourceType, limit: ResourceLimit) -> Result<()>
Sandbox::get_resource_usage(&self, resource: ResourceType) -> ResourceUsage
Sandbox::list_resource_limits(&self) -> Vec<(ResourceType, ResourceLimit, ResourceUsage)>

// 审批模式
Sandbox::get_approval_mode(&self) -> ApprovalMode
Sandbox::set_approval_mode(&self, mode: ApprovalMode) -> Result<()>

// 网络策略
Sandbox::get_network_policy(&self) -> NetworkPolicy
Sandbox::set_network_policy(&self, policy: NetworkPolicy) -> Result<()>
```

### 4.2 IPC 命令

```typescript
sandbox.checkTrust(worktreeRoot: string): Promise<ProjectTrust>
sandbox.setTrust(worktreeRoot: string, trust: 'trusted' | 'untrusted' | 'ask'): Promise<void>
sandbox.getRecentAuditView(filter?: SandboxAuditViewFilter): Promise<SandboxAuditViewEntry[]>  // 近期视图缓存；结构化事实源走 kernel::AuditRecorder/AuditSink
sandbox.getCommandRules(): Promise<CommandRule[]>
sandbox.setCommandRules(rules: CommandRule[]): Promise<void>
sandbox.getApprovalMode(): Promise<ApprovalMode>
sandbox.setApprovalMode(mode: 'suggest' | 'auto-edit' | 'full-auto'): Promise<void>
sandbox.getNetworkPolicy(): Promise<NetworkPolicy>
sandbox.setNetworkPolicy(policy: NetworkPolicy): Promise<void>
```

---

## 五、默认命令规则

命令校验不是单纯字符串黑白名单。Navis Go 会先匹配用户/项目配置的 `CommandRule`，然后执行内建危险命令 warning 检测，最后才进入 Shell 只读语义识别。

执行顺序：

```text
CommandRule deny 命中
  → 直接拒绝
CommandRule confirm/allow 命中
  → 暂存结果
DangerousCommandWarning 命中
  → 强制确认，覆盖 allow
无危险 warning 且有暂存规则
  → 返回规则结果
无规则命中
  → 按实际 shell 判断是否只读
只读
  → 自动放行
非只读或无法判断
  → 需要确认
```

实际执行路径必须调用 `check_command_for_shell`。例如 `Get-ChildItem` 在 PowerShell 是只读命令，但在 cmd 执行路径中不能因为 PowerShell 语义被自动放行。

只读语义首批覆盖：

- Bash：`ls/cat/head/tail/wc/grep/rg/find/stat/file/which/du/df/git status/diff/log/show` 等，拒绝重定向、命令替换、后台执行、`find -delete`、`sed -i`。
- PowerShell：`Get-ChildItem/Get-Content/Select-String/Test-Path/Get-Item/Get-Process/Get-Service` 等，拒绝管道写入和输出重定向。
- Cmd：`dir/type/findstr/find/where/whoami/ver/cd/echo/set` 等，拒绝 `del/copy/robocopy /mir` 等写入或破坏性命令。

危险 warning 首批覆盖：

- Git 数据丢失：`git reset --hard`、`git checkout/restore .`、`git stash drop/clear`。
- 远端历史改写：`git push --force/--force-with-lease/-f`。
- 文件删除：`rm -rf`、`del /s /q`、`Remove-Item -Recurse/-Force`、`git clean -f`、强制删分支。
- 数据库破坏：`DROP/TRUNCATE TABLE/DATABASE/SCHEMA`、无条件 `DELETE FROM table`。
- 基础设施变更：`kubectl delete`、`terraform destroy`、`docker stop/restart/kill/down`。
- 安全绕过：`--no-verify`、`git commit --amend`。
- 凭据/配置写入：写入 `.env/.ssh/.npmrc/.pypirc/.netrc` 等。

```toml
# 黑名单（永远禁止）
[[command_rules]]
pattern = "^rm\\s+-rf\\s+/$"
action = "deny"
description = "禁止删除根目录"

[[command_rules]]
pattern = "^sudo\\s+rm"
action = "deny"
description = "禁止 sudo 删除"

[[command_rules]]
pattern = ":\\(\\)\\{.*:\\|.*\\}"
action = "deny"
description = "禁止 fork bomb"

# 需要确认
[[command_rules]]
pattern = "^sudo\\s+"
action = "confirm"
description = "sudo 命令需要确认"

[[command_rules]]
pattern = "^git\\s+push"
action = "confirm"
description = "Git 推送需要确认"

[[command_rules]]
pattern = "^git\\s+reset\\s+--hard"
action = "confirm"
description = "Git 硬重置需要确认"

[[command_rules]]
pattern = "rm\\s+.*-r"
action = "confirm"
description = "递归删除需要确认"

# 白名单（无需确认）
[[command_rules]]
pattern = "^git\\s+(status|diff|log|branch)"
action = "allow"
description = "Git 只读操作"

[[command_rules]]
pattern = "^(npm|cargo|pnpm)\\s+(test|run|build)"
action = "allow"
description = "包管理器常用命令"
```

---

## 六、Project / Worktree 信任流程

```
打开 Project 或为 Session 绑定 Worktree
     │
     ▼
检查 Sandbox::get_trust()
     │
     ├── Trusted → 全部放行（Level 0-3 按规则）
     ├── Untrusted → 只允许 Level 0（只读）
     └── 未记录 → 弹出确认对话框
                    │
                    ├── [信任] → 记录 Trusted，全部放行
                    ├── [不信任] → 记录 Untrusted，只读
                    └── [仅本次] → 不记录，本次放行
```

---

## 七、网络策略

### 7.1 策略概述

Sandbox 的网络策略**仅管控 Agent 工具调用产生的网络请求**，不管控 Gateway 模型连接。

| 请求来源 | 管控方 | 说明 |
|---------|--------|------|
| Gateway 模型 API 连接 | Auth + Config | 用户在配置中显式指定的端点，不需要 Sandbox 介入 |
| Agent 工具调用（Terminal 执行 curl/npm install/git clone 等） | Sandbox | Agent 自主行为，需要安全管控 |
| MCP Server 网络请求 | Sandbox | 第三方工具，需要安全管控 |

**默认策略：放行，仅拦截已知恶意域名。**

设计原则：
- 默认信任用户——用户启动 Agent 就是授权 Agent 执行任务
- 仅在明确危险时拦截（已知恶意域名、挖矿池等）
- 用户可通过 Suggest 模式对每次网络请求逐一确认（如果需要更多控制）
- 不限制 localhost、不限制内网 IP——开发场景下访问本地服务是常态

### 7.2 审批模式与网络策略的关联

```
审批模式选择
     │
     ├── Suggest（建议模式）
     │     └── 网络请求 → 用户逐一确认（最严格的用户控制）
     │
     ├── AutoEdit（自动编辑模式）
     │     └── 网络请求 → 默认放行，仅拦截黑名单域名
     │
     └── FullAuto（完全自动模式）
           └── 网络请求 → 默认放行，仅拦截黑名单域名
```

### 7.3 网络校验流程

```
Agent 工具调用发起网络请求
     │
     ▼
判断请求来源
     │
     ├── Gateway 模型连接 → 不经过 Sandbox，直接放行
     │
     └── Agent 工具调用 → 进入 Sandbox 校验
           │
           ▼
         获取当前 ApprovalMode
           │
           ├── Suggest → 返回 require_confirm=true，等待用户确认
           │
           ├── AutoEdit / FullAuto
           │     │
           │     ▼
           │   检查 blocked_domains 黑名单
           │     │
           │     ├── 命中黑名单 → 拒绝，记录审计日志
           │     │
           │     └── 未命中 → 放行
           │
           └── 记录网络请求日志
```
     │     │     ▼
     │     │   解析请求 URL 域名
     │     │     │
     │     │     ├── 在 blocked_domains 中 → 拒绝
     │     │     ├── 在 allowed_domains 中 → 放行
     │     │     └── 不在任何列表中 → 拒绝
     │     │
     │     └── Open → 放行（记录审计日志）
     │
     └── 校验通过 → 发起网络请求
```

### 7.4 网络校验规则（嵌入 gate.rs）

```rust
impl Sandbox {
    /// 校验 Agent 工具调用产生的网络请求
    /// Gateway 模型连接不经过此方法，由 Auth/Config 直接放行
    fn check_network_with_policy(&self, url: &str) -> CheckResult {
        let mode = self.get_approval_mode();
        let policy = self.get_network_policy();
        let domain = extract_domain(url);

        // Suggest 模式：所有网络请求都需要用户确认
        if mode == ApprovalMode::Suggest {
            return CheckResult {
                allowed: true,
                level: PermissionLevel::UserConfirm,
                reason: None,
                require_confirm: true,
                confirm_message: Some(format!("Agent 请求访问网络: {}", url)),
            };
        }

        // AutoEdit / FullAuto 模式：默认放行，仅拦截黑名单
        if policy.blocked_domains.iter().any(|d| domain_matches(&domain, d)) {
            self.audit_blocked_network(&domain);
            return CheckResult {
                allowed: false,
                level: PermissionLevel::UserConfirm,
                reason: Some(format!("域名 {} 在黑名单中（已知恶意域名）", domain)),
                require_confirm: false,
                confirm_message: None,
            };
        }

        // 默认放行
        CheckResult {
            allowed: true,
            level: PermissionLevel::LightCheck,
            reason: None,
            require_confirm: false,
            confirm_message: None,
        }
    }
}
```

### 7.5 配置加载

网络黑名单从配置文件加载，不在代码中硬编码。

**配置文件位置：** `<project>/.navis/sandbox.toml`（项目级）或 `~/.navis/sandbox.toml`（用户级）

```toml
[network_policy]
# 黑名单域名（已知恶意域名）
# 命中的域名将被拦截，支持子域名匹配（"evil.com" 同时拦截 "sub.evil.com"）
blocked_domains = [
    "malicious.example.com",
    "cryptominer.pool.com",
]
```

**加载策略：**
```
Sandbox 初始化
    │
    ├── 读取 ~/.navis/sandbox.toml（用户全局配置）
    │     └── network_policy.blocked_domains
    │
    ├── 读取 <project>/.navis/sandbox.toml（项目级配置）
    │     └── network_policy.blocked_domains
    │
    ├── 合并：用户级 + 项目级黑名单去重合并
    │
    └── 无配置文件 → 默认空黑名单（不拦截任何域名）
```

**运行时更新：** 用户通过 UI 或 IPC 修改黑名单后，实时生效，无需重启。

---

## 八、事件定义

```typescript
type SandboxEvents = {
  'sandbox.check.allowed':    { operation: string; target: string; level: string; actor: 'user' | 'agent' | 'extension' }
  'sandbox.check.denied':     { operation: string; target: string; reason: string; actor: 'user' | 'agent' | 'extension' }
  'sandbox.check.confirm':    { operation: string; target: string; message: string; actor: 'user' | 'agent' | 'extension' }
  'sandbox.trust.changed':    { worktreeRoot: string; trust: ProjectTrust }
  'sandbox.approvalMode.changed': { mode: ApprovalMode; previousMode: ApprovalMode }
  'sandbox.resource.warning': { resource: string; usage: number; limit: number }
  'sandbox.resource.exceeded':{ resource: string; usage: number; limit: number }
  'sandbox.networkPolicy.changed': { policy: string; previousPolicy: string }  // 全局事件，无需 sessionId
  'sandbox.policy.changed':   { rules: Vec<CommandRule> }
}
```

Sandbox 校验入口会把允许、拒绝、确认等结构化审计事实写入 `kernel::AuditRecorder`，再同步一份到 `SandboxAuditView` 近期视图缓存，供 `sandbox.getRecentAuditView` 和 UI 快速查询。该缓存可清理、可丢失，不作为持久审计事实源。

---

## 九、性能指标

| 指标 | 要求 | 说明 |
|------|------|------|
| 路径校验 | < 0.1ms | 正则匹配 |
| 命令校验 | < 0.5ms | 多规则匹配 |
| 审计写入 | < 1ms | 异步写入 |

---

## 十、测试策略

```
单元测试：路径白名单/黑名单、命令规则匹配、操作分级
集成测试：Project / Worktree 信任流程、Kernel Policy 约束接入、审计日志完整性
```
