import type { Component } from 'solid-js';

/**
 * 产品入口的通用契约。
 *
 * 产品只能通过这个契约组合 Navis 宿主，宿主不依赖任何具体产品实现。
 */
export interface ProductDefinition {
  /** 产品稳定标识，必须与产品目录名一致。 */
  id: string;
  /** 产品界面组件。 */
  component: Component;
}

/** 产品清单的构建期描述。 */
export interface ProductManifest {
  id: string;
  name: string;
  version: string;
  entry: string;
  default?: boolean;
}
