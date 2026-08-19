Navis 底层白板框架与业务扩展（navis-code）全量重构实施文档一、 改造目标与设计准则底座纯净白板化（Navis Microkernel Host）：src/ 与 src-tauri/ 仅提供桌面白板容器、Cordis 风格上下文服务总线（NavisContext）、动态命名插槽渲染器（SlotRenderer）和底层动态 RPC 路由分发器。彻底解除固定 6 槽位限制：底座不预设左侧栏、右侧栏、状态栏等物理分区，仅暴露顶级插槽 root（主视口）与 overlay（全局浮层），支持扩展递归开辟子插槽（Slot-in-Slot）。万物皆插件（DeepSeek-Harness Inspired）：所有 Agent、Editor、Terminal、Git、LSP、MCP、Session、Settings 等业务代码全部收敛到 extensions/navis-code/。业务扩展自主在 root 插槽内挂载定制布局（如开发者工作台、柜面双屏、双录向导等）。物理与逻辑强隔离：底层框架禁止直接 import 任何具体业务目录中的内部实现，统一通过标准契约挂载。二、 必须删除的文件清单 (Deletion List)在重构前，请先清理仓库根目录下遗留的旧临时补丁脚本、临时日志与无关文件。1. 根目录临时/陈旧 Python 脚本与日志（直接物理删除）Plaintextanalyze_missing.py
analyze_missing2.py
analyze_needs.py
assess.py
batch_update_imports.py
cargo-test-no-run.log
check_funcs.py
check_impl.py
check_real.py
cheeky-juggling-eagle.md
comment_refs.py
create_stubs.py
create_stubs2.py
create_type_stubs.py
find_stubs.py
fix_all2.py
fix_all_imports.py
fix_app.py
fix_bridge.py
fix_bridge2.py
fix_bridge3.py
fix_crlf.py
fix_dirs.py
fix_doc_comments.py
fix_domains_imports.py
fix_errors.py
fix_errors2.py
fix_final.py
fix_final2.py
fix_final3.py
fix_lifetime.py
fix_lt.py
fix_mods.py
fix_op.py
fix_op2.py
fix_path.py
fix_remaining.py
fix_remaining2.py
fix_removed.py
fix_sandbox.py
fix_sandbox2.py
fix_syntax.py
fix_trait.py
fix_trait2.py
remove_refs.py
remove_stubs.py
replace_domains.py
replace_domains2.py
reset_mods.py
resume.rtf
scan_missing.py
2. src/ 与 src-tauri/ 中散落的旧业务文件（若存在则删除）若 src/components/ 下仍有业务相关的 Chat、Composer、Editor、Terminal、Git 组件，确认已物理迁移至 extensions/navis-code/ 后，从 src/ 中彻底删除。若 src-tauri/src/ 下存在旧业务模块（如 src-tauri/src/agent/、src-tauri/src/terminal/、src-tauri/src/lsp/、src-tauri/src/git/ 等），直接删除，后端微内核仅保留 kernel 模块。三、 底层核心框架（Navis Host）改造内容1. 前端底座上下文引擎：src/core/context.ts改动目标：实现 Cordis 风格的服务依赖注入、事件总线与无物理位置限制的动态插槽系统。TypeScriptimport { ReactNode } from 'react';

export type EventHandler<T = any> = (payload: T) => void | Promise<void>;

export interface SlotItem {
  id: string;
  priority?: number;
  component: () => ReactNode;
}

export interface NavisPlugin {
  name: string;
  apply: (ctx: NavisContext) => void | Promise<void>;
}

export class NavisContext {
  private services = new Map<string, any>();
  private listeners = new Map<string, Set<EventHandler>>();
  private slotRegistry = new Map<string, SlotItem[]>();
  private commands = new Map<string, (args?: any) => void | Promise<void>>();

  // 1. 服务依赖注入 (Cordis Service Provider)
  provide<T>(name: string, service: T): void {
    this.services.set(name, service);
    this.emit(`service:${name}:ready`, service);
  }

