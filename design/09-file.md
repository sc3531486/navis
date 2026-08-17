# 09 - File 文件系统 详细设计

> 模块编号：09 | 层级：基础能力层
> 依赖：01-Logger, 02-Event+IPC, 06-Sandbox
> 被依赖：08-Session, 12-MCP, 16-Agent, 15-Code-Edit, 14-LSP, 21-Git

---

## 一、模块概述

### 1.1 定位

File 是文件系统抽象层，封装跨平台文件操作，提供统一的文件读写、目录管理、文件监听、路径管理能力。所有 UI IPC、MCP 工具和 Agent 可触发入口都必须经 Kernel Policy 评估；Sandbox 只作为 Policy Constraint 的业务实现，不允许入口直接绕过 Policy 调用裸文件 helper。

### 1.2 职责边界

```
负责：
├── 文件读写（文本/二进制、大文件分片）
├── 目录操作（创建/遍历/删除）
├── 文件监听（变更通知）
├── 路径管理（跨平台标准化、路径解析）
├── 文件类型识别（MIME、语言识别）
├── 文件元数据（大小/权限/修改时间）
└── 符号链接处理

不负责：
├── 代码编辑（精确替换/Diff） → Code Edit
├── 文件安全语义定义 → Sandbox / Policy Constraint
├── Git 操作 → Git
└── 文件内容搜索 → File / Grep MCP 工具
```

---

## 二、架构设计

```
file/
├── mod.rs              # 模块入口
├── operations.rs       # 文件操作（读写/复制/移动/删除）
├── path_manager.rs     # 路径管理（标准化/解析/跨平台）
├── watcher.rs          # 文件监听
├── file_type.rs        # 文件类型识别
├── metadata.rs         # 文件元数据
├── large_file.rs       # 大文件分片处理
└── symlink.rs          # 符号链接处理
```

---

## 三、数据模型

```rust
struct FileEntry {
    path: PathBuf,
    name: String,
    is_dir: bool,
    is_symlink: bool,
    size: u64,
    modified: DateTime<Utc>,
    created: DateTime<Utc>,
    permissions: FilePermissions,
    mime_type: Option<String>,
    language: Option<String>,      // 编程语言识别
}

struct FileChange {
    path: PathBuf,
    change_type: FileChangeType,
    timestamp: DateTime<Utc>,
}

enum FileChangeType {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf },
}

struct ReadOptions {
    encoding: Option<String>,      // 编码（默认 UTF-8）
    max_size: Option<u64>,         // 最大读取大小
    offset: Option<u64>,           // 偏移量
    limit: Option<u64>,            // 读取长度
}

struct WriteOptions {
    create_dirs: bool,             // 自动创建父目录
    backup: bool,                  // 写入前备份
    atomic: bool,                  // 原子写入（先写临时文件再重命名）
}
```

---

## 四、接口定义

### 4.1 Rust API

> UI IPC、MCP 和 Agent 入口只能调用 `*_with_policy` API。裸 helper 只服务路径解析、低层复用和单元测试，不能直接挂到用户可触发入口。
> Policy 路径固定为 `OperationRequest -> PolicyInput -> PolicyEngine -> Sandbox Constraint -> PolicyDecision`，校验失败时返回真实错误或审批需求。

