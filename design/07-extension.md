# 07 - Extension 扩展系统详细设计

> 边界说明：本文描述通用 Navis 扩展运行时。Agent、Session、Project 等名称表示扩展贡献或能力合同，不表示这些业务属于 `src-tauri/src/`；Navis Code 业务位于 `extensions/navis-code/`。

> 模块编号：07 | 层级：extension 大域
> 依赖：Cordis（Rust：cordis-rs；前端宿主：@cordisjs/core）, kernel::ExtensionRegistry, kernel::DynamicRpcHandler
> 被依赖：UI-Framework, Command-Palette, Hotkey, Gateway, MCP, Agent

---

## 一、模块概述

### 1.1 定位

Extension 是应用层基于 Cordis 的扩展组合与服务生命周期层。它读取 Extension manifest、校验 contributes、管理安装/启用/禁用/卸载状态，并把每个扩展点从固定目录 `ExtensionUI/` / `ExtensionBackend/` 装载为 Cordis plugin/service，通过 capability port 交给已有宿主域承接。

Cordis `Context` 提供类型化服务容器，`Plugin`/`Service` 表达扩展单元，`Inject` 声明服务依赖，`Fiber` 管理插件生命周期，`effect`/disposer 回收运行时副作用。Kernel 仍保留 Registry / Pipeline / EventBus / Policy 四个通用原语；Cordis 负责扩展装配与生命周期，不复制这四原语。

### 1.2 核心原则

```
1. Cordis 是唯一扩展装配底座：plugin/service/Inject/Fiber/effect 全走 Cordis
2. 清单只声明能力：manifest/contributes 是插件元数据，不是运行时执行器
3. 启用时分发承接：loader 将 ExtensionUI/ / ExtensionBackend/ 下的扩展点装载为 Cordis plugin/service
4. 权限统一治理：权限声明进入 Sandbox / Policy，不由 Extension 私自判断
5. 固定目录：前端扩展点 ExtensionUI/，后端扩展点 ExtensionBackend/
```

### 1.3 Extension 类型

Extension 按是否需要主界面 UI 分为两类：

| 类型 | 定义 | 示例 |
|------|------|------|
| UI Extension | 在 Navis 的某个 UI 区域注册入口、渲染器或面板 | 右侧面板区面板、消息渲染器、菜单项 |
| Background Extension | 不注册主界面 UI，只提供后台能力 | Gateway Adapter、MCP Server、Agent Hook |

统一规则：
- `ui` 不是 Extension 必需能力，Background Extension 可以完全没有界面挂载
- 所有 Extension，无论是否有 UI，都必须在 `Settings > Extensions` 中可见、可启停、可卸载

---

## 二、目录结构规范

```
extensions/{extension-id}/
├── extension.json          # 扩展清单（必填）
├── ExtensionUI/            # 前端扩展点：全部前端代码
│   ├── index.tsx           # NavisPlugin 入口（导出 NavisPlugin 实现）
│   ├── locales/            # i18n 语言包（可选）
│   └── scripts/            # 前端胶水脚本（可选）
└── ExtensionBackend/       # 后端扩展点：全部后端代码
    ├── mod.rs              # NavisBackendPlugin 入口（实现 NavisBackendPlugin trait）
    └── ...
```

### 2.1 ExtensionUI 契约

ExtensionUI 必须导出一个 `NavisPlugin` 实现：

```typescript
// ExtensionUI/index.tsx
import { NavisContext, NavisPlugin } from '../../../src/core/context';
import { SlotRenderer } from '../../../src/core/SlotRenderer';

export const MyExtension: NavisPlugin = {
  name: 'my-extension',
  apply: async (ctx: NavisContext) => {
    // 注册插槽
    ctx.registerSlot('root', {
      id: 'my-extension:main',
      priority: 10,
      component: () => <div>My Extension Content</div>
    });
    
    // 注册命令
    ctx.registerCommand('my-extension.open', () => {
      console.log('Open my extension');
    });
  }
};
```

### 2.2 ExtensionBackend 契约

ExtensionBackend 必须实现 `NavisBackendPlugin` trait：