  use<T>(name: string): T {
    const service = this.services.get(name);
    if (!service) {
      throw new Error(`[Navis Context] Service "${name}" is not registered.`);
    }
    return service as T;
  }

  has(name: string): boolean {
    return this.services.has(name);
  }

  // 2. 事件总线系统 (Event Bus)
  on(event: string, handler: EventHandler): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(handler);
    return () => {
      this.listeners.get(event)?.delete(handler);
    };
  }

  emit(event: string, payload?: any): void {
    const handlers = this.listeners.get(event);
    if (handlers) {
      handlers.forEach((h) => {
        try {
          h(payload);
        } catch (err) {
          console.error(`[Navis Context] Error in handler for event "${event}":`, err);
        }
      });
    }
  }

  // 3. 动态命名插槽系统（无固定物理位置限制）
  registerSlot(target: string, item: SlotItem): () => void {
    if (!this.slotRegistry.has(target)) {
      this.slotRegistry.set(target, []);
    }
    const list = this.slotRegistry.get(target)!;
    list.push({ ...item, priority: item.priority ?? 100 });
    list.sort((a, b) => (a.priority ?? 100) - (b.priority ?? 100));
    this.emit(`slot:${target}:updated`, list);

    return () => {
      const idx = list.findIndex((s) => s.id === item.id);
      if (idx !== -1) {
        list.splice(idx, 1);
        this.emit(`slot:${target}:updated`, list);
      }
    };
  }

  getSlotItems(target: string): SlotItem[] {
    return this.slotRegistry.get(target) || [];
  }

  // 4. 命令系统
  registerCommand(id: string, handler: (args?: any) => void | Promise<void>): () => void {
    this.commands.set(id, handler);
    this.emit('command:registered', id);
    return () => {
      this.commands.delete(id);
      this.emit('command:unregistered', id);
    };
  }

  executeCommand(id: string, args?: any): Promise<void> | void {
    const cmd = this.commands.get(id);
    if (cmd) {
      return cmd(args);
    }
    console.warn(`[Navis Context] Command "${id}" not found.`);
  }

  // 5. 插件挂载装配管道
  async plugin(plugin: NavisPlugin): Promise<void> {
    console.info(`[Navis Engine] Applying plugin: ${plugin.name}`);
    await plugin.apply(this);
    this.emit(`plugin:${plugin.name}:mounted`);
  }
}

export const rootContext = new NavisContext();
2. 动态插槽渲染组件：src/core/SlotRenderer.tsx改动目标：支持任意命名字符串与嵌套递归插槽的自响应渲染。TypeScriptimport React, { useEffect, useState } from 'react';
import { NavisContext } from './context';

export interface SlotRendererProps {
  ctx: NavisContext;
  target: string;
  className?: string;
  fallback?: React.ReactNode;
}

export const SlotRenderer: React.FC<SlotRendererProps> = ({
  ctx,
  target,
  className,
  fallback
}) => {
  const [, setTick] = useState(0);

  useEffect(() => {
    return ctx.on(`slot:${target}:updated`, () => {
      setTick((t) => t + 1);
    });
  }, [ctx, target]);

  const items = ctx.getSlotItems(target);

  if (items.length === 0) {
    return fallback ? <>{fallback}</> : null;
  }

  return (
    <div className={className} data-navis-slot={target}>
      {items.map((item) => (
        <React.Fragment key={item.id}>{item.component()}</React.Fragment>
      ))}
    </div>
  );
};
3. 白板容器宿主：src/app/WhiteboardShell.tsx改动目标：底座不含具体分栏，仅提供 root 与 overlay 根通道及未挂载扩展时的默认白板占位卡片。TypeScriptimport React from 'react';
import { NavisContext } from '../core/context';
import { SlotRenderer } from '../core/SlotRenderer';
import './WhiteboardShell.css';

interface WhiteboardShellProps {
  ctx: NavisContext;
  brandTitle?: string;
  brandIcon?: string;
}

