# 扩展统一根目录（extensions/）

> 权威规范请参见 `design/36-扩展开发指南.md` 与 `智能体开发规范.md`。

## 一、产品分组与共享池组织结构

为了支持多产品形态（如 Navis Code、银行柜面系统、双录系统等）并实现公共能力最大化复用，`extensions/` 采用 **“产品专属套件（Product Suites）+ 跨行业共享池（Shared Extensions）”** 分组体系：

```text
extensions/
├── navis-code/                     # 【Navis Code 产品专属套件】
│   ├── navis-code/                 # Navis Code 产品壳主布局 (root/overlay 挂载点)
│   ├── navis-agent-core/           # Agent 执行流、Composer 对话区与时间线
│   ├── navis-editor/               # 代码编辑器与 Diff 视图
│   └── navis-terminal/             # PTY 终端与 Shell 命令行
│
├── teller-system/                  # 【银行柜面产品专属套件】（多产品形态示例）
│   └── README.md                   # 柜面系统套件说明
│
└── shared/                         # 【跨产品通用共享扩展池】（所有产品清单自由组装）
    ├── navis-ai-platform/          # 统一 AI 网关、模型管理与 ToolRegistry
    ├── navis-session/              # 会话管理与历史列表
    ├── navis-project/              # 工作区与项目目录管理
    ├── navis-knowledge/            # 业务知识库与本地 RAG 检索
    ├── navis-memory/               # 长期记忆与偏好配置
    ├── navis-settings/             # 系统与扩展参数配置弹窗
    ├── navis-task/                 # 任务看板与计划跟踪
    └── navis-demo/                 # 标准全链路示例扩展
```

## 二、扩展固定目录契约

每个扩展是一个独立的分发单元，内部遵循统一物理结构：

```text
extensions/<suite>/<extension-id>/
├── extension.json                 # 扩展清单（唯一扩展元数据与能力声明入口）
├── README.md                      # 扩展说明文档
├── ExtensionUI/                   # 全部前端扩展代码、样式和资源
│   ├── src/
│   │   ├── index.tsx              # 前端入口（导出 NavisPlugin）
│   │   ├── components/            # 扩展专属 SolidJS 组件
│   │   └── stores/                # 扩展自持状态
│   └── styles/                    # 扩展专属 CSS 样式
└── ExtensionBackend/              # 全部后端扩展点代码
    ├── src/                       # 后端业务源码
    └── main.mjs (或可执行文件)     # 后端进程入口（stdio JSON-RPC）
```

## 三、产品组装契约

产品形态不是宿主层的硬编码，而是由根级产品清单 `<product>.json`（如 `navis-code.json`、`teller-system.json`）声明组装的扩展集合：

```json
{
  "id": "navis-code",
  "name": "Navis Code",
  "shell": "navis-code",
  "description": "Navis Code AI Agent IDE",
  "extensions": [
    "navis-agent-core",
    "navis-ai-platform",
    "navis-editor",
    "navis-knowledge",
    "navis-memory",
    "navis-project",
    "navis-session",
    "navis-settings",
    "navis-task",
    "navis-terminal",
    "navis-demo"
  ]
}
```

通用宿主（`src/` 与 `src-tauri/`）在启动时递归扫描 `extensions/**/extension.json`，并依据当前激活的产品清单动态装配。没有产品清单时显示纯 Navis 白板。新增产品（如银行柜面系统）无需修改底层框架源码。

## 四、关键准则

- 所有业务领域均遵循统一的 manifest、Cordis 上下文、插槽投影、IPC 通信与权限沙箱契约，官方扩展无任何特权。
- 扩展间通信统一通过 `ctx.views`、`ctx.events`、`ctx.commands`、`ctx.services` 与 IPC 桥，禁止跨扩展直接侵入私有 store。
- 框架源码（`src/` 与 `src-tauri/`）严禁写入任何特定业务代码。
