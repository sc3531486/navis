# Navis Go 开发说明

## 项目定位

Navis Go 包含通用应用白板框架（Navis）与产品形态（Navis Code）：

- `src/` 与 `src-tauri/src/` 是底层通用白板与扩展运行时底座，禁止引入垂直业务代码。
- `extensions/` 下各个目录自持有各自的业务代码（前端 `ExtensionUI/`、后端 `ExtensionBackend/`、清单 `extension.json`）。
- 根目录 `<product>.json`（如 `navis-code.json`、`teller-system.json`）负责定义产品装配方案。

## 常用命令

```bash
npm run dev
npm run build
npx tauri dev
npx tauri build
cd src-tauri && cargo check
cd src-tauri && cargo test
```

## 开发规范

- **Rust 日志**：统一使用 `tracing`。
- **扩展通信**：统一通过 `ctx.views`、`ctx.events`、`ctx.commands`、`ctx.services` 与 IPC 桥。
- **纯净化原则**：框架层禁止出现 Agent、Session、Project、Terminal 等业务硬编码。
- **代码注释**：使用规范中文注释说明职责边界。
