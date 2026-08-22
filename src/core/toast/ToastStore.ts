// 全局响应式轻量 Toast 通知系统
import { createSignal } from 'solid-js';

export interface ToastItem {
  id: string;
  type: 'info' | 'success' | 'warning' | 'error';
  message: string;
  duration?: number;
}

const [toasts, setToasts] = createSignal<ToastItem[]>([]);

export const toast = {
  show(message: string, type: ToastItem['type'] = 'info', duration = 3000) {
    const id = `toast-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    const item: ToastItem = { id, type, message, duration };
    setToasts((prev) => [...prev, item]);

    if (duration > 0) {
      setTimeout(() => {
        toast.dismiss(id);
      }, duration);
    }
    return id;
  },
  success(message: string, duration?: number) {
    return toast.show(message, 'success', duration);
  },
  info(message: string, duration?: number) {
    return toast.show(message, 'info', duration);
  },
  warning(message: string, duration?: number) {
    return toast.show(message, 'warning', duration);
  },
  error(message: string, duration?: number) {
    return toast.show(message, 'error', duration);
  },
  dismiss(id: string) {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  },
  list: toasts,
};

export default toast;
