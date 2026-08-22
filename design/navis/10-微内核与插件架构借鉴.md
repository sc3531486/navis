# DeepSeek-Harness 设计理念对齐

## 核心理念

"There is no privileged core to patch: you extend dsh by mounting a plugin beside the others."

Navis 对齐这一理念：**万物皆插件**。

## 传统 IDE vs Navis 白板

| 维度 | 传统 IDE | Navis 白板 |
|------|---------|-----------|
| 布局 | 固定分区（侧栏/编辑器/终端） | 动态插槽（扩展自主决定） |
| 扩展 | 注入固定钩子 | 自主挂载布局树 |
| 核心 | 有特权业务逻辑 | 仅 DI + 事件 + 插槽 |

## 实现原理

- `NavisContext`：Cordis 风格服务容器，扩展通过 `provide/use` 注册和消费服务
- `SlotRenderer`：动态命名插槽，支持递归嵌套
- `NavisPlugin`：扩展挂载入口，`apply(ctx)` 中注册插槽和命令
- `ExtensionRegistry`：后端动态 RPC 路由，扩展注册处理函数

## 应用场景

- 开发者工作台（navis-code）
- 柜面双屏系统
- 双录向导
- 任意业务场景
