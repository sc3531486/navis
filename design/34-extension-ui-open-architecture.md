# 34 - 扩展 UI 开放架构详细设计

> 边界说明：本文描述 Navis 扩展 UI 的开放架构设计。Navis Code 的路由、布局和业务视图属于 `extensions/navis-code/` 产品组合。

> 模块编号：34 | 层级：UI 层 × Extension 层（cross-cutting）
> 依赖：07-Extension、NavisContext（前端）、DynamicRpcHandler（后端）、Sandbox
> 被依赖：Notification、Hotkey、Editor、Gateway

---

## 一、模块概述

### 1.1 定位

本设计解决 Navis Go 的"万物皆扩展"在 UI 层的落地：扩展通过 `NavisPlugin` 接口自主挂载布局树到 root 插槽，支持 **Slot-in-Slot 递归**开辟子插槽，完全脱离固定分区限制。

核心约束：
- **不新开窗口、不脱离桌面端**
- **扩展代码全部运行在本地桌面进程内，离线可用**
- **能力一律经 Rust 白名单授权**

### 1.2 设计原则

1. **开放插槽命名空间**：扩展可定义任意命名的插槽，支持递归嵌套
2. **声明式挂载**：扩展通过 `NavisPlugin.apply(ctx)` 声明式注册插槽和命令
3. **宿主布局最终裁决权**：扩展插槽的尺寸和位置由宿主布局算法裁决
4. **严格沙箱 + 白名单桥**：扩展只能调用 manifest 声明的白名单能力
5. **fail-closed**：未声明的能力必须显式拒绝，禁止静默忽略

---

## 二、NavisContext 插槽系统

### 2.1 NavisContext 接口

```typescript
class NavisContext {
  // 服务注册与消费
  provide<T>(name: string, service: T): void;
  use<T>(name: string): T;
  has(name: string): boolean;
  
  // 事件系统
  on(event: string, handler: EventHandler): () => void;
  emit(event: string, payload?: any): void;
  
  // 插槽系统
  registerSlot(target: string, item: SlotItem): () => void;
  getSlotItems(target: string): SlotItem[];
  
  // 命令系统
  registerCommand(id: string, handler: (args?: any) => void | Promise<void>): () => void;
  executeCommand(id: string, args?: any): Promise<void> | void;
  
  // 插件加载
  async plugin(plugin: NavisPlugin): Promise<void>;
}

interface SlotItem {
  id: string;                      // 插槽项 ID
  priority?: number;               // 优先级（数字越小越靠前，默认 100）
  component: () => JSX.Element;    // 渲染组件
}
```

### 2.2 插槽注册与渲染

```typescript
// 注册插槽
const unsub = ctx.registerSlot('root', {
  id: 'my-extension:panel',
  priority: 10,
  component: () => <MyPanel />
});

// 渲染插槽
<SlotRenderer ctx={ctx} target="root" />

// 取消注册
unsub();
```

### 2.3 插槽事件

```typescript
// 监听插槽更新
ctx.on('slot:root:updated', (items) => {
  console.log('Root slot items:', items);
});

// 监听插件挂载
ctx.on('plugin:my-extension:mounted', () => {
  console.log('Plugin mounted');
});
```

---

## 三、SlotRenderer 动态渲染

### 3.1 SlotRenderer 接口

```typescript
interface SlotRendererProps {
  ctx: NavisContext;               // NavisContext 实例
  target: string;                  // 目标插槽名称
  class?: string;                  // CSS 类名
  fallback?: JSX.Element;          // 空插槽时的降级内容
}

const SlotRenderer: Component<SlotRendererProps>;
```

### 3.2 渲染机制

SlotRenderer 自动订阅指定插槽的更新事件，当插槽内容变化时重新渲染：

```typescript
// 基础用法
<SlotRenderer ctx={ctx} target="root" />

// 带降级内容
<SlotRenderer 
  ctx={ctx} 
  target="my-extension:sidebar"
  fallback={<div>Empty sidebar</div>}
/>

// 嵌套渲染
<SlotRenderer ctx={ctx} target="root">
  <SlotRenderer ctx={ctx} target="left-sidebar" />
  <SlotRenderer ctx={ctx} target="main-content" />
  <SlotRenderer ctx={ctx} target="right-sidebar" />
</SlotRenderer>
```

