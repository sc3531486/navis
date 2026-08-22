// 通用扩展清单 TypeScript 类型定义：泛型 Schema-Agnostic 设计。
// 宿主仅强制约定 id/name/version 与基础元数据，contributes 允许任意扩展自由声明贡献点。

export interface ExtensionManifest {
  id: string;
  name: string;
  version: string;
  displayName?: string;
  publisher?: string;
  description?: string;
  /** 后端进程入口（.mjs/.js/.cjs/.py 或可执行文件） */
  main?: string;
  /** 前端 UI 入口（打包后 ESM 路径，开发期由 import.meta.glob 装载） */
  ui?: string;
  /** 泛型贡献点集合（可由任意插件注册 handler 解析） */
  contributes?: {
    slots?: SlotContribution[];
    providesSlots?: string[];
    commands?: CommandContribution[];
    [key: string]: unknown;
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