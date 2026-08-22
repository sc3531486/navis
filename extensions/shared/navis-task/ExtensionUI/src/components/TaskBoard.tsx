import { Component, createSignal, For } from 'solid-js';
import type { NavisContext } from '@/core/context';

interface TaskBoardProps {
  ctx: NavisContext;
}

interface TaskItem {
  id: string;
  title: string;
  status: 'pending' | 'in-progress' | 'completed';
}

export const TaskBoard: Component<TaskBoardProps> = (props) => {
  const [tasks] = createSignal<TaskItem[]>([
    { id: '1', title: '万物皆扩展架构物理迁移', status: 'completed' },
    { id: '2', title: 'Cordis 上下文原语重构', status: 'completed' },
    { id: '3', title: 'Navis Code 扩展套件实现', status: 'in-progress' },
    { id: '4', title: '边界与构建全量验证', status: 'pending' },
  ]);

  return (
    <div style="padding: 12px; display: flex; flex-direction: column; gap: 8px;">
      <div style="font-size: 11px; font-weight: 700; color: #888; letter-spacing: 0.5px;">TASKS & PLANS</div>
      <div style="display: flex; flex-direction: column; gap: 6px;">
        <For each={tasks()}>
          {(t) => (
            <div style="display: flex; align-items: center; gap: 8px; font-size: 12px; color: #ccc;">
              <span>{t.status === 'completed' ? '✅' : t.status === 'in-progress' ? '🔄' : '⏳'}</span>
              <span style={t.status === 'completed' ? 'text-decoration: line-through; opacity: 0.6;' : ''}>
                {t.title}
              </span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
};
