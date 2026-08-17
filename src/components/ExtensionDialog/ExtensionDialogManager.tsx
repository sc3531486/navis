import { Component, For, Show } from 'solid-js';
import CloseIcon from '../Icon/CloseIcon';
import HostViewRenderer from '../HostView/HostViewRenderer';
import {
  closeExtensionDialog,
  extensionDialogState,
  focusExtensionDialog,
  updateExtensionDialog,
} from './store';

function beginDrag(event: PointerEvent, id: string): void {
  if (event.button !== 0) return;
  const dialog = extensionDialogState.dialogs.find((entry) => entry.id === id);
  if (!dialog) return;
  focusExtensionDialog(id);
  const startX = event.clientX;
  const startY = event.clientY;
  const originX = dialog.x;
  const originY = dialog.y;
  const move = (moveEvent: PointerEvent) => {
    updateExtensionDialog(id, {
      x: Math.max(8, originX + moveEvent.clientX - startX),
      y: Math.max(30, originY + moveEvent.clientY - startY),
    });
  };
  const end = () => {
    document.removeEventListener('pointermove', move);
    document.removeEventListener('pointerup', end);
  };
  document.addEventListener('pointermove', move);
  document.addEventListener('pointerup', end);
}

function beginResize(event: PointerEvent, id: string): void {
  if (event.button !== 0) return;
  const dialog = extensionDialogState.dialogs.find((entry) => entry.id === id);
  if (!dialog) return;
  focusExtensionDialog(id);
  const startX = event.clientX;
  const startY = event.clientY;
  const originWidth = dialog.width;
  const originHeight = dialog.height;
  const move = (moveEvent: PointerEvent) => {
    updateExtensionDialog(id, {
      width: Math.max(320, originWidth + moveEvent.clientX - startX),
      height: Math.max(220, originHeight + moveEvent.clientY - startY),
    });
  };
  const end = () => {
    document.removeEventListener('pointermove', move);
    document.removeEventListener('pointerup', end);
  };
  document.addEventListener('pointermove', move);
  document.addEventListener('pointerup', end);
}

const ExtensionDialogManager: Component = () => (
  <div class="pointer-events-none fixed inset-0 z-[1000]" aria-label="Extension dialogs">
    <For each={extensionDialogState.dialogs}>
      {(dialog, index) => (
        <>
          <Show when={dialog.modal}>
            <div class="pointer-events-auto fixed inset-0 bg-black/20" onClick={() => dialog.view.allowClose && closeExtensionDialog(dialog.id)} />
          </Show>
          <section
            class="pointer-events-auto absolute flex min-h-0 flex-col overflow-hidden rounded-lg border border-[#c8c8c8] bg-white shadow-2xl"
            style={{
              left: `${dialog.x}px`,
              top: `${dialog.y}px`,
              width: `${dialog.width}px`,
              height: `${dialog.height}px`,
              'z-index': String(1000 + index()),
            }}
            role="dialog"
            aria-label={dialog.title}
            onPointerDown={() => focusExtensionDialog(dialog.id)}
          >
            <header
              class="flex h-9 shrink-0 cursor-move items-center justify-between border-b border-[#e2e2e2] bg-[#f6f6f6] px-3 text-xs font-medium text-[#333]"
              onPointerDown={(event) => beginDrag(event, dialog.id)}
            >
              <span class="truncate">{dialog.title}</span>
              <Show when={dialog.view.allowClose}>
                <button
                  type="button"
                  class="ml-2 rounded p-1 text-[#666] hover:bg-[#e5e5e5]"
                  aria-label={`Close ${dialog.title}`}
                  onClick={() => closeExtensionDialog(dialog.id)}
                >
                  <CloseIcon />
                </button>
              </Show>
            </header>
            <div class="min-h-0 flex-1 overflow-auto">
              <HostViewRenderer view={dialog.view} surface="dialog" />
            </div>
            <div
              class="absolute bottom-0 right-0 h-3 w-3 cursor-se-resize"
              aria-hidden="true"
              onPointerDown={(event) => beginResize(event, dialog.id)}
            />
          </section>
        </>
      )}
    </For>
  </div>
);

export default ExtensionDialogManager;