export const WhiteboardShell: React.FC<WhiteboardShellProps> = ({
  ctx,
  brandTitle = 'Navis Whiteboard',
  brandIcon = '/icons/NAVIS.png'
}) => {
  return (
    <div className="navis-whiteboard-shell">
      {/* 根级插槽：无任何物理槽位预设，完全由业务/布局扩展决定呈现形式 */}
      <SlotRenderer
        ctx={ctx}
        target="root"
        className="navis-root-viewport"
        fallback={
          <div className="navis-empty-canvas">
            <div className="navis-canvas-card">
              <img src={brandIcon} alt="Navis Logo" className="navis-canvas-logo" />
              <h1 className="navis-canvas-title">{brandTitle}</h1>
              <p className="navis-canvas-desc">
                通用应用白板运行时已就绪。当前未挂载业务插件。
              </p>
              <div className="navis-canvas-hints">
                <span>可通过插件向 <code>root</code> 或自定义命名空间动态注入 UI 与业务能力。</span>
              </div>
            </div>
          </div>
        }
      />

      {/* 全局浮层插槽：供弹窗、全局抽屉、悬浮菜单使用 */}
      <SlotRenderer ctx={ctx} target="overlay" className="navis-overlay-layer" />
    </div>
  );
};
4. 白板容器样式：src/app/WhiteboardShell.cssCSS.navis-whiteboard-shell {
  width: 100vw;
  height: 100vh;
  margin: 0;
  padding: 0;
  overflow: hidden;
  background-color: #0f1117;
  color: #e5e7eb;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
  display: flex;
  flex-direction: column;
  position: relative;
}

.navis-root-viewport {
  flex: 1;
  width: 100%;
  height: 100%;
  display: flex;
  overflow: hidden;
}

.navis-empty-canvas {
  flex: 1;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: radial-gradient(circle at 50% 50%, #1e2433 0%, #0f1117 100%);
}

.navis-canvas-card {
  text-align: center;
  max-width: 480px;
  padding: 40px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 16px;
  backdrop-filter: blur(12px);
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
}

.navis-canvas-logo {
  width: 72px;
  height: 72px;
  margin-bottom: 20px;
  filter: drop-shadow(0 4px 12px rgba(99, 102, 241, 0.3));
}

.navis-canvas-title {
  font-size: 24px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: #f9fafb;
}

.navis-canvas-desc {
  font-size: 14px;
  color: #9ca3af;
  line-height: 1.6;
  margin-bottom: 24px;
}

.navis-canvas-hints {
  font-size: 12px;
  color: #6b7280;
  background: rgba(0, 0, 0, 0.2);
  padding: 8px 12px;
  border-radius: 6px;
}

.navis-canvas-hints code {
  color: #818cf8;
  font-family: monospace;
}

.navis-overlay-layer {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 9999;
}
5. 前端应用主入口：src/main.tsx改动目标：在入口处挂载业务插件并初始化白板容器。TypeScriptimport React, { useEffect, useState } from 'react';
import ReactDOM from 'react-dom/client';
import { rootContext } from './core/context';
import { WhiteboardShell } from './app/WhiteboardShell';

// 装载 navis-code 业务扩展
import { NavisCodeExtension } from '../extensions/navis-code/ExtensionUI/src/index';

const AppRoot: React.FC = () => {
  const [ready, setReady] = useState(false);

  useEffect(() => {
    async function bootstrap() {
      // 动态挂载插件（此处可轻松切换为其他扩展，如柜面系统/双录系统）
      await rootContext.plugin(NavisCodeExtension);
      setReady(true);
    }
    bootstrap();
  }, []);

  if (!ready) {
    return null;
  }

  return (
    <WhiteboardShell
      ctx={rootContext}
      brandTitle="Navis Code Studio"
      brandIcon="/icons/NAVIS.png"
    />
  );
};

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <AppRoot />
  </React.StrictMode>
);
6. Rust 微内核 Cargo 配置：src-tauri/Cargo.toml改动目标：移除所有业务 crate，只保留微内核必要依赖。Ini, TOML[package]
name = "navis"
version = "0.1.0"
description = "Navis Desktop Microkernel Host"
edition = "2021"