```rust
// ExtensionBackend/mod.rs
use tauri::{AppHandle, command};
use serde_json::Value;
use navis_kernel::kernel::{ExtensionRegistry, NavisBackendPlugin};

pub struct MyExtensionBackend;

impl NavisBackendPlugin for MyExtensionBackend {
    fn name(&self) -> &str {
        "my-extension"
    }

    fn activate(&self, app: &AppHandle, registry: &ExtensionRegistry) -> Result<(), String> {
        // 注册动态 RPC 路由
        let app_handle = app.clone();
        registry.register_route(
            "my-extension:doSomething",
            Box::new(move |_app, payload| {
                // 处理前端调用
                Ok(serde_json::json!({ "result": "done" }))
            })
        );
        Ok(())
    }
}
```

---

## 三、数据模型

### 3.1 extension.json 格式规范

扩展清单文件统一命名为 `extension.json`，位于扩展根目录：

```json
{
  "id": "com.example.my-extension",
  "name": "My Extension",
  "version": "1.0.0",
  "description": "示例扩展",
  "author": "Example",
  "entry": "ExtensionUI/index.tsx",
  "backendEntry": "ExtensionBackend/mod.rs",
  "permissions": {
    "filesystem": ["read:./src/**"],
    "terminal": ["npm", "git"],
    "network": ["https://api.example.com"],
    "resources": { "max_memory_mb": 512, "max_cpu_percent": 50.0, "timeout_ms": 30000 }
  },
  "contributes": {
    "views": [...],
    "commands": [...],
    "menus": [...],
    "slots": [...]
  }
}
```

### 3.2 ExtensionManifest 结构

```rust
struct ExtensionManifest {
    id: String,                    // 扩展稳定 ID
    name: String,                  // 显示名称
    version: String,               // 版本号
    description: String,           // 描述
    author: String,                // 作者
    entry: Option<String>,         // 前端入口（ExtensionUI/index.tsx）
    backendEntry: Option<String>,  // 后端入口（ExtensionBackend/mod.rs）
    permissions: ExtensionPermissions,
    contributes: ExtensionContributes,
}
```

### 3.3 Contributes 声明格式

#### slots（插槽声明）

扩展声明可挂载的插槽位置：

```json
{
  "contributes": {
    "slots": [
      {
        "id": "my-extension:sidebar",
        "name": "Sidebar Panel",
        "target": "root",
        "position": "left",
        "size": "300px"
      }
    ]
  }
}
```

```rust
struct SlotRegistration {
    id: String,                    // 插槽 ID（扩展内唯一）
    name: String,                  // 显示名称
    target: String,                // 目标父插槽（如 "root"、"rightWorkspace"）
    position: SlotPosition,        // 位置：left/right/top/bottom/center
    size: Option<String>,          // 尺寸（如 "300px"、"50%"）
    resizable: Option<bool>,       // 是否可调整大小
    collapsible: Option<bool>,     // 是否可折叠
}

enum SlotPosition {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}
```

#### commands（命令声明）

```rust
struct CommandRegistration {
    id: String,                    // 命令 ID
    label: String,                 // 显示名称
    description: Option<String>,   // 描述
    icon: Option<String>,          // 图标
    category: Option<String>,      // 分类
    when: Option<String>,          // 条件表达式
}
```

#### views（视图声明）

```rust
struct ViewRegistration {
    id: String,                    // 视图 ID
    name: String,                  // 显示名称
    icon: Option<String>,          // 图标
    slot: String,                  // 挂载插槽 ID
    component: String,             // 组件路径（相对于 ExtensionUI/）
    activation_events: Vec<String>, // 激活条件
    allow_close: Option<bool>,     // 是否允许关闭
    default_visible: Option<bool>, // 默认是否可见
}
```

#### menus（菜单声明）

```rust
struct MenuRegistration {
    id: String,                    // 菜单项 ID
    label: String,                 // 显示文本
    target: String,                // 菜单位置
    command: String,               // 关联的命令 ID
    group: Option<String>,         // 分组名
    when: Option<String>,          // 条件表达式
    icon: Option<String>,          // 图标
    position: Option<u32>,         // 排序位置
}
```

---

## 四、NavisPlugin 前端挂载规范

