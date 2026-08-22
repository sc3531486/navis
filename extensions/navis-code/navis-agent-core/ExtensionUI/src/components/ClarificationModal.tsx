import { Component, createSignal, For, Show } from 'solid-js';

export interface ClarificationQuestion {
  title: string;
  options: string[];
  defaultSelected?: number[];
}

export const ClarificationModal: Component<{
  open: boolean;
  question: ClarificationQuestion;
  onSkip: () => void;
  onConfirm: (selectedOptions: string[], customInput: string) => void;
}> = (props) => {
  const [selectedIndices, setSelectedIndices] = createSignal<number[]>([0]);
  const [customText, setCustomText] = createSignal('');

  const toggleOption = (idx: number) => {
    const curr = selectedIndices();
    if (curr.includes(idx)) {
      setSelectedIndices(curr.filter((i) => i !== idx));
    } else {
      setSelectedIndices([...curr, idx]);
    }
  };

  const handleConfirm = () => {
    const selected = selectedIndices().map((i) => props.question.options[i]).filter(Boolean);
    props.onConfirm(selected, customText().trim());
  };

  return (
    <Show when={props.open}>
      <div
        id="clarification-modal-overlay"
        style="position: fixed; inset: 0; background: rgba(15, 23, 42, 0.45); backdrop-filter: blur(4px); display: flex; align-items: center; justify-content: center; z-index: 200; padding: 20px; animation: fadeIn 0.2s ease-out;"
      >
        <div
          id="clarification-modal-card"
          style="width: 100%; max-width: 520px; background: #ffffff; border-radius: 14px; box-shadow: 0 20px 40px rgba(0, 0, 0, 0.15); border: 1px solid #e2e8f0; overflow: hidden; display: flex; flex-direction: column;"
        >
          {/* 1. 弹窗头部 */}
          <div
            style="padding: 18px 20px 14px; border-bottom: 1px solid #f1f5f9; display: flex; align-items: center; justify-content: space-between;"
          >
            <div style="display: flex; align-items: center; gap: 8px;">
              <div
                style="width: 28px; height: 28px; border-radius: 7px; background: #eff6ff; color: #2563eb; display: flex; align-items: center; justify-content: center; font-size: 14px; font-weight: 700;"
              >
                ?
              </div>
              <div>
                <div style="font-size: 14.5px; font-weight: 600; color: #0f172a;">计划模式 · 关键决策确认</div>
                <div style="font-size: 11.5px; color: #64748b;">请确认以下关键技术架构与执行策略，以便生成精准的里程碑计划</div>
              </div>
            </div>
          </div>

          {/* 2. 问题与选项列表 */}
          <div style="padding: 18px 20px; display: flex; flex-direction: column; gap: 12px; max-height: 60vh; overflow-y: auto;">
            <div style="font-size: 13.5px; font-weight: 600; color: #1e293b;">
              {props.question.title}
            </div>

            {/* 可选列表 */}
            <div style="display: flex; flex-direction: column; gap: 8px;">
              <For each={props.question.options}>
                {(opt, idx) => {
                  const isSelected = () => selectedIndices().includes(idx());
                  return (
                    <div
                      onClick={() => toggleOption(idx())}
                      style={`padding: 10px 14px; border-radius: 8px; border: 1px solid ${
                        isSelected() ? '#2563eb' : '#e2e8f0'
                      }; background: ${
                        isSelected() ? '#eff6ff' : '#ffffff'
                      }; cursor: pointer; display: flex; align-items: center; gap: 10px; transition: all 0.15s ease;`}
                    >
                      <div
                        style={`width: 16px; height: 16px; border-radius: 4px; border: 1.5px solid ${
                          isSelected() ? '#2563eb' : '#94a3b8'
                        }; background: ${
                          isSelected() ? '#2563eb' : 'transparent'
                        }; display: flex; align-items: center; justify-content: center; color: #ffffff; font-size: 11px; font-weight: 700;`}
                      >
                        <Show when={isSelected()}>✓</Show>
                      </div>
                      <span
                        style={`font-size: 13px; color: ${
                          isSelected() ? '#1d4ed8' : '#334155'
                        }; font-weight: ${isSelected() ? '500' : '400'};`}
                      >
                        {opt}
                      </span>
                    </div>
                  );
                }}
              </For>
            </div>

            {/* 3. 最后一个：自定义手输选项 (自由输入补充) */}
            <div style="margin-top: 4px; display: flex; flex-direction: column; gap: 6px;">
              <div style="font-size: 12px; font-weight: 500; color: #64748b;">
                其他补充需求（可选手输）：
              </div>
              <input
                id="clarification-custom-input"
                type="text"
                value={customText()}
                onInput={(e) => setCustomText(e.currentTarget.value)}
                placeholder="例如：优先保证向下兼容，不需要引入额外的第三方重依赖..."
                style="width: 100%; padding: 8px 12px; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 13px; color: #18181b; outline: none; background: #fafafa; font-family: inherit;"
                onFocus={(e) => (e.currentTarget.style.borderColor = '#2563eb')}
                onBlur={(e) => (e.currentTarget.style.borderColor = '#e2e8f0')}
              />
            </div>
          </div>

          {/* 4. 底部操作按钮：跳过 与 确认 */}
          <div
            style="padding: 12px 20px; border-top: 1px solid #f1f5f9; background: #f8fafc; display: flex; align-items: center; justify-content: flex-end; gap: 10px;"
          >
            <button
              id="clarification-btn-skip"
              onClick={props.onSkip}
              style="padding: 6px 14px; background: transparent; border: 1px solid #cbd5e1; border-radius: 7px; font-size: 13px; font-weight: 500; color: #475569; cursor: pointer; transition: all 0.15s ease;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#e2e8f0')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              跳过 (使用默认策略)
            </button>
            <button
              id="clarification-btn-confirm"
              onClick={handleConfirm}
              style="padding: 6px 18px; background: #16a34a; border: none; border-radius: 7px; font-size: 13px; font-weight: 600; color: #ffffff; cursor: pointer; transition: all 0.15s ease; box-shadow: 0 1px 4px rgba(22, 163, 74, 0.3);"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#15803d')}
              onMouseLeave={(e) => (e.currentTarget.style.background = '#16a34a')}
            >
              确认并生成执行计划
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
};