[lib]
name = "navis_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2.0.0", features = [] }

[dependencies]
tauri = { version = "2.0.0", features = [] }
tauri-plugin-opener = "2.0.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
7. Rust 微内核扩展调度中心：src-tauri/src/kernel/mod.rs改动目标：提供通用的动态 RPC 路由注册与分发机制，微内核不写死业务 Handler。Rustuse serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tauri::AppHandle;

pub type DynamicRpcHandler = Arc<dyn Fn(&AppHandle, Value) -> Result<Value, String> + Send + Sync>;

#[derive(Default, Clone)]
pub struct ExtensionRegistry {
    routes: Arc<RwLock<HashMap<String, DynamicRpcHandler>>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 供后端扩展动态挂载 RPC 路由
    pub fn register_route(&self, route: &str, handler: DynamicRpcHandler) {
        let mut map = self.routes.write().unwrap();
        map.insert(route.to_string(), handler);
        println!("[Navis Kernel] Dynamic route registered: {}", route);
    }

    /// 统一调度执行
    pub fn dispatch(&self, app: &AppHandle, route: &str, payload: Value) -> Result<Value, String> {
        let map = self.routes.read().unwrap();
        if let Some(handler) = map.get(route) {
            handler(app, payload)
        } else {
            Err(format!("[Navis Kernel] Route '{}' not found in registry", route))
        }
    }
}

pub trait NavisBackendPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn activate(&self, app: &AppHandle, registry: &ExtensionRegistry) -> Result<(), String>;
}
8. Rust 微内核启动入口：src-tauri/src/lib.rsRustpub mod kernel;

use kernel::ExtensionRegistry;
use serde_json::Value;
use tauri::{AppHandle, State};

#[tauri::command]
fn navis_dispatch_rpc(
    app: AppHandle,
    registry: State<'_, ExtensionRegistry>,
    route: String,
    payload: Value,
) -> Result<Value, String> {
    registry.dispatch(&app, &route, payload)
}

