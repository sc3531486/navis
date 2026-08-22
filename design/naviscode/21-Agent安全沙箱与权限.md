# 07 — 安全沙箱与动态 ACL 详细设计（Sandbox & Dynamic ACL）

> 模块编号：07 | 模块归属：Navis 通用宿主底座 / 安全层  
> 依赖：01-日志服务, 02-进程间通信, 04-配置系统, 06-身份鉴权与凭据  
> 被依赖：文件操作扩展、终端执行、插件进程管理器

---

## 一、模块概述与定位

### 1.1 定位
Sandbox 是 Navis 运行时的**安全防御中心**。所有文件读写、系统 Shell 命令执行、外部网络请求必须经过 Sandbox 校验，提供基于能力原语的动态访问控制、命令黑白名单与安全审计。

### 1.2 职责边界
```text
负责：
├── 能力原语动态授权与校验（FsRead / FsWrite / ShellExec / Network / EventEmit）
├── 命令规则引擎（危险命令拦截、正则模式匹配、Shell 语义识别）
├── 路径访问边界校验（白名单 / 黑名单）
└── 操作安全审计轨迹记录与查询（navis_audit_log）

不负责：
├── 凭据加密与存储（由 06-身份鉴权与凭据 负责）
└── 操作系统级进程硬隔离
```

---

## 二、数据模型与能力原语（`src-tauri/src/core/sandbox.rs`）

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    FsRead,      // 本地文件读取
    FsWrite,     // 本地文件写入与删除
    ShellExec,   // 启动系统 Shell 执行命令
    Network,     // 发起外部网络 HTTP / WebSocket 请求
    EventEmit,   // 发布系统广播事件
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionToken {
    pub plugin_id: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub ts: u128,
    pub plugin_id: String,
    pub capability: String,
    pub allowed: bool,
    pub detail: String,
}
```

---

## 三、审计日志命令与查询

宿主暴露通用命令 `navis_audit_log` 供调试与管理员审计：

```rust
#[tauri::command]
fn navis_audit_log(sandbox: State<'_, Arc<Sandbox>>) -> Result<Vec<AuditEntry>, String> {
    Ok(sandbox.audit_log())
}
```
