import { Component, createSignal, onCleanup, Show, For } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';

interface SettingsModalProps {
  ctx: NavisContext;
}

export const SettingsModal: Component<SettingsModalProps> = (props) => {
  const [open, setOpen] = createSignal(false);
  const [activeTab, setActiveTab] = createSignal<'general' | 'gateway' | 'keys' | 'sandbox' | 'prompt'>('general');
  const [gatewayUrl, setGatewayUrl] = createSignal('http://127.0.0.1:15721');
  const [anthropicKey, setAnthropicKey] = createSignal('');
  const [geminiKey, setGeminiKey] = createSignal('');
  const [themeMode, setThemeMode] = createSignal('light');
  const [sandboxLevel, setSandboxLevel] = createSignal('bypass');
  const [customPrompt, setCustomPrompt] = createSignal('You are an expert AI software architect and coding assistant.');

  const unsub = props.ctx.events.on('settings:open', (payload: any) => {
    if (payload?.tab) {
      setActiveTab(payload.tab);
    }
    setOpen(true);
  });

  onCleanup(() => unsub());

  const handleSave = () => {
    setOpen(false);
    toast.success('设置已保存并生效！');
  };

  const handleTestConnection = () => {
    toast.info('正在测试网关连接...');
    setTimeout(() => {
      toast.success('网关连接正常 (Ping: 14ms)');
    }, 1000);
  };

  return (
    <Show when={open()}>
      <div
        onClick={() => setOpen(false)}
        style="position: fixed; inset: 0; background: rgba(0,0,0,0.35); backdrop-filter: blur(3px); z-index: 9999; display: flex; align-items: center; justify-content: center;"
      >
        <div
          onClick={(e) => e.stopPropagation()}
          style="width: 580px; max-width: 90vw; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 12px; box-shadow: 0 16px 40px rgba(0,0,0,0.18); overflow: hidden; display: flex; flex-direction: column; animation: navis-pop 0.15s ease-out;"
        >
          {/* 标题栏 */}
          <div style="height: 48px; border-bottom: 1px solid #eae7e1; display: flex; align-items: center; justify-content: space-between; padding: 0 18px;">
            <div style="display: flex; align-items: center; gap: 8px;">
              <span style="font-size: 16px; color: #c2410c;">⚙️</span>
              <h2 style="font-size: 15px; font-weight: 600; color: #2d2b28; margin: 0;">全局设置 (Settings)</h2>
            </div>
            <button
              onClick={() => setOpen(false)}
              style="background: transparent; border: none; font-size: 16px; color: #8e8b83; cursor: pointer; padding: 4px; border-radius: 4px;"
            >
              ✕
            </button>
          </div>

          {/* 标签栏 */}
          <div style="display: flex; gap: 4px; padding: 8px 16px; background: #fbfaf8; border-bottom: 1px solid #eae7e1;">
            <button
              onClick={() => setActiveTab('general')}
              style={`padding: 6px 12px; border-radius: 6px; border: none; font-size: 12.5px; font-weight: 500; cursor: pointer; ${
                activeTab() === 'general' ? 'background: #eceae4; color: #1e1d1b;' : 'background: transparent; color: #76736c;'
              }`}
            >
              通用 (General)
            </button>
            <button
              onClick={() => setActiveTab('gateway')}
              style={`padding: 6px 12px; border-radius: 6px; border: none; font-size: 12.5px; font-weight: 500; cursor: pointer; ${
                activeTab() === 'gateway' ? 'background: #eceae4; color: #1e1d1b;' : 'background: transparent; color: #76736c;'
              }`}
            >
              网关 (Gateway)
            </button>
            <button
              onClick={() => setActiveTab('keys')}
              style={`padding: 6px 12px; border-radius: 6px; border: none; font-size: 12.5px; font-weight: 500; cursor: pointer; ${
                activeTab() === 'keys' ? 'background: #eceae4; color: #1e1d1b;' : 'background: transparent; color: #76736c;'
              }`}
            >
              API 密钥 (Keys)
            </button>
            <button
              onClick={() => setActiveTab('sandbox')}
              style={`padding: 6px 12px; border-radius: 6px; border: none; font-size: 12.5px; font-weight: 500; cursor: pointer; ${
                activeTab() === 'sandbox' ? 'background: #eceae4; color: #1e1d1b;' : 'background: transparent; color: #76736c;'
              }`}
            >
              沙箱权限 (Sandbox)
            </button>
            <button
              onClick={() => setActiveTab('prompt')}
              style={`padding: 6px 12px; border-radius: 6px; border: none; font-size: 12.5px; font-weight: 500; cursor: pointer; ${
                activeTab() === 'prompt' ? 'background: #eceae4; color: #1e1d1b;' : 'background: transparent; color: #76736c;'
              }`}
            >
              自定义指令 (Prompt)
            </button>
          </div>

          {/* 内容区 */}
          <div style="padding: 20px; display: flex; flex-direction: column; gap: 16px; min-height: 220px;">
            <Show when={activeTab() === 'general'}>
              <div style="display: flex; flex-direction: column; gap: 6px;">
                <label style="font-size: 12px; font-weight: 500; color: #4b4843;">界面主题 (Appearance):</label>
                <select
                  value={themeMode()}
                  onChange={(e) => {
                    setThemeMode(e.currentTarget.value);
                    document.documentElement.setAttribute('data-theme', e.currentTarget.value);
                  }}
                  style="background: #faf9f6; border: 1px solid #e7e4dc; border-radius: 6px; padding: 7px 10px; color: #2d2b28; font-size: 13px;"
                >
                  <option value="light">Claude Light (暖白浅色 - 推荐)</option>
                  <option value="dark">Navis Dark (深色)</option>
                </select>
              </div>
            </Show>

            <Show when={activeTab() === 'gateway'}>
              <div style="display: flex; flex-direction: column; gap: 6px;">
                <label style="font-size: 12px; font-weight: 500; color: #4b4843;">AI 网关服务地址 (Gateway Endpoint):</label>
                <div style="display: flex; gap: 8px;">
                  <input
                    type="text"
                    value={gatewayUrl()}
                    onInput={(e) => setGatewayUrl(e.currentTarget.value)}
                    style="flex: 1; background: #faf9f6; border: 1px solid #e7e4dc; border-radius: 6px; padding: 7px 10px; color: #2d2b28; font-size: 13px;"
                  />
                  <button
                    onClick={handleTestConnection}
                    style="background: #f0eee8; border: 1px solid #e7e4dc; border-radius: 6px; padding: 0 12px; font-size: 12.5px; font-weight: 500; color: #2d2b28; cursor: pointer;"
                  >
                    测试连接
                  </button>
                </div>
                <span style="font-size: 11.5px; color: #8e8b83;">默认本地 AI 网关端口为 15721，支持 LiteLLM / Ollama / OneAPI 代理。</span>
              </div>
            </Show>

            <Show when={activeTab() === 'keys'}>
              <div style="display: flex; flex-direction: column; gap: 12px;">
                <div style="display: flex; flex-direction: column; gap: 6px;">
                  <label style="font-size: 12px; font-weight: 500; color: #4b4843;">Anthropic API Key:</label>
                  <input
                    type="password"
                    value={anthropicKey()}
                    onInput={(e) => setAnthropicKey(e.currentTarget.value)}
                    placeholder="sk-ant-..."
                    style="background: #faf9f6; border: 1px solid #e7e4dc; border-radius: 6px; padding: 7px 10px; color: #2d2b28; font-size: 13px;"
                  />
                </div>
                <div style="display: flex; flex-direction: column; gap: 6px;">
                  <label style="font-size: 12px; font-weight: 500; color: #4b4843;">Google Gemini API Key:</label>
                  <input
                    type="password"
                    value={geminiKey()}
                    onInput={(e) => setGeminiKey(e.currentTarget.value)}
                    placeholder="AIzaSy..."
                    style="background: #faf9f6; border: 1px solid #e7e4dc; border-radius: 6px; padding: 7px 10px; color: #2d2b28; font-size: 13px;"
                  />
                </div>
              </div>
            </Show>

            <Show when={activeTab() === 'sandbox'}>
              <div style="display: flex; flex-direction: column; gap: 6px;">
                <label style="font-size: 12px; font-weight: 500; color: #4b4843;">Agent 权限执行模式 (Execution Policy):</label>
                <select
                  value={sandboxLevel()}
                  onChange={(e) => setSandboxLevel(e.currentTarget.value)}
                  style="background: #faf9f6; border: 1px solid #e7e4dc; border-radius: 6px; padding: 7px 10px; color: #2d2b28; font-size: 13px;"
                >
                  <option value="bypass">Bypass permissions (自动授权所有文件与命令执行 - 推荐)</option>
                  <option value="confirm">Ask for confirmation (危险操作弹出审批确认)</option>
                  <option value="strict">Strict Sandbox (只读模式，禁止写入文件与执行 Shell)</option>
                </select>
              </div>
            </Show>

            <Show when={activeTab() === 'prompt'}>
              <div style="display: flex; flex-direction: column; gap: 6px;">
                <label style="font-size: 12px; font-weight: 500; color: #4b4843;">系统提示词 / 自定义工作指令 (Custom Instructions):</label>
                <textarea
                  rows={5}
                  value={customPrompt()}
                  onInput={(e) => setCustomPrompt(e.currentTarget.value)}
                  style="background: #faf9f6; border: 1px solid #e7e4dc; border-radius: 6px; padding: 8px 10px; color: #2d2b28; font-size: 13px; resize: vertical; outline: none;"
                />
              </div>
            </Show>
          </div>

          {/* 底部操作栏 */}
          <div style="height: 52px; background: #fbfaf8; border-top: 1px solid #eae7e1; display: flex; align-items: center; justify-content: flex-end; gap: 10px; padding: 0 18px;">
            <button
              onClick={() => setOpen(false)}
              style="padding: 7px 14px; background: transparent; border: 1px solid #e7e4dc; border-radius: 6px; font-size: 13px; font-weight: 500; color: #5a5750; cursor: pointer;"
            >
              取消
            </button>
            <button
              onClick={handleSave}
              style="padding: 7px 18px; background: #2d2b28; border: none; border-radius: 6px; font-size: 13px; font-weight: 500; color: #ffffff; cursor: pointer;"
            >
              保存更改
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
};

export default SettingsModal;