pub fn run() {
    let registry = ExtensionRegistry::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(registry.clone())
        .invoke_handler(tauri::generate_handler![navis_dispatch_rpc])
        .setup(|_app| {
            println!("[Navis Kernel] Whiteboard host microkernel initialized.");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running navis core app");
}
四、 业务扩展域（navis-code）改造内容1. 扩展元数据清单：extensions/navis-code/extension.jsonJSON{
  "name": "navis-code",
  "version": "1.0.0",
  "displayName": "Navis Code Agent Studio",
  "publisher": "navis",
  "description": "Full-lifecycle Agent, Timeline, Editor, Terminal and AI Platform Extension for Navis.",
  "contributes": {
    "slots": [
      { "id": "navis-code.layout.studio", "target": "root", "priority": 10 },
      { "id": "navis-code.overlay.dialogs", "target": "overlay", "priority": 10 }
    ],
    "commands": [
      { "id": "navis-code.new-session", "title": "New Agent Session" },
      { "id": "navis-code.open-settings", "title": "Settings" }
    ]
  }
}
2. 前端扩展入口与自定义子布局树：extensions/navis-code/ExtensionUI/src/index.tsx改动目标：在 root 插槽中开辟属于 navis-code 的自定义多栏子插槽树，挂载业务组件。TypeScriptimport React from 'react';
import { NavisContext, NavisPlugin } from '../../../../src/core/context';
import { SlotRenderer } from '../../../../src/core/SlotRenderer';

// 引入业务组件
import { Sidebar } from '../../navis-session/ExtensionUI/src/layouts/Sidebar';
import { MainLayout } from './layouts/MainLayout';
import { StatusBar } from '../../navis-agent-core/ExtensionUI/src/layouts/StatusBar';
import { CommandPalette } from './components/CommandPalette/CommandPalette';
import { DialogManager } from './components/Dialog/DialogManager';

import './styles/navis-code-studio.css';

export const NavisCodeExtension: NavisPlugin = {
  name: 'navis-code',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-code] Registering Studio layout into root slot...');

    // 1. 在 root 插槽挂载 navis-code 专属布局树，内部开辟命名子插槽
    ctx.registerSlot('root', {
      id: 'navis-code.layout.root',
      priority: 10,
      component: () => (
        <div className="navis-code-studio-root">
          <div className="navis-code-body-grid">
            {/* 子插槽：左侧会话与文件树 */}
            <SlotRenderer
              ctx={ctx}
              target="navis-code.sidebar.left"
              className="navis-code-sidebar-container"
            />
            {/* 子插槽：主工作区 */}
            <SlotRenderer
              ctx={ctx}
              target="navis-code.viewport.main"
              className="navis-code-main-container"
            />
          </div>
          {/* 底部状态栏插槽 */}
          <SlotRenderer
            ctx={ctx}
            target="navis-code.statusbar"
            className="navis-code-statusbar-container"
          />
        </div>
      )
    });

    // 2. 向自建子插槽注入业务组件
    ctx.registerSlot('navis-code.sidebar.left', {
      id: 'navis-code.component.sidebar',
      priority: 10,
      component: () => <Sidebar />
    });

    ctx.registerSlot('navis-code.viewport.main', {
      id: 'navis-code.component.main-layout',
      priority: 10,
      component: () => <MainLayout />
    });

    ctx.registerSlot('navis-code.statusbar', {
      id: 'navis-code.component.statusbar',
      priority: 10,
      component: () => <StatusBar />
    });

    // 3. 向底座 overlay 注册弹窗与命令面板
    ctx.registerSlot('overlay', {
      id: 'navis-code.overlay.palette',
      priority: 10,
      component: () => (
        <>
          <CommandPalette />
          <DialogManager />
        </>
      )
    });

    // 4. 注册全局命令
    ctx.registerCommand('navis-code.new-session', () => {
      ctx.emit('session:create', { timestamp: Date.now() });
    });
  }
};

export default NavisCodeExtension;
3. 工作室布局样式：extensions/navis-code/ExtensionUI/src/styles/navis-code-studio.cssCSS.navis-code-studio-root {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background-color: #0f1117;
  overflow: hidden;
}

.navis-code-body-grid {
  flex: 1;
  display: flex;
  flex-direction: row;
  overflow: hidden;
}

.navis-code-sidebar-container {
  width: 260px;
  min-width: 200px;
  max-width: 400px;
  height: 100%;
  border-right: 1px solid rgba(255, 255, 255, 0.08);
  background-color: #141721;
}

.navis-code-main-container {
  flex: 1;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background-color: #0f1117;
}

.navis-code-statusbar-container {
  height: 28px;
  min-height: 28px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background-color: #141721;
  display: flex;
  align-items: center;
  padding: 0 8px;
}
4. 后端扩展入口：extensions/navis-code/ExtensionBackend/src/lib.rs改动目标：向微内核动态挂载 Agent、Terminal、Git 等后台服务。Rustuse std::sync::Arc;
use serde_json::{json, Value};
use tauri::AppHandle;

