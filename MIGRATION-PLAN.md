# Navis Go 架构迁移计划

> 更新日期：2026-08-18
> 状态：迁移进行中。本文只记录当前代码已经具备的能力，不把桩实现描述为完整业务实现。

## 一、不可变边界

1. `src/` 与 `src-tauri/src/` 只保留 Navis 通用宿主能力：窗口、白板、扩展发现与生命周期、Cordis/Kernel 原语、能力注册、事件、权限、存储、IPC、流和 UI 投影。
2. Agent、Gateway、MCP、LSP、会话、项目、任务、编辑器、终端、设置、知识库、记忆，以及柜面系统、双录系统等全部是扩展业务。
3. 新产品必须新增 `extensions/<product>/`，通过产品清单和产品入口组合扩展；新增产品不得修改 `src/` 或 `src-tauri/src/`。
4. 扩展只能依赖 Navis 稳定能力契约和其他扩展的公开能力，不得依赖宿主业务模块或其他扩展的内部 store。

## 二、当前已落地

- Navis 宿主前端已收敛到 `src/` 的白板、扩展视图投影、通用状态、主题、菜单、快捷键和桥接能力。
- Navis 宿主后端已收敛到 `src-tauri/src/{kernel,extension,foundation,security,app,ui}`。
- Navis Code 扩展套件已物理放入 `extensions/navis-code/<extension-id>/`，每个扩展具备 `extension.json`、`ExtensionUI/`、`ExtensionBackend/` 固定目录。
- 开发期扩展扫描支持 `extensions/<product>/<extension>/extension.json`，运行时安装仍使用 `<app_data>/extensions/<extension-id>/`。
- 根入口已经改为通用产品加载器：扫描 `extensions/*/product.json`，按请求产品、默认产品或首个产品选择入口；无产品时显示纯 Navis 白板。
- 扩展后端的历史 `crate::domains::*`、`crate::ai::*`、`crate::tool::*` 等宿主反向引用已移除，后端源码由对应扩展目录自持有。
- Cordis Context、Service、Fiber、Scope、扩展生命周期、能力服务、组件轨和事件订阅已接入通用宿主装配链路。

## 三、当前真实未完成项

### 3.1 扩展后端实现完整化

当前部分 `ExtensionBackend/src` 文件仍是迁移适配层或最小桩，目录归属已经正确，但尚未全部恢复为可独立编译的完整业务 crate。后续必须：

- 为需要 native Rust 的扩展建立独立构建入口和 Navis SDK 依赖；
- 恢复真实 Agent、Gateway、Session、Project、Terminal 等业务实现；
- 通过能力端口、事件、IPC 和 stream 与宿主通信；
- 禁止把实现搬回 `src-tauri/src`。

### 3.2 前端产品壳继续收敛

`extensions/navis-code/ExtensionUI/` 是 Navis Code 产品壳，不是通用宿主。`src/components/HostView`、`src/stores` 中只允许保留通用投影和桥接；发现新的产品业务后应继续迁入 Navis Code 产品壳或具体扩展。

### 3.3 构建期扩展资源隔离

扩展资源必须位于所属扩展的 `ExtensionUI/assets` 或 `ExtensionUI/src/assets`。跨扩展引用应通过稳定资源协议、公开资源入口或复制到使用方扩展，禁止指向宿主 `src/assets`。

## 四、目录契约

```text
extensions/<product>/<extension-id>/
├── extension.json
├── ExtensionUI/
└── ExtensionBackend/

extensions/<product>/
├── product.json                 # 产品组合清单
├── <product>-ui.tsx             # 产品入口，组合宿主与扩展
└── <extension-id>/              # 业务扩展
```

`product.json` 至少声明 `id`、`name`、`version`、`entry`；默认产品使用 `default: true`。产品入口导出通用 `ProductDefinition`，宿主不写任何产品 ID 特判。

## 五、验收标准

- 新增柜面系统或双录系统时，只新增产品目录、产品清单、产品入口和扩展，不修改 Navis 底层源码。
- `src/` 与 `src-tauri/src/` 不出现业务实现、业务 store、产品入口或产品 ID 特判。
- 扩展契约测试覆盖嵌套开发目录、manifest、固定目录、入口路径和后端反向依赖审计。
- `npm run build`、`cargo fmt --check`、`cargo check`、`cargo test` 全部通过。
