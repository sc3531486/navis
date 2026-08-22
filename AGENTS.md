# AGENTS.md

本文件是 `D:\myworkspace\Navis Go` 的核心架构与开发约束。文档中的“框架”默认指通用 Navis 白板宿主与扩展运行时；文档中的“产品”默认指由产品清单组合出的具体应用形态（如 Navis Code、柜面系统、双录系统等），不能把二者混为一谈。

## 项目定位

Navis 是基于 Tauri 2 的通用桌面应用白板与扩展运行时（灵感源自 Cordis 扩展体系）。Navis 框架只负责窗口与宿主生命周期、扩展发现/加载/启停、IoC 服务容器（DI）、事件总线（emit/waterfall/serial/parallel）、响应式插槽树（DynamicSlot）、通用命令、沙箱权限、存储、多路复用 IPC 与流式通道等通用基础设施。

AI、Agent、会话、项目、编辑器、终端、知识库、记忆、任务、设置，以及银行柜面系统、双录系统等所有垂直业务，**全部属于扩展（Extensions）**，严禁写入底层框架层（`src/` 与 `src-tauri/`）。

Navis Code 是在 Navis 框架上装配的第一个产品形态，由 `navis-code.json` 声明装配的套件扩展组合而成。未来增加其他产品形态（如银行柜面系统、双录系统）时，只需新增扩展目录并在其产品清单（如 `teller-system.json`）中声明装配，**无需修改 Navis 通用框架任何一行源码**。

## 常用命令

```bash
npm run dev
npm run build
npx tauri dev
npx tauri build
cd src-tauri && cargo check
cd src-tauri && cargo test
```

## 物理目录边界

```text
Navis Go/
├── navis-code.json                   # Navis Code 产品组装清单
├── teller-system.json                # 柜面系统产品组装清单（示例）
├── src/                              # Navis 通用宿主前端（纯净框架层，禁止业务逻辑）
│   ├── app/                          # 通用应用白板外壳 (WhiteboardShell)
│   ├── core/                         # Cordis 上下文、插槽树、清单泛型分发、IPC 桥接
│   │   ├── context.ts                # NavisContext (DI, Events, Views, Commands, Services)
│   │   ├── bootstrap.ts              # 宿主启动生命周期编排
│   │   ├── loader.ts                 # 扩展 UI 插件动态加载器
│   │   ├── tauri-bridge.ts           # 前后端 IPC/Stream 通信桥
│   │   ├── slots/                    # 响应式插槽树 (DynamicSlot, SlotStore)
│   │   ├── manifest/                 # 泛型贡献点分发中心 (ContributionRegistry, handlers)
│   │   └── components/               # 延迟组件解析注册表 (ComponentRegistry)
│   ├── styles/                       # 通用基础重置、滚动条、白板表面与布局样式
│   ├── theme/                        # 通用主题变量与模式定义
│   └── index.tsx                     # 宿主统一渲染入口
├── src-tauri/src/                    # Navis 通用宿主后端（纯净框架层，禁止业务逻辑）
│   ├── kernel/                       # 清单解析、扩展注册表、产品装配配置 (ProductConfig)
│   │   ├── mod.rs                    # 扩展注册表与 RPC 分发
│   │   ├── manifest.rs               # 通用 ExtensionManifest 清单解析
│   │   └── product.rs                # 产品形态装配清单解析与过滤
│   ├── core/                         # 通用 IPC、进程管理与安全沙箱
│   │   ├── ipc_bridge.rs             # 通用多路复用 JSON-RPC stdio 路由器与流式 Channel
│   │   ├── process_supervisor.rs     # Node/Python/可执行文件插件后端进程生命周期管理
│   │   ├── sandbox.rs                # 细粒度权限控制与审计日志
│   │   └── mod.rs
│   ├── lib.rs                        # Tauri 命令注册与宿主初始化
│   └── main.rs
└── extensions/                       # 业务扩展目录（万物皆扩展）
    ├── navis-code/                   # 【Navis Code 产品专属套件】
    │   ├── navis-code/               # 产品壳扩展（声明根布局与公共子插槽）
    │   ├── navis-agent-core/         # Agent 执行流、Composer 与时间线扩展
    │   ├── navis-editor/             # 代码编辑器与 Diff 视图扩展
    │   └── navis-terminal/           # 终端与 PTY 扩展
    ├── teller-system/                # 【银行柜面专属套件】（示例形态）
    │   └── README.md                 # 柜面系统套件说明
    └── shared/                       # 【跨产品通用共享扩展池】（各产品清单按需装配）
        ├── navis-ai-platform/        # 统一 AI 网关、模型管理与 ToolRegistry
        ├── navis-session/            # 会话管理与历史列表扩展
        ├── navis-project/            # 项目工作区与文件树扩展
        ├── navis-knowledge/          # 知识库与 RAG 扩展
        ├── navis-memory/             # 长期记忆扩展
        ├── navis-settings/           # 设置面板与配置扩展
        ├── navis-task/               # 任务看板与计划跟踪扩展
        └── navis-demo/               # 通用独立扩展示例
```

## 扩展固定契约

每个扩展目录必须遵循统一物理结构：

```text
extensions/<extension-id>/
├── extension.json                    # 扩展清单（元数据、权限声明、contributes 贡献点）
├── README.md                         # 扩展说明
├── ExtensionUI/                      # 全部前端扩展代码、样式与资源
│   ├── src/
│   │   ├── index.tsx                 # 扩展入口（导出 NavisPlugin）
│   │   └── components/               # 扩展专属 SolidJS 组件
│   └── styles/                       # 扩展专属 CSS 样式
└── ExtensionBackend/                 # 全部后端扩展点代码
    ├── src/                          # 逻辑实现代码
    └── main.mjs (或可执行文件)        # 后端进程入口（通过 stdio 承接 JSON-RPC）
```

- **开发期扫描**：自动扫描 `extensions/*/extension.json`，并按激活产品清单过滤。
- **运行期安装**：位于 `<app_data>/navis/extensions/<extension-id>/`。
- **扩展通信**：扩展通过 `ctx.views.register` 投影 UI 到插槽、通过 `ctx.events` 发布/订阅事件、通过 `ctx.commands` 注册/执行命令、通过 `ctx.services` 注入/获取服务，严禁直接跨扩展侵入对方内部私有模块。

## 关键约定

- **Rust 日志**：统一使用 `tracing`，严禁使用 `log` 或直接 `println!`。
- **前后端通信**：统一通过通用 IPC `navis_dispatch`、`core_route_ipc`、`core_route_stream`，宿主不暴露 Git/Terminal/LSP 等任何专有命令。
- **前端状态管理**：扩展自持 SolidJS store，宿主只维护通用插槽树与服务容器。
- **框架纯净化**：`src/` 与 `src-tauri/src/` 中绝对禁止出现 Agent、Session、Project、Terminal 等特定业务实现或产品 ID 硬编码特判。
- **注释与文档**：每个 Rust 与 TypeScript 核心模块均需提供清晰中文职责说明。