/// 对接 Navis 微内核的扩展注册函数
pub fn register_extension(
    _app: &AppHandle,
    registry: &crate::kernel::ExtensionRegistry,
) -> Result<(), String> {
    // 1. 挂载 Agent 调度服务
    registry.register_route(
        "navis-code:agent:run_turn",
        Arc::new(|_app, payload: Value| {
            println!("[navis-code backend] Agent Turn executed with payload: {:?}", payload);
            Ok(json!({
                "status": "success",
                "taskId": payload.get("taskId").and_then(|v| v.as_str()).unwrap_or("task-default"),
                "state": "running"
            }))
        }),
    );

    // 2. 挂载 Terminal PTY 服务
    registry.register_route(
        "navis-code:terminal:create_pty",
        Arc::new(|_app, payload: Value| {
            println!("[navis-code backend] Terminal instance created: {:?}", payload);
            Ok(json!({
                "ptyId": "pty-001",
                "shell": "powershell"
            }))
        }),
    );

    // 3. 挂载 Git 服务
    registry.register_route(
        "navis-code:git:get_status",
        Arc::new(|_app, _payload: Value| {
            Ok(json!({
                "branch": "main",
                "staged": [],
                "unstaged": []
            }))
        }),
    );

    Ok(())
}
五、 设计文档更新清单 (design/)请同步更新 design/ 目录下的核心架构文档，确保技术规格书与重构后的代码保持 100% 对齐：文档路径重点修订内容design/00-architecture-overview.md  移除固定 6 分区物理布局说明，更新为“微内核白板底座 + Cordis Context + 动态命名插槽树”架构。design/07-extension.md  确立 ExtensionUI 与 ExtensionBackend 契约规范，定义 registerSlot 与动态 RPC 命名空间标准（navis-code:*）。design/34-extension-ui-open-architecture.md  详细说明 Slot-in-Slot 递归开辟机制，阐述如何向 root 挂载完全自定义布局。design/35-whiteboard-container.md  规范纯白板容器在无扩展状态下的品牌展示与占位规范。design/38-deepseek-harness-inspiration.md  说明万物皆插件的实现原理，对比传统 IDE 与自由插槽白板的区别。



但如果要达到「项目 100% 物理合规、编译零报错、业务功能完整闭环跑通」的标准，还需要完成以下 4 个落地的深水区收尾工作。现状差距分析与深度合规清单 (Gap Analysis)改造维度刚才的骨架方案达到 100% 最终合规还需补充的内容1. 物理目录收敛确立了 ExtensionUI 与 ExtensionBackend 根入口历史存量的 10 个子领域（navis-agent-core, navis-session, navis-editor, navis-terminal 等）需要完整聚合导出，消除层级割裂。2. 前端 Import 引用提供了插件化插槽挂载示例检查各子组件内部的相对引用路径（如历史残留的 @/ 别名或 ../../src/ 跨域引用），防止 Vite/TS 报 Module not found。3. 后端微内核与 Cargo 链接提供了动态 RPC 路由调度器（ExtensionRegistry）src-tauri/Cargo.toml 需要将 extensions/navis-code/ExtensionBackend 作为本地 crate 依赖引入，并完成全部具体命令（PTY/Git/Agent）的注册绑定。4. 设计文档全量对齐确定了 design/ 文档的修改原则design/ 下的 38 篇设计文档中，需将涉及“固定 6 分区”和“单体架构”的旧描述彻底修正。彻底符合要求的 4 步落地实现步骤一：子模块代码向 extensions/navis-code 统一聚合目前 extensions/navis-code/ 下存在多个按功能划分的子目录（如 navis-agent-core, navis-session, navis-terminal, navis-editor 等）。为了保持极高内聚性，我们需要在 navis-code/ExtensionUI/src/index.tsx 中做统一总装与聚合导出：  TypeScript// extensions/navis-code/ExtensionUI/src/index.tsx
import React from 'react';
import { NavisContext, NavisPlugin } from '../../../src/core/context';
import { SlotRenderer } from '../../../src/core/SlotRenderer';

// 1. 导入各子领域的视图组件
import { Sidebar as SessionSidebar } from '../navis-session/ExtensionUI/src/layouts/Sidebar';
import { MainLayout as EditorMainLayout } from './layouts/MainLayout';
import { StatusBar as AgentStatusBar } from '../navis-agent-core/ExtensionUI/src/layouts/StatusBar';
import { TerminalPanel } from '../navis-terminal/ExtensionUI/src/components/Terminal/TerminalPanel';
import { CommandPalette } from './components/CommandPalette/CommandPalette';
import { DialogManager } from './components/Dialog/DialogManager';

import './styles/navis-code-studio.css';