### 4.1 NavisPlugin 接口

```typescript
interface NavisPlugin {
  name: string;                    // 插件名称（唯一标识）
  apply: (ctx: NavisContext) => void | Promise<void>;  // 挂载函数
}
```

### 4.2 挂载流程

```
ExtensionUI/index.tsx 导出 NavisPlugin
       │
       ▼
宿主扫描 extensions/ 目录，发现 NavisPlugin 导出
       │
       ▼
调用 ctx.plugin(plugin) 执行挂载
       │
       ├── plugin.apply(ctx) 执行
       │   ├── ctx.registerSlot() 注册插槽
       │   ├── ctx.registerCommand() 注册命令
       │   └── ctx.provide() 提供服务
       │
       ▼
触发 slot:{target}:updated 事件，SlotRenderer 重新渲染
```

### 4.3 注册插槽示例

```typescript
export const MyPlugin: NavisPlugin = {
  name: 'my-extension',
  apply: async (ctx) => {
    // 在 root 插槽中注册一个子插槽
    ctx.registerSlot('root', {
      id: 'my-extension:main',
      priority: 10,
      component: () => (
        <div class="my-extension-container">
          <h1>My Extension</h1>
          <SlotRenderer ctx={ctx} target="my-extension:content" />
        </div>
      )
    });
    
    // 在子插槽中注册内容
    ctx.registerSlot('my-extension:content', {
      id: 'my-extension:dashboard',
      priority: 100,
      component: () => <Dashboard />
    });
  }
};
```

---

## 五、DynamicRpcHandler 后端挂载规范

### 5.1 ExtensionRegistry 接口

```rust
pub type DynamicRpcHandler = Arc<dyn Fn(&AppHandle, Value) -> Result<Value, String> + Send + Sync>;

pub struct ExtensionRegistry {
    routes: Arc<RwLock<HashMap<String, DynamicRpcHandler>>>,
}

impl ExtensionRegistry {
    pub fn register_route(&self, route: &str, handler: DynamicRpcHandler);
    pub fn dispatch(&self, app: &AppHandle, route: &str, payload: Value) -> Result<Value, String>;
}
```

### 5.2 NavisBackendPlugin trait

```rust
pub trait NavisBackendPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn activate(&self, app: &AppHandle, registry: &ExtensionRegistry) -> Result<(), String>;
}
```

### 5.3 注册路由示例

```rust
pub struct MyBackendPlugin;

impl NavisBackendPlugin for MyBackendPlugin {
    fn name(&self) -> &str {
        "my-extension-backend"
    }

    fn activate(&self, app: &AppHandle, registry: &ExtensionRegistry) -> Result<(), String> {
        // 注册 RPC 路由
        let app_handle = app.clone();
        registry.register_route(
            "my-extension:fetchData",
            Arc::new(move |_app, payload| {
                let data = serde_json::json!({
                    "items": ["item1", "item2"]
                });
                Ok(data)
            })
        );
        
        Ok(())
    }
}
```

### 5.4 前端调用后端路由

```typescript
// 前端通过 Tauri invoke 调用后端路由
import { invoke } from '@tauri-apps/api/core';

const result = await invoke('kernel_dispatch', {
  route: 'my-extension:fetchData',
  payload: { query: 'test' }
});
```

---

## 六、Slot-in-Slot 递归插槽

### 6.1 递归插槽机制

Navis 支持插槽的递归嵌套，扩展可以在任意插槽内开辟子插槽：

```
root (顶级插槽)
├── my-extension:sidebar (扩展注册的子插槽)
│   ├── my-extension:nav (扩展注册的孙插槽)
│   └── my-extension:content (扩展注册的孙插槽)
└── navis-code:main (Navis Code 注册的子插槽)
    ├── navis-code:chat (Navis Code 注册的孙插槽)
    └── navis-code:editor (Navis Code 注册的孙插槽)
```

### 6.2 SlotRenderer 渲染机制

```typescript
// SlotRenderer 自动渲染指定插槽的所有子项
<SlotRenderer ctx={ctx} target="root" />

// SlotRenderer 支持嵌套
<SlotRenderer ctx={ctx} target="my-extension:sidebar">
  <SlotRenderer ctx={ctx} target="my-extension:nav" />
  <SlotRenderer ctx={ctx} target="my-extension:content" />
</SlotRenderer>
```

