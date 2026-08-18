import { Component, For, Show, createSignal, onCleanup, onMount } from 'solid-js';
import type { MenuActionItem } from '@/stores/menu';
import rightPanelPreviewIconUrl from '@project-ext/assets/right-panel-preview.svg';
import rightPanelDiffIconUrl from '@project-ext/assets/right-panel-diff.svg';
import rightPanelTerminalIconUrl from '@project-ext/assets/right-panel-terminal.svg';
import rightPanelBackgroundTasksIconUrl from '@project-ext/assets/right-panel-background-tasks.svg';
import rightPanelPlanIconUrl from '@project-ext/assets/right-panel-plan.svg';
import rightPanelDesignIconUrl from '@project-ext/assets/right-panel-design.svg';

/** 图标名称 → URL 映射 */
const ICON_MAP: Record<string, string> = {
  preview: rightPanelPreviewIconUrl,
  diff: rightPanelDiffIconUrl,
  terminal: rightPanelTerminalIconUrl,
  'background-tasks': rightPanelBackgroundTasksIconUrl,
  plan: rightPanelPlanIconUrl,
  design: rightPanelDesignIconUrl,
};

interface FloatingMenuProps {
  items: MenuActionItem[];
  triggerLabel: string;
  placement?: 'above' | 'below';
  align?: 'left' | 'right';
  width?: number;
  submenuWidth?: number;
  selectedCommands?: string[];
  showSourceLabel?: boolean;
  getSubmenuItems?: (item: MenuActionItem) => MenuActionItem[];
  onSelect: (item: MenuActionItem) => void;
}

const FloatingMenu: Component<FloatingMenuProps> = (props) => {
  const placement = () => props.placement ?? 'below';
  const align = () => props.align ?? 'left';
  const width = () => props.width ?? 180;
  const submenuWidth = () => props.submenuWidth ?? width();
  const [position, setPosition] = createSignal({ left: 0, top: 0 });
  const [submenu, setSubmenu] = createSignal<{
    parentId: string;
    label: string;
    items: MenuActionItem[];
    left: number;
    top: number;
  } | null>(null);
  let menuRef: HTMLDivElement | undefined;

  const updatePosition = () => {
    const anchor = menuRef?.parentElement;
    if (!anchor || !menuRef) return;

    const anchorRect = anchor.getBoundingClientRect();
    const menuRect = menuRef.getBoundingClientRect();
    const gap = 5;
    const nextLeft =
      align() === 'right'
        ? anchorRect.right - width()
        : anchorRect.left;
    const nextTop =
      placement() === 'above'
        ? anchorRect.top - menuRect.height - gap
        : anchorRect.bottom + gap;

    setPosition({
      left: Math.max(8, Math.min(nextLeft, window.innerWidth - width() - 8)),
      top: Math.max(8, Math.min(nextTop, window.innerHeight - menuRect.height - 8)),
    });
  };

  const showSubmenu = (item: MenuActionItem, target: HTMLElement) => {
    const items = props.getSubmenuItems?.(item) ?? [];
    if (items.length === 0) {
      setSubmenu(null);
      return;
    }

    const rect = target.getBoundingClientRect();
    setSubmenu({
      parentId: item.id,
      label: item.label,
      items,
      left: Math.max(8, Math.min(rect.right + 6, window.innerWidth - submenuWidth() - 8)),
      top: Math.max(8, Math.min(rect.top - 20, window.innerHeight - (items.length * 32 + 28) - 8)),
    });
  };

  const shouldSelect = (item: MenuActionItem): boolean =>
    (props.getSubmenuItems?.(item) ?? []).length === 0;

  const itemHasSubmenu = (item: MenuActionItem): boolean =>
    (props.getSubmenuItems?.(item) ?? []).length > 0;

  const itemSelected = (item: MenuActionItem): boolean =>
    props.selectedCommands?.includes(item.command) ?? false;

  const trailingText = (item: MenuActionItem): string | undefined =>
    item.shortcut ?? (itemHasSubmenu(item) ? '›' : undefined);

  const renderMenuItem = (item: MenuActionItem, onHover?: (target: HTMLElement) => void) => (
    <button
      type="button"
      role="menuitem"
      class={`navis-floating-menu-item ${item.risk === 'high' ? 'is-danger' : ''} ${
        itemSelected(item) || submenu()?.parentId === item.id ? 'is-selected' : ''
      }`}
      onMouseEnter={(event) => onHover?.(event.currentTarget)}
      onClick={(event) => {
        if (shouldSelect(item)) {
          props.onSelect(item);
        } else {
          showSubmenu(item, event.currentTarget);
        }
      }}
    >
      <Show when={item.icon && ICON_MAP[item.icon!]}>
        <img src={ICON_MAP[item.icon!]} alt="" class="mr-2 h-4 w-4 shrink-0 opacity-60" aria-hidden="true" />
      </Show>
      <span class="min-w-0 flex-1 truncate">{item.label}</span>
      <Show when={trailingText(item) && !itemSelected(item)}>
        <span class="navis-floating-menu-shortcut">{trailingText(item)}</span>
      </Show>
      <Show when={itemSelected(item)}>
        <span class="navis-floating-menu-check" aria-hidden="true" />
      </Show>
    </button>
  );

  onMount(() => {
    requestAnimationFrame(updatePosition);
    window.addEventListener('resize', updatePosition);
    window.addEventListener('scroll', updatePosition, true);
  });

  onCleanup(() => {
    window.removeEventListener('resize', updatePosition);
    window.removeEventListener('scroll', updatePosition, true);
  });

  return (
    <div
      ref={menuRef}
      class={`navis-floating-menu ${placement() === 'above' ? 'is-above' : 'is-below'} ${
        align() === 'right' ? 'is-right' : 'is-left'
      }`}
      role="menu"
      style={{ width: `${width()}px`, left: `${position().left}px`, top: `${position().top}px` }}
    >
      <Show when={props.showSourceLabel ?? true}>
        <div class="navis-floating-menu-source">{props.triggerLabel}</div>
      </Show>
      <For each={props.items}>
        {(item, index) => (
          <>
            <Show when={index() > 0 && props.items[index() - 1]?.group !== item.group}>
              <div class="navis-floating-menu-divider" role="separator" />
            </Show>
            {renderMenuItem(item, (target) => showSubmenu(item, target))}
          </>
        )}
      </For>
      <Show when={submenu()}>
        {(activeSubmenu) => (
          <div
            class="navis-floating-menu navis-floating-submenu"
            role="menu"
            style={{
              width: `${submenuWidth()}px`,
              left: `${activeSubmenu().left}px`,
              top: `${activeSubmenu().top}px`,
            }}
          >
            <Show when={props.showSourceLabel ?? true}>
              <div class="navis-floating-menu-source">{activeSubmenu().label}</div>
            </Show>
            <For each={activeSubmenu().items}>
              {(item, index) => (
                <>
                  <Show when={index() > 0 && activeSubmenu().items[index() - 1]?.group !== item.group}>
                    <div class="navis-floating-menu-divider" role="separator" />
                  </Show>
                  {renderMenuItem(item)}
                </>
              )}
            </For>
          </div>
        )}
      </Show>
    </div>
  );
};

export default FloatingMenu;
