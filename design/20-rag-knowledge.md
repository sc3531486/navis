# 20 - Knowledge 项目知识管理

> 模块编号：20 | 层级：能力层
> 依赖：01-Logger, 02-Event+IPC, 09-File
> 被依赖：18-Context-Manager, 16-Agent

---

## 一、模块概述

### 1.1 定位

Knowledge 提供项目知识管理能力，通过配置注入、文件引用和关键词搜索，为 Agent 提供项目级上下文。

### 1.2 职责边界

```
负责：
├── 项目配置注入（加载 navis.md）
├── 文件引用（@file 读取指定文件）
├── 项目知识组织（摘要、索引、推荐）
├── 关键词搜索（grep 命令行搜索，当前版本不实现，使用 MCP 工具提供）
├── 项目结构摘要（文件树 + 关键文件列表）
└── 知识源管理

不负责：
├── 文件系统操作 → File 模块
├── 上下文组装 → Context Manager（只提供检索结果）
└── Agent 推理决策 → Agent 模块
```

---

## 二、架构设计

```
knowledge/
├── mod.rs                  # 模块入口
└── project_knowledge.rs    # 项目知识管理
                             # - navis.md 加载
                             # - @file 引用解析
                             # - 项目结构扫描
```

---

## 三、数据模型

```rust
struct KnowledgeSource {
    path: PathBuf,
    source_type: SourceType,     // file / directory
}

enum SourceType {
    File,
    Directory,
}

struct ProjectSummary {
    root: PathBuf,
    file_tree: String,           // 缩进式文件树文本
    key_files: Vec<String>,      // 关键文件路径列表
}
```

---

## 四、接口定义

```typescript
// 加载项目配置（navis.md）
knowledge.loadProjectConfig(projectPath: string): Promise<string>

// 读取指定文件内容（@file 引用）
knowledge.readFile(filePath: string): Promise<string>

// grep 关键词搜索
knowledge.searchCode(query: string, path?: string): Promise<string>

// 获取项目结构摘要
knowledge.getProjectSummary(projectPath: string): Promise<ProjectSummary>

// 列出已配置的知识源
knowledge.listSources(): Promise<KnowledgeSource[]>
```

---

## 五、知识检索流程

```
用户："这个项目的认证逻辑是怎么实现的？"
     │
     ▼
Agent 判断需要查找代码
     │
     ▼
├─ 已知文件路径 → readFile（@file 引用）
├─ 不确定位置 → searchCode（grep 关键词搜索）
├─ 需要全局概览 → getProjectSummary
└─ 需要项目约定 → loadProjectConfig（navis.md）
     │
     ▼
将结果注入上下文
     │
     ▼
Agent 基于上下文回答
```

**与竞品对齐**：Claude Code 使用 Grep / Glob / Read 工具完成同样的事情，Codex 使用 shell 命令直接搜索。Navis Go 采用相同策略——不需要向量数据库，grep + 文件读取已足够覆盖绝大多数项目知识检索场景。

---

## 六、事件定义

```typescript
type KnowledgeEvents = {
  'knowledge.config.loaded':   { projectPath: string; filePath: string }
  'knowledge.file.read':       { filePath: string; success: boolean }
  'knowledge.search.completed': { query: string; resultCount: number; duration: number }
  'knowledge.summary.built':   { projectPath: string; fileCount: number }
}
```

---

## 七、测试策略

```
单元测试：navis.md 加载解析、@file 路径解析、项目结构扫描
集成测试：searchCode（grep）端到端、多知识源切换
```

---

## 八、v2 增强路径

> 当前方案（v1）完全依赖 grep + 文件读取，零外部依赖。
> v2 可考虑引入"项目文件摘要索引"作为轻量中间层：

```
v2 可选增强：
├── 项目文件摘要索引（纯文本 JSON 或 SQLite，非向量）
│   ├── 启动时扫描项目，为每个文件生成一行摘要（路径 + 语言 + 行数 + 首行注释）
│   ├── Agent 搜索时先查摘要索引定位文件，再 readFile 获取全文
│   └── 相比纯 grep：更快定位、支持按语言/目录过滤
└── 这是超越竞品的轻量中间方案，无需引入 Embedding 模型或向量数据库
```
