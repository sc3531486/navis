# navis-knowledge

知识库扩展（骨架占位）：文档检索、索引与知识注入。

## 贡献点

| 类型 | 键 | 说明 |
| --- | --- | --- |
| slot | `navis-code.sidebar.left` → `KnowledgePanel` | 知识面板 |
| providesSlots | `navis-knowledge.panel` | 面板子插槽 |
| tool | `knowledge.search` / `knowledge.add` | 检索/写入 |
| pipelineHook | `assembleContext` → `injectKnowledge` | 上下文知识注入 |

## 下一步

- 提供后端进程暴露 `knowledge.*` 工具
- 绑定 `KnowledgePanel` 组件