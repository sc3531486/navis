import { createStore } from 'solid-js/store';
import type { UiExtensionView } from '@/lib/extension-ui';

export interface ExtensionDialogOptions {
  width?: number;
  height?: number;
  x?: number;
  y?: number;
  modal?: boolean;
}
/** Parse declarative OpenDialog options without accepting ambiguous values. */
export function parseExtensionDialogOptions(input: unknown): ExtensionDialogOptions {
  if (!input || typeof input !== 'object') return {};
  const record = input as Record<string, unknown>;
  const options: ExtensionDialogOptions = {};
  const size = typeof record.size === 'string' ? record.size.trim().match(/^(\d+)x(\d+)$/i) : null;
  if (size) {
    options.width = Number(size[1]);
    options.height = Number(size[2]);
  }
  if (typeof record.width === 'number' && Number.isFinite(record.width)) options.width = record.width;
  if (typeof record.height === 'number' && Number.isFinite(record.height)) options.height = record.height;
  if (typeof record.modal === 'boolean') options.modal = record.modal;

  const position = typeof record.position === 'string' ? record.position.trim() : '';
  if (position.toLowerCase() !== 'center') {
    const coordinates = position.match(/^(-?\d+)\s*,\s*(-?\d+)$/);
    if (coordinates) {
      options.x = Number(coordinates[1]);
      options.y = Number(coordinates[2]);
    }
  }
  return options;
}

export interface ExtensionDialogEntry {
  id: string;
  title: string;
  view: UiExtensionView;
  x: number;
  y: number;
  width: number;
  height: number;
  modal: boolean;
}

const DEFAULT_WIDTH = 560;
const DEFAULT_HEIGHT = 420;
let sequence = 0;

const [extensionDialogState, setExtensionDialogState] = createStore<{ dialogs: ExtensionDialogEntry[] }>({
  dialogs: [],
});

export { extensionDialogState };

function viewportSize(): { width: number; height: number } {
  return {
    width: typeof window === 'undefined' ? 1280 : window.innerWidth,
    height: typeof window === 'undefined' ? 800 : window.innerHeight,
  };
}

function defaultPosition(width: number, height: number): { x: number; y: number } {
  const viewport = viewportSize();
  const offset = (sequence % 6) * 28;
  return {
    x: Math.max(24, Math.round((viewport.width - width) / 2) + offset),
    y: Math.max(48, Math.round((viewport.height - height) / 2) + offset),
  };
}

export function openExtensionDialog(view: UiExtensionView, options: ExtensionDialogOptions = {}): string {
  const width = Math.max(320, options.width ?? DEFAULT_WIDTH);
  const height = Math.max(220, options.height ?? DEFAULT_HEIGHT);
  const position = defaultPosition(width, height);
  const id = `extension-dialog-${++sequence}`;
  setExtensionDialogState('dialogs', (dialogs) => [
    ...dialogs,
    {
      id,
      title: view.name,
      view,
      x: options.x ?? position.x,
      y: options.y ?? position.y,
      width,
      height,
      modal: options.modal ?? false,
    },
  ]);
  return id;
}

export function closeExtensionDialog(id: string): void {
  setExtensionDialogState('dialogs', (dialogs) => dialogs.filter((dialog) => dialog.id !== id));
}

export function closeExtensionDialogsForExtension(extensionId: string): void {
  setExtensionDialogState('dialogs', (dialogs) => dialogs.filter((dialog) => dialog.view.extensionId !== extensionId));
}

export function focusExtensionDialog(id: string): void {
  setExtensionDialogState('dialogs', (dialogs) => {
    const target = dialogs.find((dialog) => dialog.id === id);
    if (!target || dialogs[dialogs.length - 1]?.id === id) return dialogs;
    return [...dialogs.filter((dialog) => dialog.id !== id), target];
  });
}

export function updateExtensionDialog(
  id: string,
  patch: Partial<Pick<ExtensionDialogEntry, 'x' | 'y' | 'width' | 'height'>>,
): void {
  setExtensionDialogState('dialogs', (dialogs) =>
    dialogs.map((dialog) => dialog.id === id ? { ...dialog, ...patch } : dialog),
  );
}

export function extensionDialogForView(extensionId: string, viewId: string): ExtensionDialogEntry | undefined {
  return extensionDialogState.dialogs.find(
    (dialog) => dialog.view.extensionId === extensionId && dialog.view.viewId === viewId,
  );
}
