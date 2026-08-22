import { Component, For, Show, createSignal } from 'solid-js';

export interface MilestoneItem {
  id: string;
  title: string;
  desc?: string;
  status: 'pending' | 'running' | 'completed';
  effort: 'small' | 'medium' | 'large'; // S (15m), M (45m), L (2h)
  progress?: number;
}

export const MilestoneRoadmap: Component<{
  milestones: MilestoneItem[];
  onClose?: () => void;
}> = (props) => {
  const [hoveredNodeId, setHoveredNodeId] = createSignal<string | null>(null);

  const completedCount = () => props.milestones.filter((m) => m.status === 'completed').length;
  const totalCount = () => props.milestones.length;

  const getEffortGap = (effort: 'small' | 'medium' | 'large') => {
    switch (effort) {
      case 'small':
        return 40; // 较小工作量
      case 'medium':
        return 75; // 中等工作量
      case 'large':
        return 120; // 较大工作量
      default:
        return 60;
    }
  };

  const getEffortBadge = (effort: 'small' | 'medium' | 'large') => {
    switch (effort) {
      case 'small':
        return { label: 'S · 15m', color: '#0ea5e9', bg: '#f0f9ff' };
      case 'medium':
        return { label: 'M · 45m', color: '#f59e0b', bg: '#fffbeb' };
      case 'large':
        return { label: 'L · 2h', color: '#8b5cf6', bg: '#f5f3ff' };
    }
  };

  return (
    <div
      id="milestone-roadmap-card"
      style="border-bottom: 1px solid #f1f5f9; background: #fafafa; border-radius: 13px 13px 0 0; padding: 10px 16px 12px; display: flex; flex-direction: column; gap: 10px; position: relative; user-select: none;"
    >
      <style>{`
        @keyframes spinRing {
          0% { transform: rotate(0deg); }
          100% { transform: rotate(360deg); }
        }
        @keyframes pulseGlow {
          0%, 100% { box-shadow: 0 0 0 0 rgba(34, 197, 94, 0.4); }
          50% { box-shadow: 0 0 0 6px rgba(34, 197, 94, 0); }
        }
      `}</style>

      {/* 1. 里程碑概览标题栏 */}
      <div style="display: flex; align-items: center; justify-content: space-between;">
        <div style="display: flex; align-items: center; gap: 8px;">
          <div
            style="width: 20px; height: 20px; border-radius: 5px; background: #ecfdf5; color: #16a34a; display: flex; align-items: center; justify-content: center; font-size: 11px; font-weight: 700;"
          >
            ✦
          </div>
          <span style="font-size: 12.5px; font-weight: 600; color: #18181b;">
            执行计划里程碑
          </span>
          <span style="font-size: 11.5px; color: #64748b; font-weight: 500;">
            ({completedCount()}/{totalCount()} 已完成)
          </span>
        </div>

        <Show when={props.onClose}>
          <button
            onClick={props.onClose}
            style="background: transparent; border: none; color: #94a3b8; cursor: pointer; font-size: 14px; padding: 2px 6px; border-radius: 4px;"
            title="隐藏里程碑"
            onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
            onMouseLeave={(e) => (e.currentTarget.style.color = '#94a3b8')}
          >
            ✕
          </button>
        </Show>
      </div>

      {/* 2. 动态自适应工作量节点流程图 */}
      <div
        style="display: flex; align-items: center; overflow-x: auto; padding: 6px 4px 10px; scrollbar-width: none;"
      >
        <For each={props.milestones}>
          {(m, idx) => {
            const isFirst = () => idx() === 0;
            const prevNode = () => (idx() > 0 ? props.milestones[idx() - 1] : null);
            const isPrevCompleted = () => prevNode()?.status === 'completed';
            const gapWidth = () => getEffortGap(m.effort);
            const effortBadge = () => getEffortBadge(m.effort);

            return (
              <div style="display: flex; align-items: center; flex-shrink: 0;">
                {/* 节点前置连接线 (与工作量成比例) */}
                <Show when={!isFirst()}>
                  <div
                    style={`height: 3px; width: ${gapWidth()}px; transition: all 0.3s ease; border-radius: 2px; ${
                      m.status === 'completed'
                        ? 'background: #22c55e;'
                        : m.status === 'running'
                        ? 'background: linear-gradient(90deg, #22c55e 0%, #86efac 100%);'
                        : isPrevCompleted()
                        ? 'background: #cbd5e1;'
                        : 'background: #e2e8f0;'
                    }`}
                  />
                </Show>

                {/* 里程碑节点主体 */}
                <div
                  style="display: flex; flex-direction: column; align-items: center; gap: 6px; position: relative; cursor: pointer;"
                  onMouseEnter={() => setHoveredNodeId(m.id)}
                  onMouseLeave={() => setHoveredNodeId(null)}
                >
                  {/* 节点图标 / 状态圆环 */}
                  <div style="position: relative; width: 28px; height: 28px; display: flex; align-items: center; justify-content: center;">
                    {/* A. 正在执行态 (绿色旋转外圈 + 发光) */}
                    <Show when={m.status === 'running'}>
                      <div
                        style="position: absolute; inset: -2px; border: 2.5px solid transparent; border-top-color: #16a34a; border-right-color: #22c55e; border-radius: 50%; animation: spinRing 0.9s linear infinite;"
                      />
                      <div
                        style="width: 22px; height: 22px; border-radius: 50%; background: #ecfdf5; border: 1.5px solid #22c55e; display: flex; align-items: center; justify-content: center; color: #16a34a; font-size: 11px; font-weight: 700; animation: pulseGlow 1.5s infinite;"
                      >
                        {idx() + 1}
                      </div>
                    </Show>

                    {/* B. 已完成态 (实心翠绿 + 白勾) */}
                    <Show when={m.status === 'completed'}>
                      <div
                        style="width: 24px; height: 24px; border-radius: 50%; background: #22c55e; display: flex; align-items: center; justify-content: center; color: #ffffff; font-size: 12px; font-weight: 700; box-shadow: 0 1px 4px rgba(34, 197, 94, 0.4);"
                      >
                        ✓
                      </div>
                    </Show>

                    {/* C. 待执行未开始态 (灰色圆点) */}
                    <Show when={m.status === 'pending'}>
                      <div
                        style="width: 24px; height: 24px; border-radius: 50%; background: #f1f5f9; border: 1.5px solid #cbd5e1; display: flex; align-items: center; justify-content: center; color: #94a3b8; font-size: 11.5px; font-weight: 600;"
                      >
                        {idx() + 1}
                      </div>
                    </Show>
                  </div>

                  {/* 节点标题 */}
                  <div
                    style={`font-size: 12px; white-space: nowrap; max-width: 110px; overflow: hidden; text-overflow: ellipsis; ${
                      m.status === 'running'
                        ? 'color: #16a34a; font-weight: 600;'
                        : m.status === 'completed'
                        ? 'color: #15803d; font-weight: 500;'
                        : 'color: #64748b; font-weight: 400;'
                    }`}
                    title={m.title}
                  >
                    {m.title}
                  </div>

                  {/* 工作量标识胶囊 (S / M / L) */}
                  <div
                    style={`font-size: 10px; padding: 1px 6px; border-radius: 4px; background: ${effortBadge().bg}; color: ${effortBadge().color}; font-weight: 600; white-space: nowrap;`}
                  >
                    {effortBadge().label}
                  </div>

                  {/* 悬停详细 Tooltip 浮窗 */}
                  <Show when={hoveredNodeId() === m.id}>
                    <div
                      style="position: absolute; bottom: 100%; margin-bottom: 8px; left: 50%; transform: translateX(-50%); background: #18181b; color: #ffffff; padding: 8px 12px; border-radius: 8px; font-size: 11.5px; line-height: 1.5; white-space: nowrap; z-index: 160; box-shadow: 0 8px 20px rgba(0,0,0,0.2); pointer-events: none;"
                    >
                      <div style="font-weight: 600; color: #f4f4f5; margin-bottom: 2px;">
                        里程碑 {idx() + 1}: {m.title}
                      </div>
                      <div style="color: #a1a1aa; font-size: 11px;">
                        状态: {m.status === 'completed' ? '已完成 ✓' : m.status === 'running' ? '正在执行中 ⚙' : '待开始 ⏳'} · 预估工作量: {effortBadge().label}
                      </div>
                      <Show when={m.desc}>
                        <div style="color: #d4d4d8; font-size: 10.5px; margin-top: 4px;">
                          {m.desc}
                        </div>
                      </Show>
                    </div>
                  </Show>
                </div>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
};