```rust
// 文件读取
File::read_with_policy(&self, sandbox: Arc<Sandbox>, path: &Path, options: ReadOptions) -> Result<String>
// 内部：Kernel PolicyEngine evaluate(tool.file.read) 后执行读取
File::read_bytes(&self, path: &Path) -> Result<Vec<u8>>
// 用户入口必须提供 with_policy 包装
File::read_lines(&self, path: &Path, start: usize, end: usize) -> Result<Vec<String>>
// 用户入口必须提供 with_policy 包装

// 文件写入
File::write_with_policy(&self, sandbox: Arc<Sandbox>, path: &Path, content: &str, options: WriteOptions) -> Result<()>
// 内部：Kernel PolicyEngine evaluate(tool.file.write) 后执行写入
File::write_bytes(&self, path: &Path, data: &[u8]) -> Result<()>
// 用户入口必须提供 with_policy 包装
File::append(&self, path: &Path, content: &str) -> Result<()>
// 用户入口必须提供 with_policy 包装

// 文件操作
File::copy_with_policy(&self, sandbox: Arc<Sandbox>, from: &Path, to: &Path) -> Result<()>
// 内部：Kernel PolicyEngine evaluate(read/write) 后执行复制
File::move_file_with_policy(&self, sandbox: Arc<Sandbox>, from: &Path, to: &Path) -> Result<()>
// 内部：Kernel PolicyEngine evaluate(delete/write) 后执行移动
File::delete_with_policy(&self, sandbox: Arc<Sandbox>, path: &Path) -> Result<()>
// 内部：Kernel PolicyEngine evaluate(tool.file.delete / tool.dir.delete) 后执行删除
File::exists(&self, path: &Path) -> bool
// 用户入口必须提供 with_policy 包装

// 目录操作
File::create_dir_with_policy(&self, sandbox: Arc<Sandbox>, path: &Path, recursive: bool) -> Result<()>
// 内部：Kernel PolicyEngine evaluate(tool.dir.create) 后创建目录
File::list_dir_with_policy(&self, sandbox: Arc<Sandbox>, path: &Path) -> Result<Vec<FileEntry>>
// 内部：Kernel PolicyEngine evaluate(tool.file.read) 后遍历目录
File::list_dir_recursive_with_policy(&self, sandbox: Arc<Sandbox>, path: &Path, max_depth: Option<usize>) -> Result<Vec<FileEntry>>
// 内部：Kernel PolicyEngine evaluate(tool.file.read) 后递归遍历
File::delete_dir_with_policy(&self, sandbox: Arc<Sandbox>, path: &Path, recursive: bool) -> Result<()>
// 内部：Kernel PolicyEngine evaluate(tool.dir.delete) 后删除目录

// 元数据
File::metadata(&self, path: &Path) -> Result<FileEntry>
// 用户入口必须提供 with_policy 包装
File::file_type(&self, path: &Path) -> Result<String>
// 用户入口必须提供 with_policy 包装
File::language(&self, path: &Path) -> Option<String>
// 用户入口必须提供 with_policy 包装

// 文件监听
File::watch(&self, path: &Path, recursive: bool) -> Result<WatcherId>
// 内部：Sandbox::check_path(path)? 后启动监听
File::unwatch(&self, watcher_id: WatcherId) -> Result<()>

// 路径管理
File::normalize(&self, path: &Path) -> PathBuf
File::resolve(&self, base: &Path, relative: &Path) -> PathBuf
File::relative(&self, path: &Path, base: &Path) -> PathBuf
File::is_within(&self, path: &Path, base: &Path) -> bool
```

### 4.2 MCP 工具（暴露给 Agent）

```
file.read(path, encoding?, offset?, limit?) → string
file.readLines(path, start, end) → string[]
file.readBytes(path) → base64-string
file.write(path, content, createDirs?) → void
file.append(path, content) → void
file.list(dir, recursive?, maxDepth?) → FileEntry[]
file.exists(path) → boolean
file.metadata(path) → FileEntry
file.copy(from, to) → void
file.move(from, to) → void
file.delete(path) → void
file.createDir(path, recursive?) → void
file.watch(path, recursive?) → WatcherId
file.unwatch(watcherId) → void
```

#### WatcherId 类型定义

```
类型：String（UUID v4 格式，如 "a1b2c3d4-e5f6-7890-abcd-ef1234567890"）
说明：每次调用 file.watch() 返回唯一 WatcherId，用于后续 unwatch 精确取消监听
```

---

## 五、跨平台路径处理

```
Windows: C:\Users\user\project\src\main.ts
macOS:   /Users/user/project/src/main.ts
Linux:   /home/user/project/src/main.ts

统一内部表示：使用 std::path::PathBuf，自动处理分隔符
API 输入：接受 "/" 和 "\"，自动标准化
API 输出：使用当前平台的分隔符
```

---

## 六、事件定义

```typescript
type FileEvents = {
  'file.created':  { path: string; watcherId?: string }
  'file.modified': { path: string; watcherId?: string }
  'file.deleted':  { path: string; watcherId?: string }
  'file.renamed':  { from: string; to: string; watcherId?: string }
  'file.watcher.error': { path: string; error: string; watcherId?: string }
}
```

---

## 七、性能指标

| 指标 | 要求 | 说明 |
|------|------|------|
| 文件读取（1MB） | < 10ms | 含 Sandbox 校验 |
| 文件写入（1MB） | < 20ms | 原子写入 |
| 目录遍历（1000文件） | < 50ms | 含元数据读取 |
| 文件监听延迟 | < 100ms | 变更到通知 |
| 路径标准化 | < 0.01ms | 字符串操作 |

---

## 八、测试策略

```
单元测试：路径标准化、文件类型识别、大文件分片
集成测试：跨平台路径、文件监听、Sandbox 集成
```
