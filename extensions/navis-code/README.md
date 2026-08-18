# Navis Code 扩展套件

Navis Code 是基于通用 Navis 框架组合的 Agent IDE 产品。

本目录包含 Navis Code 的产品入口、产品组合层和业务扩展。`src/` 与 `src-tauri/src/` 是通用 Navis 宿主；当前仍有少量历史产品壳位于宿主前端目录，详见根目录 `ARCHITECTURE_REVIEW.md`，不得继续扩展该过渡区。

产品入口为 `navis-code-ui.tsx`。除产品入口和组合层外，每个子目录都是独立扩展包，拥有自己的 `extension.json`、`ExtensionUI/` 与 `ExtensionBackend/`。

```text
extensions/navis-code/
├── navis-code-ui.tsx
├── ExtensionUI/                  # 产品组合层和产品级 UI 资源
├── navis-agent-core/
├── navis-ai-platform/
├── navis-session/
├── navis-project/
├── navis-task/
├── navis-editor/
├── navis-terminal/
├── navis-settings/
├── navis-knowledge/
└── navis-memory/
```