### 6.3 优先级与排序

插槽内的子项按 `priority` 排序（数字越小越靠前）：

```typescript
ctx.registerSlot('root', {
  id: 'extension-a:panel',
  priority: 10,  // 靠前
  component: () => <PanelA />
});

ctx.registerSlot('root', {
  id: 'extension-b:panel',
  priority: 20,  // 靠后
  component: () => <PanelB />
});
```

---

## 七、扩展生命周期

```
安装（install）
  │
  ├── 解压/复制到扩展目录
  ├── 解析 extension.json
  ├── 校验权限声明
  ├── 注册到扩展表
  │
  ▼
启用（enable）
  │
  ├── 读取并校验 Extension manifest
  ├── 加载 ExtensionUI/index.tsx（NavisPlugin）
  ├── 调用 plugin.apply(ctx) 执行挂载
  ├── 加载 ExtensionBackend/mod.rs（NavisBackendPlugin）
  ├── 调用 backend.activate(app, registry) 注册路由
  │
  ▼
运行中
  │
  ├── 前端：NavisContext 管理插槽、命令、服务
  ├── 后端：ExtensionRegistry 管理 RPC 路由
  └── 权限校验（Sandbox）
  │
  ▼
禁用（disable）
  │
  ├── 前端：清理插槽、命令、服务
  ├── 后端：注销 RPC 路由
  └── 保留配置和数据
  │
  ▼
卸载（uninstall）
  │
  ├── 禁用（如果已启用）
  ├── 删除扩展文件
  └── 清理扩展数据
```

---

## 八、扩展权限与隔离

### 8.1 权限声明

```json
{
  "permissions": {
    "filesystem": ["read:./src/**", "write:./docs/**"],
    "terminal": ["git", "npm"],
    "network": ["https://api.example.com"],
    "ipc": ["agent.cancelTask", "git"],
    "events": ["agent.*", "project.*"],
    "resources": {
      "max_memory_mb": 512,
      "max_cpu_percent": 50,
      "timeout_ms": 30000
    }
  }
}
```

### 8.2 权限粒度

| 权限 | 说明 |
|------|------|
| `filesystem[]` | 允许读取/写入声明范围内的文件 |
| `terminal[]` | 允许执行指定命令 |
| `network[]` | 允许访问指定网络 origin |
| `ipc[]` | 允许调用指定 IPC 命令 |
| `events[]` | 允许订阅宿主事件 |
| `resources` | 扩展资源配额 |

### 8.3 沙箱隔离

- 前端沙箱：Solid.js 组件级隔离，扩展无法访问宿主组件的内部状态
- 后端沙箱：所有系统调用通过 Sandbox 权限检查
- RPC 隔离：扩展只能调用自己注册的路由

---

## 九、事件定义

```typescript
type ExtensionEvents = {
  'extension.installed':    { extensionId: string; version: string }
  'extension.uninstalled':  { extensionId: string }
  'extension.enabled':      { extensionId: string }
  'extension.disabled':     { extensionId: string }
  'extension.updated':      { extensionId: string; fromVersion: string; toVersion: string }
  'extension.error':        { extensionId: string; error: string }
  'extension.slot.registered':   { extensionId: string; slotId: string }
  'extension.slot.unregistered': { extensionId: string; slotId: string }
  'extension.command.registered':   { extensionId: string; commandId: string }
  'extension.command.unregistered': { extensionId: string; commandId: string }
}
```

---

## 十、测试策略

```
单元测试：manifest 解析、NavisPlugin 挂载、NavisContext 服务注入、SlotRenderer 渲染、
         DynamicRpcHandler 路由注册与分发、权限校验、生命周期状态转换
集成测试：扩展安装/卸载、插槽递归嵌套、命令注册与执行、RPC 路由调用、
         沙箱隔离、资源限制
端到端：扩展声明插槽 → 插槽出现 → 扩展声明命令 → 命令可执行 → 
       扩展注册路由 → 前端可调用
```