### 3.3 渲染流程

```
SlotRenderer 挂载
  │
  ├── 订阅 slot:{target}:updated 事件
  │
  ▼
获取插槽内容
  │
  ├── ctx.getSlotItems(target)
  │
  ▼
按 priority 排序
  │
  ├── items.sort((a, b) => a.priority - b.priority)
  │
  ▼
渲染每个 SlotItem
  │
  ├── <For each={items}>{item => item.component()}</For>
  │
  ▼
插槽更新时重新渲染
  │
  ├── setTick(t => t + 1)
  └── 重新执行渲染流程
```

---

## 四、Slot-in-Slot 递归插槽

### 4.1 递归开辟机制

扩展可以在任意已注册的插槽内开辟子插槽，支持无限递归嵌套：

```
root (顶级插槽，由 WhiteboardShell 渲染)
├── navis-code:workbench (Navis Code 注册)
│   ├── navis-code:left-sidebar (Navis Code 注册)
│   │   ├── navis-code:session-list (Navis Code 注册)
│   │   └── navis-code:mode-selector (Navis Code 注册)
│   ├── navis-code:main-content (Navis Code 注册)
│   │   ├── navis-code:chat (Navis Code 注册)
│   │   └── navis-code:editor (Navis Code 注册)
│   └── navis-code:right-sidebar (Navis Code 注册)
│       └── navis-code:terminal (Navis Code 注册)
└── my-extension:panel (其他扩展注册)
```

### 4.2 递归注册示例

```typescript
// Navis Code 产品扩展
export const NavisCodeExtension: NavisPlugin = {
  name: 'navis-code',
  apply: async (ctx) => {
    // 在 root 中注册主工作台
    ctx.registerSlot('root', {
      id: 'navis-code:workbench',
      priority: 10,
      component: () => (
        <div class="navis-workbench">
          <SlotRenderer ctx={ctx} target="navis-code:left-sidebar" />
          <SlotRenderer ctx={ctx} target="navis-code:main-content" />
          <SlotRenderer ctx={ctx} target="navis-code:right-sidebar" />
        </div>
      )
    });
    
    // 在主工作台中注册左侧栏
    ctx.registerSlot('navis-code:left-sidebar', {
      id: 'navis-code:session-list',
      priority: 10,
      component: () => <SessionList />
    });
    
    // 在左侧栏中注册会话列表
    ctx.registerSlot('navis-code:session-list', {
      id: 'navis-code:session-item',
      priority: 100,
      component: () => <SessionItem />
    });
  }
};
```

### 4.3 插槽查找算法

```
查找插槽 "navis-code:session-item" 的渲染路径：
  │
  ├── 1. 在 root 中查找 "navis-code:workbench"
  │   └── 渲染 navis-code:workbench 组件
  │
  ├── 2. 在 navis-code:workbench 中查找 "navis-code:left-sidebar"
  │   └── 渲染 navis-code:left-sidebar 组件
  │
  ├── 3. 在 navis-code:left-sidebar 中查找 "navis-code:session-list"
  │   └── 渲染 navis-code:session-list 组件
  │
  └── 4. 在 navis-code:session-list 中查找 "navis-code:session-item"
      └── 渲染 navis-code:session-item 组件
```

---

## 五、NavisPlugin 接口规范

### 5.1 NavisPlugin 接口

```typescript
interface NavisPlugin {
  name: string;                    // 插件名称（唯一标识）
  apply: (ctx: NavisContext) => void | Promise<void>;  // 挂载函数
}
```

### 5.2 挂载契约

扩展必须在 `apply` 函数中完成以下注册：

