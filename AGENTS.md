# AGENTS.md

本文件是 `D:\myworkspace\Navis Go` 的开发约束。文档中的“框架”默认指通用 Navis 宿主；文档中的“产品”默认指由扩展组合出的 Navis Code，不能把二者混为一谈。

## 项目定位

Navis 是基于 Tauri 2 的通用桌面应用白板和扩展运行时。Navis 只负责窗口与宿主生命周期、扩展发现/加载/启停、能力注册、事件、权限、存储、IPC、流式通道和 UI 投影等通用机制。

AI、Agent、会话、项目、编辑器、终端、知识库、记忆、任务、设置，以及柜面系统、双录系统等垂直业务，全部属于扩展，不得写入通用框架层。

Navis Code 是 Navis 的第一个产品，由 `extensions/navis-code/` 下的业务扩展和产品入口组合而成。未来增加其他产品时，应新增产品目录或独立扩展组合，不修改 Navis 的业务实现。

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
├── src/                              # 通用宿主前端；不得新增产品业务
│   ├── components/HostView/          # 扩展视图投影
│   ├── components/CommandPalette/    # 通用命令面板壳
│   ├── components/Dialog/            # 通用对话框壳
│   ├── components/ui/                # 通用原子组件
│   ├── stores/                       # 宿主状态、扩展桥和通用投影
│   ├── lib/                          # 热键、流、状态等通用工具
│   ├── styles/ theme/ i18n/          # 通用视觉、主题和国际化基础设施
│   └── bootstrap.ts                  # 宿主启动生命周期
├── src-tauri/src/                    # 通用宿主后端；不得新增产品业务
│   ├── kernel/                       # Kernel、Cordis 原语、事件、注册表、策略
│   ├── extension/                    # 扩展清单、加载、生命周期、组件和技能
│   ├── foundation/                   # 配置、IPC、存储、日志、流等基础能力
│   ├── security/                     # 认证、沙箱、权限和审计
│   ├── app/                          # Tauri 启动与基础设施装配
│   └── ui/                           # 通用扩展桥、路由、存储、网络和 UI 命令
└── extensions/
    ├── navis-code/                   # Navis Code 产品套件
    │   ├── navis-agent-core/         # Agent 业务扩展
    │   ├── navis-ai-platform/        # Gateway / MCP / LSP 等 AI 平台扩展
    │   ├── navis-session/            # 会话业务扩展
    │   ├── navis-project/            # 项目与工作树业务扩展
    │   ├── navis-task/               # 任务业务扩展
    │   ├── navis-editor/             # 编辑器、文件、Git、剪贴板扩展
    │   ├── navis-terminal/           # 终端扩展
    │   ├── navis-settings/           # 设置扩展
    │   ├── navis-knowledge/          # 知识库扩展
    │   ├── navis-memory/             # 记忆扩展
    │   ├── ExtensionUI/              # Navis Code 产品入口和产品壳（迁移完成前的组合层）
    │   └── navis-code-ui.tsx         # Navis Code 产品入口
    └── navis-demo/                   # 通用扩展示例
```

### 产品组合入口契约

产品入口不属于通用宿主。每个产品目录可声明：

```text
extensions/<product>/
├── product.json
├── <product>-ui.tsx
└── <extension-id>/
```

`product.json` 至少包含 `id`、`name`、`version`、`entry`；入口导出通用 `ProductDefinition`。`src/index.tsx` 只负责扫描并动态加载产品入口，无产品时显示纯白板。新增产品只能新增 `extensions/<product>/`，不得修改 `src/`、`src-tauri/src/` 或宿主中的产品 ID 特判。
### 前端产品壳迁移说明

当前 `src/router/`、`src/layouts/`、部分 `src/stores/` 聚合导出和少量 HostView 内置投影仍承载 Navis Code 工作台组合逻辑，这是历史过渡状态，不是通用框架契约。新的业务不得继续放入这些目录；最终应迁入 `extensions/navis-code/` 的产品壳或具体扩展，并由产品入口组合宿主能力。

## 扩展固定契约

每个扩展目录必须包含：

```text
extensions/<product>/<extension-id>/
├── extension.json
├── ExtensionUI/                   # 全部前端扩展代码、样式和资源
└── ExtensionBackend/              # 全部后端扩展点代码、逻辑组件或 native 服务
```

开发期扫描支持 `extensions/<product>/<extension>/extension.json`；运行时安装目录为 `<app_data>/extensions/<extension-id>/`。扩展目录名必须与 manifest 的 `id` 一致。

扩展通过 manifest、HostView、命令/菜单/快捷键、能力端口、Kernel EventBus、IPC/流和权限契约与宿主或其他扩展通信，不直接依赖其他扩展的内部 store 或实现。

## 关键约定

- Rust 使用 `tracing`，不使用 `log`。
- 前端到后端统一使用 Tauri `invoke`、事件或通用 stream。
- Store 使用 `createStore` 和 `set*` action；业务 store 归属对应扩展。
- 框架层不出现 Agent、Session、Project、Terminal 等业务实现或产品 ID 特判。
- 每个 Rust 模块用中文注释说明职责边界；新增代码注释使用中文。
- 修改文档时以当前代码和 manifest 为事实来源；目标态必须明确标注，不能冒充已完成。

