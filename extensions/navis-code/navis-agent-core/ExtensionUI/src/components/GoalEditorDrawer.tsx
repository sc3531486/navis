import { Component, createSignal, Show, onMount } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';

const IconTargetSVG = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10" />
    <circle cx="12" cy="12" r="6" />
    <circle cx="12" cy="12" r="2" />
  </svg>
);

const IconCloseSVG = () => (
  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <line x1="18" y1="6" x2="6" y2="18"></line>
    <line x1="6" y1="6" x2="18" y2="18"></line>
  </svg>
);

const IconExpandCornersSVG = () => (
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"></path>
  </svg>
);

const IconSidebarToggleSVG = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
    <line x1="15" y1="3" x2="15" y2="21"></line>
  </svg>
);

const IconUndoSVG = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <path d="M3 7v6h6"></path>
    <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"></path>
  </svg>
);

export const GoalEditorDrawer: Component<{
  ctx: NavisContext;
  open: boolean;
  goalTitle: string;
  onClose: () => void;
  onSave: (newTitle: string) => void;
}> = (props) => {
  const [content, setContent] = createSignal(props.goalTitle || '我们的目标是做一个万物皆扩展的底座');
  const [originalContent, setOriginalContent] = createSignal(props.goalTitle || '我们的目标是做一个万物皆扩展的底座');
  const [lastUpdated, setLastUpdated] = createSignal('刚刚更新');

  onMount(() => {
    setContent(props.goalTitle);
    setOriginalContent(props.goalTitle);
  });

  const handleSave = () => {
    const val = content().trim();
    if (!val) {
      toast.error('目标内容不能为空');
      return;
    }
    props.onSave(val);
    setOriginalContent(val);
    setLastUpdated('刚刚更新');
    toast.success('目标已保存更新');
  };

  const handleUndo = () => {
    setContent(originalContent());
    toast.info('已恢复修改');
  };

  return (
    <Show when={props.open}>
      <div
        id="goal-editor-drawer"
        style="width: 420px; min-width: 360px; max-width: 50%; height: 100%; background: #ffffff; border-left: 1px solid #e5e5e5; display: flex; flex-direction: column; flex-shrink: 0; box-shadow: -2px 0 10px rgba(0,0,0,0.03); z-index: 50;"
      >
        {/* 1. 顶部 Tab 栏与操作工具 (1:1 像素级复刻参考图) */}
        <div
          style="height: 42px; border-bottom: 1px solid #f1f5f9; display: flex; align-items: center; justify-content: space-between; padding: 0 12px; background: #ffffff; flex-shrink: 0;"
        >
          {/* 左侧：编辑目标 Tab + 添加按钮 */}
          <div style="display: flex; align-items: center; gap: 6px;">
            {/* 激活态 Tab */}
            <div
              style="display: flex; align-items: center; gap: 6px; padding: 4px 10px; background: #f4f4f5; border-radius: 6px; font-size: 13px; font-weight: 500; color: #18181b; user-select: none;"
            >
              <span style="display: flex; align-items: center; color: #71717a;">
                <IconTargetSVG />
              </span>
              <span>编辑目标</span>
              <button
                id="goal-editor-close-btn"
                onClick={props.onClose}
                style="background: transparent; border: none; color: #71717a; cursor: pointer; padding: 1px; margin-left: 2px; display: flex; align-items: center; border-radius: 3px;"
                title="关闭编辑区域"
                onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
                onMouseLeave={(e) => (e.currentTarget.style.color = '#71717a')}
              >
                <IconCloseSVG />
              </button>
            </div>

            {/* + 按钮 */}
            <button
              onClick={() => toast.info('已添加新目标草稿')}
              style="width: 22px; height: 22px; border: none; background: transparent; color: #71717a; font-size: 15px; cursor: pointer; display: flex; align-items: center; justify-content: center; border-radius: 4px;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              title="新建目标"
            >
              +
            </button>
          </div>

          {/* 右侧：全屏与侧边栏切换工具 */}
          <div style="display: flex; align-items: center; gap: 4px; color: #71717a;">
            <button
              style="width: 24px; height: 24px; border: none; background: transparent; color: #71717a; cursor: pointer; display: flex; align-items: center; justify-content: center; border-radius: 4px;"
              title="全屏视图"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <IconExpandCornersSVG />
            </button>
            <button
              style="width: 24px; height: 24px; border: none; background: transparent; color: #71717a; cursor: pointer; display: flex; align-items: center; justify-content: center; border-radius: 4px;"
              title="切换侧边栏"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <IconSidebarToggleSVG />
            </button>
          </div>
        </div>

        {/* 2. 中间大文档编辑器主体 */}
        <div style="flex: 1; padding: 20px 24px; display: flex; flex-direction: column; overflow-y: auto;">
          <textarea
            id="goal-editor-textarea"
            value={content()}
            onInput={(e) => setContent(e.currentTarget.value)}
            placeholder="描述要达成的核心目标、里程碑以及交付成果..."
            style="width: 100%; height: 100%; border: none; outline: none; resize: none; font-size: 15px; line-height: 1.75; color: #18181b; background: transparent; font-family: inherit; word-break: break-word;"
          />
        </div>

        {/* 3. 底部更新时间与保存栏 (1:1 像素级复刻参考图) */}
        <div
          style="height: 48px; border-top: 1px solid #f1f5f9; padding: 0 16px; display: flex; align-items: center; justify-content: space-between; background: #ffffff; flex-shrink: 0;"
        >
          {/* 左侧：更新时间 */}
          <span style="font-size: 12px; color: #94a3b8;">
            {lastUpdated()}
          </span>

          {/* 右侧：撤回 + 保存按钮 */}
          <div style="display: flex; align-items: center; gap: 10px;">
            <button
              onClick={handleUndo}
              style="background: transparent; border: none; color: #94a3b8; cursor: pointer; padding: 4px; border-radius: 4px; display: flex; align-items: center; transition: color 0.15s ease;"
              title="撤回修改"
              onMouseEnter={(e) => (e.currentTarget.style.color = '#475569')}
              onMouseLeave={(e) => (e.currentTarget.style.color = '#94a3b8')}
            >
              <IconUndoSVG />
            </button>
            <button
              id="goal-editor-save-btn"
              onClick={handleSave}
              style="padding: 5px 14px; background: #9ca3af; border: none; border-radius: 6px; font-size: 12.5px; font-weight: 500; color: #ffffff; cursor: pointer; transition: all 0.15s ease;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#4b5563')}
              onMouseLeave={(e) => (e.currentTarget.style.background = '#9ca3af')}
            >
              保存
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
};