```typescript
export const MyPlugin: NavisPlugin = {
  name: 'my-extension',
  apply: async (ctx) => {
    // 1. 注册插槽（必须）
    ctx.registerSlot('root', {
      id: 'my-extension:main',
      priority: 10,
      component: () => <MyComponent />
    });
    
    // 2. 注册命令（可选）
    ctx.registerCommand('my-extension.open', () => {
      console.log('Open my extension');
    });
    
    // 3. 提供服务（可选）
    ctx.provide('my-extension:service', {
      getData: () => fetch('/api/data')
    });
  }
};
```

### 5.3 生命周期

```
插件注册
  │
  ├── ctx.plugin(plugin) 调用
  │
  ▼
执行 apply(ctx)
  │
  ├── 注册插槽
  ├── 注册命令
  ├── 提供服务
  │
  ▼
触发 plugin:{name}:mounted 事件
  │
  ▼
插件运行中
  │
  ├── 监听插槽更新
  ├── 执行命令
  └── 消费服务
  │
  ▼
插件卸载
  │
  ├── 清理插槽
  ├── 清理命令
  └── 清理服务
```

---

## 六、扩展 UI 通信架构

### 6.1 通信方式

扩展 UI 与宿主通信采用以下方式：

| 通信方式 | 说明 | 示例 |
|----------|------|------|
| NavisContext | 前端服务注入与事件 | `ctx.provide()` / `ctx.use()` |
| SlotRenderer | 插槽渲染与更新 | `<SlotRenderer target="..." />` |
| 命令系统 | 前端命令注册与执行 | `ctx.registerCommand()` |
| Tauri IPC | 前后端通信 | `invoke('command', args)` |
| EventBus | 事件发布与订阅 | `ctx.emit()` / `ctx.on()` |

### 6.2 数据流

```
扩展 UI
  │
  ├── 通过 ctx.use() 消费宿主服务
  ├── 通过 ctx.emit() 发布事件
  ├── 通过 ctx.executeCommand() 执行命令
  │
  ▼
NavisContext
  │
  ├── 服务注入
  ├── 事件路由
  ├── 命令分发
  │
  ▼
宿主核心
  │
  ├── 服务实现
  ├── 事件处理
  ├── 命令执行
  │
  ▼
后端 (Tauri)
  │
  ├── IPC 处理
  ├── 数据存储
  └── 系统调用
```

---

## 七、安全考量

### 7.1 沙箱隔离

- 前端沙箱：Solid.js 组件级隔离，扩展无法访问宿主组件的内部状态
- 后端沙箱：所有系统调用通过 Sandbox 权限检查
- 插槽隔离：扩展只能访问自己注册的插槽

### 7.2 权限控制

```json
{
  "permissions": {
    "slots": ["root", "my-extension:*"],
    "commands": ["my-extension.*"],
    "services": ["my-extension:service"]
  }
}
```

### 7.3 安全边界

- 扩展只能挂载到已声明的插槽
- 扩展只能注册已声明的命令
- 扩展只能消费已声明的服务
- 未声明的能力必须 fail-closed

---

## 八、错误处理

| 场景 | 行为 |
|------|------|
| 插槽注册失败 | 返回取消注册函数，不抛出异常 |
| 插槽渲染失败 | 显示 fallback 内容，控制台输出错误 |
| 命令注册失败 | 返回取消注册函数，不抛出异常 |
| 命令执行失败 | 返回 Promise reject，控制台输出错误 |
| 服务注入失败 | 抛出异常，阻止插件继续加载 |
| 服务消费失败 | 抛出异常，阻止组件继续渲染 |

---

## 九、性能指标

| 指标 | 目标 |
|------|------|
| 插槽注册 | <= 1ms |
| 插槽渲染 | <= 16ms |
| 命令注册 | <= 1ms |
| 命令执行 | <= 100ms |
| 事件传播 | <= 16ms |
| 插件加载 | <= 500ms |

---

## 十、测试策略

```
单元测试：NavisContext 服务注入、插槽注册与渲染、命令注册与执行、事件传播
集成测试：插槽递归嵌套、多插件并存、插件生命周期、权限校验
端到端：扩展声明插槽 → 插槽出现 → 扩展注册命令 → 命令可执行 → 
       扩展提供服务 → 服务可消费
性能测试：大量插槽渲染、高频命令执行、事件风暴
```
