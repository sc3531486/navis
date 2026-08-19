// 统一扩展协议 TypeScript 类型：与 src-tauri/src/kernel/manifest.rs 对齐。
export interface ExtensionManifest {
  id: string;
  name: string;
  version: string;
  /** 后端进程入口（.mjs/.js/.cjs/.py 或可执行文件） */
  main?: string;
  /** 前端 UI 入口（打包后 ESM 路径，开发期由 import.meta.glob 装载） */
  ui?: string;
  contributes: {
    /** 声明挂载到宿主插槽的条目（component 为具名组件，由插件组件注册表解析） */
    slots?: SlotContribution[];
    /** 扩展向系统发布的新插槽名（供其他扩展挂载） */
    providesSlots?: string[];
    /** 传统命令贡献点 */
    commands?: CommandContribution[];
    /** 工具能力声明（供统一工具网关注册） */
    tools?: ToolContribution[];
    /** Agent 管线拦截钩子声明 */
    pipelineHooks?: PipelineHookContribution[];
  };
  permissions?: Record<string, unknown>;
}

export interface SlotContribution {
  id: string;
  target: string;
  component?: string;
  priority?: number;
}

export interface CommandContribution {
  id: string;
  title: string;
}

export interface ToolContribution {
  name: string;
  description?: string;
  parameters?: any;
}

export interface PipelineHookContribution {
  hook: string;
  handler: string;
}