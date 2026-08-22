import { Component, For, Show } from 'solid-js';
import { toast } from './ToastStore';

export const ToastContainer: Component = () => {
  const list = toast.list;

  return (
    <div style="position: fixed; top: 16px; right: 16px; z-index: 10000; display: flex; flex-direction: column; gap: 8px; pointer-events: none;">
      <For each={list()}>
        {(item) => (
          <div
            style={`pointer-events: auto; min-width: 240px; max-width: 380px; padding: 10px 14px; border-radius: 8px; font-size: 13px; font-weight: 500; display: flex; align-items: center; justify-content: space-between; gap: 10px; box-shadow: 0 4px 16px rgba(0,0,0,0.12); transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1); animation: navis-slide-in 0.2s ease-out; ${
              item.type === 'success'
                ? 'background: #f0fdf4; border: 1px solid #bbf7d0; color: #166534;'
                : item.type === 'warning'
                ? 'background: #fffbeb; border: 1px solid #fde68a; color: #92400e;'
                : item.type === 'error'
                ? 'background: #fef2f2; border: 1px solid #fecaca; color: #991b1b;'
                : 'background: #ffffff; border: 1px solid #e7e4dc; color: #2d2b28;'
            }`}
          >
            <div style="display: flex; align-items: center; gap: 8px;">
              <span>
                {item.type === 'success'
                  ? '✅'
                  : item.type === 'warning'
                  ? '⚠️'
                  : item.type === 'error'
                  ? '❌'
                  : 'ℹ️'}
              </span>
              <span>{item.message}</span>
            </div>
            <button
              onClick={() => toast.dismiss(item.id)}
              style="background: transparent; border: none; font-size: 14px; color: inherit; opacity: 0.6; cursor: pointer; padding: 0 2px;"
            >
              ✕
            </button>
          </div>
        )}
      </For>
    </div>
  );
};

export default ToastContainer;
