# navis-ai-platform

AI 平台扩展（骨架占位）：大模型网关、MCP Server、LSP 客户端。

## 贡献点

| 类型 | 键 | 说明 |
| --- | --- | --- |
| slot | `navis-code.statusbar` → `GatewayStatus` | 网关状态 |
| providesSlots | `navis-ai.gateway-panel` | 网关配置面板 |
| tool | `gateway.chat` / `gateway.list-models` / `mcp.call` | 模型对话、模型列表、MCP 调用 |

## 下一步

- 提供 stdio JSON-RPC 后端进程（网关转发、MCP transport）
- 绑定 `GatewayStatus` 组件