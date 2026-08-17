/**
 * Hotkey 全局快捷键模块 - 本地通知器
 *
 * 仅用于 Hotkey 模块内部的前端状态通知，对应设计文档 §6 通知定义。
 * 这不是应用事件总线，也不和后端 Kernel EventBus 对等；业务事实通知仍
 * 只通过后端 Kernel EventBus 的 UI 投影进入前端。
 *
 * 支持的事件：
 * - hotkey.triggered: 快捷键被触发
 * - hotkey.registered: 快捷键注册成功
 * - hotkey.unregistered: 快捷键被注销
 * - hotkey.conflict: 检测到冲突
 *
 * 设计说明：
 * - 使用 Map 存储监听器，支持同一通知的多个监听器
 * - 每个 on() 调用返回 unsubscribe 函数，便于 Solid.js 的 onCleanup 清理
 * - off() 方法支持移除特定监听器或某通知的所有监听器
 */

import type { HotkeyEventPayloads } from './types';

/** 本地通知回调函数类型 */
type HotkeyNotificationCallback<T = unknown> = (payload: T) => void;

/**
 * Hotkey 专用的类型安全本地通知器
 *
 * 事件名和负载类型固定为 HotkeyEventPayloads，避免这个工具被复用成
 * 前端平行 EventBus。
 */
export class HotkeyNotifier {
  /**
   * 通知监听器存储
   * 键为通知名称，值为该通知的回调函数集合
   */
  private listeners = new Map<string, Set<HotkeyNotificationCallback>>();

  /**
   * 订阅本地通知
   *
   * @param event 通知名称
   * @param callback 通知回调函数
   * @returns 取消订阅的函数（unsubscribe）
   */
  on<K extends keyof HotkeyEventPayloads & string>(
    event: K,
    callback: HotkeyNotificationCallback<HotkeyEventPayloads[K]>,
  ): () => void {
    // 获取或创建该通知的监听器集合
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(callback as HotkeyNotificationCallback);

    // 返回 unsubscribe 函数，便于在 Solid.js onCleanup 中使用
    return () => {
      this.off(event, callback);
    };
  }

  /**
   * 取消订阅本地通知
   *
   * @param event 通知名称
   * @param callback 要移除的回调函数（可选，不传则移除该事件所有监听器）
   */
  off<K extends keyof HotkeyEventPayloads & string>(
    event: K,
    callback?: HotkeyNotificationCallback<HotkeyEventPayloads[K]>,
  ): void {
    const eventListeners = this.listeners.get(event);
    if (!eventListeners) return;

    if (callback) {
      // 移除特定监听器
      eventListeners.delete(callback as HotkeyNotificationCallback);
      // 如果该通知已无监听器，清理 Map 条目
      if (eventListeners.size === 0) {
        this.listeners.delete(event);
      }
    } else {
      // 移除该通知的所有监听器
      this.listeners.delete(event);
    }
  }

  /**
   * 发布本地通知，通知所有已订阅的监听器
   *
   * @param event 通知名称
   * @param payload 通知负载数据
   */
  notify<K extends keyof HotkeyEventPayloads & string>(
    event: K,
    payload: HotkeyEventPayloads[K],
  ): void {
    const eventListeners = this.listeners.get(event);
    if (!eventListeners) return;

    // 遍历调用所有监听器
    for (const callback of eventListeners) {
      try {
        (callback as HotkeyNotificationCallback<HotkeyEventPayloads[K]>)(payload);
      } catch (error) {
        // 防止单个监听器的错误影响其他监听器
        console.error(`[Hotkey] 通知监听器执行出错 (${event}):`, error);
      }
    }
  }

  /**
   * 移除所有事件监听器
   *
   * 用于模块销毁时的清理。
   */
  clear(): void {
    this.listeners.clear();
  }
}

/**
 * 创建快捷键模块专用的本地通知器实例。
 */
export function createHotkeyNotifier(): HotkeyNotifier {
  return new HotkeyNotifier();
}