export const NavisCodeExtension: NavisPlugin = {
  name: 'navis-code',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-code] Registering full agent studio into root slot...');

    // 2. 接管 root 插槽，自定义工作台布局树（左侧栏 + 主编辑/对话区 + 底部终端 + 状态栏）
    ctx.registerSlot('root', {
      id: 'navis-code.layout.studio',
      priority: 10,
      component: () => (
        <div className="navis-code-studio-root">
          <div className="navis-code-body-grid">
            <SlotRenderer
              ctx={ctx}
              target="navis-code.sidebar"
              className="navis-code-sidebar-container"
            />
            <div className="navis-code-main-column">
              <SlotRenderer
                ctx={ctx}
                target="navis-code.viewport"
                className="navis-code-main-viewport"
              />
              <SlotRenderer
                ctx={ctx}
                target="navis-code.bottom-panel"
                className="navis-code-bottom-panel"
              />
            </div>
          </div>
          <SlotRenderer
            ctx={ctx}
            target="navis-code.statusbar"
            className="navis-code-statusbar-container"
          />
        </div>
      )
    });

    // 3. 将各子模块组件挂载至对应的命名子插槽
    ctx.registerSlot('navis-code.sidebar', {
      id: 'navis-code.sidebar.sessions',
      priority: 10,
      component: () => <SessionSidebar />
    });

    ctx.registerSlot('navis-code.viewport', {
      id: 'navis-code.viewport.main',
      priority: 10,
      component: () => <EditorMainLayout />
    });

    ctx.registerSlot('navis-code.bottom-panel', {
      id: 'navis-code.panel.terminal',
      priority: 10,
      component: () => <TerminalPanel />
    });

    ctx.registerSlot('navis-code.statusbar', {
      id: 'navis-code.statusbar.agent',
      priority: 10,
      component: () => <AgentStatusBar />
    });

    // 4. 挂载全局浮层（命令面板、弹窗管理器）
    ctx.registerSlot('overlay', {
      id: 'navis-code.overlay.manager',
      priority: 10,
      component: () => (
        <>
          <CommandPalette />
          <DialogManager />
        </>
      )
    });

    // 5. 注册全局快捷命令
    ctx.registerCommand('navis-code.new-session', () => {
      ctx.emit('session:create', { timestamp: Date.now() });
    });
  }
};

export default NavisCodeExtension;
步骤二：Rust 后端工程链接与动态 RPC 真实对接后端微内核需要与 navis-code 后端代码形成依赖与装配关系。1. 修改 src-tauri/Cargo.toml 引入扩展 CrateIni, TOML# src-tauri/Cargo.toml
[package]
name = "navis"
version = "0.1.0"
description = "Navis Desktop Microkernel Host"
edition = "2021"

[lib]
name = "navis_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[dependencies]
tauri = { version = "2.0.0", features = [] }
tauri-plugin-opener = "2.0.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

# 引入业务扩展后端 (路径引用)
navis-code-backend = { path = "../extensions/navis-code/ExtensionBackend" }
2. 在 src-tauri/src/lib.rs 中激活扩展Rust// src-tauri/src/lib.rs
pub mod kernel;

use kernel::ExtensionRegistry;
use serde_json::Value;
use tauri::{AppHandle, State};

#[tauri::command]
fn navis_dispatch_rpc(
    app: AppHandle,
    registry: State<'_, ExtensionRegistry>,
    route: String,
    payload: Value,
) -> Result<Value, String> {
    registry.dispatch(&app, &route, payload)
}

pub fn run() {
    let registry = ExtensionRegistry::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(registry.clone())
        .invoke_handler(tauri::generate_handler![navis_dispatch_rpc])
        .setup(|app| {
            println!("[Navis Kernel] Host microkernel initialized.");

            // 动态激活 navis-code 扩展后端
            navis_code_backend::register_extension(app.handle(), &registry)
                .expect("Failed to activate navis-code extension");

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running navis core app");
}
步骤三：消除废弃脚本，彻底净化根目录执行以下命令清理历史补丁脚本与无用日志：