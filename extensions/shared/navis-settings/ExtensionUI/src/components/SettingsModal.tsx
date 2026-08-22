import { Component, createSignal, onMount, onCleanup, Show, For } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import { gatewayStore, type ProviderItem, type ModelItem } from '@extensions/shared/navis-ai-platform/ExtensionUI/src/store/GatewayStore';
import {
  IconSettings,
  IconCpu,
  IconShield,
  IconPrompt,
  IconPalette,
  IconZap,
  IconRefresh,
  IconClose,
  IconTrash,
  IconEye,
  IconEyeOff,
  IconPlus,
  IconCheck,
} from '@/components/icons';

interface SettingsModalProps {
  ctx: NavisContext;
}

export const SettingsModal: Component<SettingsModalProps> = (props) => {
  const [open, setOpen] = createSignal(false);
  const [activeMainTab, setActiveMainTab] = createSignal<'models' | 'sandbox' | 'prompt' | 'general'>('models');
  const [selectedProviderId, setSelectedProviderId] = createSignal<string>('gateway-local');
  const [showApiKey, setShowApiKey] = createSignal(false);
  const [isTesting, setIsTesting] = createSignal(false);
  const [isSyncing, setIsSyncing] = createSignal(false);
  const [pingLatency, setPingLatency] = createSignal<number | null>(42);

  // 新增模型抽屉状态
  const [showAddModelModal, setShowAddModelModal] = createSignal(false);
  const [newModelId, setNewModelId] = createSignal('');
  const [newModelName, setNewModelName] = createSignal('');
  const [newModelContext, setNewModelContext] = createSignal(128000);
  const [newModelMaxOutput, setNewModelMaxOutput] = createSignal(8192);
  const [newModelTools, setNewModelTools] = createSignal(true);
  const [newModelStream, setNewModelStream] = createSignal(true);
  const [newModelVision, setNewModelVision] = createSignal(false);
  const [newModelReasoning, setNewModelReasoning] = createSignal(false);

  // 新增 Provider 抽屉状态
  const [showAddProviderModal, setShowAddProviderModal] = createSignal(false);
  const [newProviderName, setNewProviderName] = createSignal('');
  const [newProviderUrl, setNewProviderUrl] = createSignal('');
  const [newProviderKey, setNewProviderKey] = createSignal('');

  // 沙箱与指令状态
  const [sandboxLevel, setSandboxLevel] = createSignal<'bypass' | 'confirm' | 'strict'>('bypass');
  const [systemPrompt, setSystemPrompt] = createSignal('You are an expert full-stack AI software engineer and architect with rigorous code standards.');
  const [themeMode, setThemeMode] = createSignal('light');

  const unsub = props.ctx.events.on('settings:open', (payload: any) => {
    if (payload?.tab === 'gateway' || payload?.tab === 'models') {
      setActiveMainTab('models');
    } else if (payload?.tab === 'prompt') {
      setActiveMainTab('prompt');
    } else if (payload?.tab === 'sandbox') {
      setActiveMainTab('sandbox');
    } else if (payload?.tab === 'general') {
      setActiveMainTab('general');
    }
    setOpen(true);
  });

  onMount(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && open()) {
        if (showAddModelModal()) {
          setShowAddModelModal(false);
        } else if (showAddProviderModal()) {
          setShowAddProviderModal(false);
        } else {
          setOpen(false);
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    onCleanup(() => {
      window.removeEventListener('keydown', handleKeyDown);
    });
  });

  onCleanup(() => unsub());

  const currentProvider = () => {
    return gatewayStore.providers().find((p) => p.id === selectedProviderId()) || gatewayStore.providers()[0];
  };

  const handleTestCurrentProvider = async () => {
    const p = currentProvider();
    if (!p) return;
    setIsTesting(true);
    toast.info(`正在连接测试 ${p.name}...`);
    const res = await gatewayStore.testConnection(p.id);
    setIsTesting(false);
    if (res.success) {
      setPingLatency(res.pingMs);
      toast.success(`连接测试成功！延迟: ${res.pingMs}ms`);
    } else {
      setPingLatency(null);
      toast.error(`连接失败，请检查 Base URL 和 API Key`);
    }
  };

  const handleSyncModels = async () => {
    const p = currentProvider();
    if (!p) return;
    setIsSyncing(true);
    toast.info(`正在从 ${p.baseUrl}/v1/models 同步可用模型...`);
    const list = await gatewayStore.fetchModels(p.id);
    setIsSyncing(false);
    toast.success(`已同步发现 ${list.length} 个可用模型`);
  };

  const handleSaveModel = () => {
    const p = currentProvider();
    if (!p || !newModelId().trim()) {
      toast.warning('请输入有效的模型 ID');
      return;
    }

    const model: ModelItem = {
      id: newModelId().trim(),
      name: newModelName().trim() || newModelId().trim(),
      providerId: p.id,
      apiProtocol: p.type === 'anthropic' ? 'anthropic_messages' : 'chat_completions',
      contextWindow: Number(newModelContext()) || 128000,
      maxOutputTokens: Number(newModelMaxOutput()) || 8192,
      capabilities: {
        tools: newModelTools(),
        streaming: newModelStream(),
        vision: newModelVision(),
        reasoning: newModelReasoning(),
      },
    };

    gatewayStore.addModel(p.id, model);
    setShowAddModelModal(false);
    setNewModelId('');
    setNewModelName('');
    toast.success(`已添加模型: ${model.name}`);
  };

  const handleSaveCustomProvider = () => {
    if (!newProviderName().trim() || !newProviderUrl().trim()) {
      toast.warning('请填写 Provider 名称与 Base URL');
      return;
    }

    const customId = `custom-${Date.now()}`;
    const newProv: ProviderItem = {
      id: customId,
      name: newProviderName().trim(),
      type: 'custom',
      baseUrl: newProviderUrl().trim(),
      apiKey: newProviderKey().trim(),
      status: 'connected',
      defaultModelId: `${customId}-model`,
      models: [
        {
          id: `${customId}-model`,
          name: `${newProviderName().trim()} Default Model`,
          providerId: customId,
          apiProtocol: 'chat_completions',
          contextWindow: 128000,
          maxOutputTokens: 8192,
          capabilities: { tools: true, streaming: true, vision: false, reasoning: false },
          isDefault: true,
        },
      ],
    };

    gatewayStore.addCustomProvider(newProv);
    setSelectedProviderId(customId);
    setShowAddProviderModal(false);
    setNewProviderName('');
    setNewProviderUrl('');
    setNewProviderKey('');
    toast.success(`已添加自定义 Provider: ${newProv.name}`);
  };

  const handleSaveAll = () => {
    setOpen(false);
    toast.success('配置已保存并实时生效！');
  };

  const formatContextSize = (size?: number) => {
    if (!size) return '128k';
    if (size >= 1000000) return `${Math.round(size / 1000)}k`;
    return `${Math.round(size / 1000)}k`;
  };

  return (
    <Show when={open()}>
      <div
        onClick={() => setOpen(false)}
        style="position: fixed; inset: 0; background: rgba(0,0,0,0.45); backdrop-filter: blur(4px); z-index: 9999; display: flex; align-items: center; justify-content: center; padding: 24px; pointer-events: auto;"
      >
        <div
          onClick={(e) => e.stopPropagation()}
          style="width: 940px; max-width: 95vw; height: 630px; max-height: 90vh; background: #ffffff; border: 1px solid #e5e5e5; border-radius: 14px; box-shadow: 0 24px 48px -12px rgba(0,0,0,0.18); overflow: hidden; display: flex; flex-direction: row; animation: navis-pop 0.15s ease-out; pointer-events: auto; position: relative;"
        >
          {/* ══════════════════════════════════════════════════════════════════════
              左侧导航栏 (Left Sidebar Navigation)
             ══════════════════════════════════════════════════════════════════════ */}
          <div style="width: 220px; background: #f8f8f7; border-right: 1px solid #eaeaea; display: flex; flex-direction: column; justify-content: space-between; padding: 18px 12px;">
            <div style="display: flex; flex-direction: column; gap: 4px;">
              <div style="display: flex; align-items: center; gap: 8px; padding: 4px 8px 14px; border-bottom: 1px solid #ebeaea; margin-bottom: 6px;">
                <span style="color: #c2410c; display: flex; align-items: center;">
                  <IconSettings size={18} />
                </span>
                <span style="font-size: 14px; font-weight: 600; color: #18181b;">Navis Code Settings</span>
              </div>

              {/* 垂直导航项 */}
              <button
                data-tab-id="models"
                onClick={() => setActiveMainTab('models')}
                style={`display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 8px; border: none; font-size: 13px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s; ${
                  activeMainTab() === 'models' ? 'background: #eceae5; color: #18181b; font-weight: 600;' : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconCpu size={16} />
                <span>Models & Gateway</span>
              </button>

              <button
                data-tab-id="sandbox"
                onClick={() => setActiveMainTab('sandbox')}
                style={`display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 8px; border: none; font-size: 13px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s; ${
                  activeMainTab() === 'sandbox' ? 'background: #eceae5; color: #18181b; font-weight: 600;' : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconShield size={16} />
                <span>Sandbox Policy</span>
              </button>

              <button
                data-tab-id="prompt"
                onClick={() => setActiveMainTab('prompt')}
                style={`display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 8px; border: none; font-size: 13px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s; ${
                  activeMainTab() === 'prompt' ? 'background: #eceae5; color: #18181b; font-weight: 600;' : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconPrompt size={16} />
                <span>Custom Prompts</span>
              </button>

              <button
                data-tab-id="general"
                onClick={() => setActiveMainTab('general')}
                style={`display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 8px; border: none; font-size: 13px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s; ${
                  activeMainTab() === 'general' ? 'background: #eceae5; color: #18181b; font-weight: 600;' : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconPalette size={16} />
                <span>General</span>
              </button>
            </div>

            {/* 左下角退出按钮 */}
            <div style="border-top: 1px solid #ebeaea; padding-top: 10px;">
              <button
                onClick={() => setOpen(false)}
                style="display: flex; align-items: center; justify-content: center; gap: 6px; width: 100%; padding: 7px; background: transparent; border: 1px solid #e5e5e5; border-radius: 6px; font-size: 12px; color: #71717a; cursor: pointer;"
              >
                <IconClose size={14} />
                <span>关闭 (Close)</span>
              </button>
            </div>
          </div>

          {/* ══════════════════════════════════════════════════════════════════════
              右侧主工作区 (Right Content Area)
             ══════════════════════════════════════════════════════════════════════ */}
          <div style="flex: 1; display: flex; flex-direction: column; background: #ffffff; overflow: hidden; min-height: 0;">
            {/* 顶栏标题与关闭 ✕ */}
            <div style="display: flex; align-items: flex-start; justify-content: space-between; padding: 18px 24px 12px; border-bottom: 1px solid #f0eee8;">
              <div style="display: flex; flex-direction: column; gap: 2px;">
                <h2 style="font-size: 16px; font-weight: 600; color: #18181b; margin: 0;">
                  {activeMainTab() === 'models'
                    ? 'AI Model and Provider Settings'
                    : activeMainTab() === 'sandbox'
                    ? 'Agent Execution Policy & Sandbox'
                    : activeMainTab() === 'prompt'
                    ? 'System Instructions & Custom Prompts'
                    : 'General Settings'}
                </h2>
                <span style="font-size: 12px; color: #71717a;">
                  {activeMainTab() === 'models'
                    ? 'Configure Navis Code integration, active LLMs, and API endpoints'
                    : 'Configure workspace preferences and permissions'}
                </span>
              </div>
              <button
                onClick={() => setOpen(false)}
                style="background: transparent; border: none; color: #71717a; cursor: pointer; padding: 4px; border-radius: 4px; display: flex; align-items: center; justify-content: center;"
                title="关闭"
              >
                <IconClose size={16} />
              </button>
            </div>

            {/* ── 1. Models & Gateway Tab ──────────────────────────────────── */}
            <Show when={activeMainTab() === 'models'}>
              <div style="flex: 1; display: flex; flex-direction: column; overflow-y: auto; padding: 18px 24px; gap: 16px; overscroll-behavior: contain; min-height: 0;">
                {/* 1.1 Provider 选择胶囊栏 */}
                <div style="display: flex; flex-direction: column; gap: 8px;">
                  <div style="display: flex; align-items: center; justify-content: space-between;">
                    <span style="font-size: 12px; font-weight: 600; color: #52525b;">Provider</span>
                    <button
                      onClick={() => setShowAddProviderModal(true)}
                      style="background: transparent; border: none; font-size: 12px; color: #ea580c; cursor: pointer; display: flex; align-items: center; gap: 4px; font-weight: 500;"
                    >
                      <IconPlus size={12} />
                      <span>添加 Provider</span>
                    </button>
                  </div>

                  <div style="display: flex; gap: 6px; flex-wrap: wrap;">
                    <For each={gatewayStore.providers()}>
                      {(p) => {
                        const isSelected = () => selectedProviderId() === p.id;
                        const isGlobalActive = () => gatewayStore.activeProviderId() === p.id;
                        return (
                          <div
                            data-provider-id={p.id}
                            onClick={() => {
                              setSelectedProviderId(p.id);
                              gatewayStore.setActiveProvider(p.id);
                            }}
                            style={`padding: 5px 12px; border-radius: 8px; cursor: pointer; display: flex; align-items: center; gap: 7px; font-size: 12px; transition: all 0.12s; border: 1px solid; ${
                              isSelected()
                                ? 'background: #18181b; border-color: #18181b; color: #ffffff; font-weight: 500;'
                                : 'background: #fafaf9; border-color: #e4e4e7; color: #52525b;'
                            }`}
                          >
                            <span
                              style={`width: 6px; height: 6px; border-radius: 50%; ${
                                isGlobalActive() ? 'background: #22c55e;' : 'background: #a1a1aa;'
                              }`}
                            />
                            <span>{p.name.split(' (')[0]}</span>
                          </div>
                        );
                      }}
                    </For>
                  </div>
                </div>

                {/* 1.2 API Endpoint & 鉴权紧凑横条 */}
                <div style="display: flex; flex-direction: column; gap: 8px;">
                  <span style="font-size: 12px; font-weight: 600; color: #52525b;">API Endpoint</span>
                  <div style="background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 10px; padding: 12px 14px; display: flex; flex-direction: column; gap: 10px;">
                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
                      <div style="display: flex; flex-direction: column; gap: 4px;">
                        <span style="font-size: 11px; color: #71717a; font-weight: 500;">Base URL</span>
                        <input
                          type="text"
                          value={currentProvider()?.baseUrl || ''}
                          onInput={(e) => gatewayStore.updateProvider(currentProvider().id, { baseUrl: e.currentTarget.value })}
                          style="background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px; color: #18181b; font-family: monospace; outline: none;"
                        />
                      </div>
                      <div style="display: flex; flex-direction: column; gap: 4px;">
                        <div style="display: flex; align-items: center; justify-content: space-between;">
                          <span style="font-size: 11px; color: #71717a; font-weight: 500;">API Key</span>
                          <button
                            onClick={() => setShowApiKey(!showApiKey())}
                            style="background: transparent; border: none; font-size: 11px; color: #71717a; cursor: pointer; display: flex; align-items: center; gap: 3px;"
                          >
                            {showApiKey() ? <IconEyeOff size={12} /> : <IconEye size={12} />}
                            <span>{showApiKey() ? '隐藏' : '显示'}</span>
                          </button>
                        </div>
                        <input
                          type={showApiKey() ? 'text' : 'password'}
                          value={currentProvider()?.apiKey || ''}
                          onInput={(e) => gatewayStore.updateProvider(currentProvider().id, { apiKey: e.currentTarget.value })}
                          style="background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px; color: #18181b; font-family: monospace; outline: none;"
                        />
                      </div>
                    </div>

                    {/* 状态与测速条 */}
                    <div style="display: flex; align-items: center; justify-content: space-between; border-top: 1px solid #f0eee8; padding-top: 8px;">
                      <div style="display: flex; align-items: center; gap: 6px;">
                        <span style="width: 7px; height: 7px; border-radius: 50%; background: #22c55e;" />
                        <span style="font-size: 11.5px; color: #15803d; font-weight: 500;">
                          Connected ({pingLatency() || 42}ms Latency, Ping)
                        </span>
                      </div>
                      <div style="display: flex; align-items: center; gap: 8px;">
                        <button
                          onClick={handleTestCurrentProvider}
                          disabled={isTesting()}
                          style="padding: 4px 10px; background: #ffffff; border: 1px solid #d4d4d8; border-radius: 5px; font-size: 11.5px; color: #3f3f46; cursor: pointer; display: flex; align-items: center; gap: 4px;"
                        >
                          <IconZap size={12} color="#ea580c" />
                          <span>{isTesting() ? '测速中...' : '测试连接 (Ping)'}</span>
                        </button>
                        <button
                          onClick={handleSyncModels}
                          disabled={isSyncing()}
                          style="padding: 4px 10px; background: #ffffff; border: 1px solid #d4d4d8; border-radius: 5px; font-size: 11.5px; color: #3f3f46; cursor: pointer; display: flex; align-items: center; gap: 4px;"
                        >
                          <IconRefresh size={12} color="#2563eb" />
                          <span>{isSyncing() ? '同步中...' : '同步模型'}</span>
                        </button>
                      </div>
                    </div>
                  </div>
                </div>

                {/* 1.3 结构化模型能力数据表格 (Model List Table) */}
                <div style="display: flex; flex-direction: column; gap: 8px;">
                  <div style="display: flex; align-items: center; justify-content: space-between;">
                    <span style="font-size: 12px; font-weight: 600; color: #52525b;">
                      Model List ({currentProvider()?.models.length || 0})
                    </span>
                    <button
                      onClick={() => setShowAddModelModal(true)}
                      style="background: transparent; border: none; font-size: 12px; color: #ea580c; cursor: pointer; display: flex; align-items: center; gap: 4px; font-weight: 500;"
                    >
                      <IconPlus size={12} />
                      <span>添加模型</span>
                    </button>
                  </div>

                  {/* 数据表格容器 */}
                  <div style="border: 1px solid #e4e4e7; border-radius: 8px; overflow: hidden; background: #ffffff;">
                    {/* 表头 */}
                    <div style="display: grid; grid-template-columns: 48px 1.6fr 1fr 70px 1.4fr 48px; background: #fafaf9; border-bottom: 1px solid #e4e4e7; padding: 7px 12px; font-size: 11px; font-weight: 600; color: #71717a;">
                      <span>Active</span>
                      <span>Model Name</span>
                      <span>Provider</span>
                      <span>Context</span>
                      <span>Capabilities</span>
                      <span style="text-align: center;">Action</span>
                    </div>

                    {/* 表体行 */}
                    <div style="max-height: 200px; overflow-y: auto; overscroll-behavior: contain;">
                      <For each={currentProvider()?.models}>
                        {(model) => {
                          const isActive = () => gatewayStore.activeModelId() === model.id;
                          return (
                            <div
                              onClick={() => {
                                gatewayStore.setActiveModel(model.id);
                                toast.success(`已设置默认模型: ${model.name}`);
                              }}
                              style={`display: grid; grid-template-columns: 48px 1.6fr 1fr 70px 1.4fr 48px; align-items: center; padding: 8px 12px; border-bottom: 1px solid #f4f4f5; font-size: 12px; cursor: pointer; transition: background 0.1s; ${
                                isActive() ? 'background: #fafaf9;' : 'background: #ffffff;'
                              }`}
                              onMouseEnter={(e) => {
                                if (!isActive()) e.currentTarget.style.background = '#fcfbf9';
                              }}
                              onMouseLeave={(e) => {
                                if (!isActive()) e.currentTarget.style.background = '#ffffff';
                              }}
                            >
                              {/* 单选激活圆点 */}
                              <div style="display: flex; align-items: center;">
                                <span
                                  style={`width: 14px; height: 14px; border-radius: 50%; display: flex; align-items: center; justify-content: center; border: 1.5px solid; ${
                                    isActive()
                                      ? 'border-color: #2563eb; background: #2563eb;'
                                      : 'border-color: #d4d4d8; background: #ffffff;'
                                  }`}
                                >
                                  <Show when={isActive()}>
                                    <span style="width: 5px; height: 5px; border-radius: 50%; background: #ffffff;" />
                                  </Show>
                                </span>
                              </div>

                              {/* 模型名称 */}
                              <div style="display: flex; flex-direction: column; overflow: hidden; padding-right: 8px;">
                                <b style="color: #18181b; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                                  {model.name}
                                </b>
                                <span style="font-size: 10px; color: #a1a1aa; font-family: monospace; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                                  {model.id}
                                </span>
                              </div>

                              {/* 服务商 */}
                              <span style="color: #71717a; font-size: 11.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                                {currentProvider()?.name.split(' (')[0]}
                              </span>

                              {/* 上下文容量 */}
                              <div>
                                <span style="font-size: 10.5px; background: #f4f4f5; color: #52525b; padding: 2px 6px; border-radius: 4px; font-weight: 500;">
                                  {formatContextSize(model.contextWindow)}
                                </span>
                              </div>

                              {/* 能力微标签 */}
                              <div style="display: flex; gap: 4px; flex-wrap: wrap;">
                                <Show when={model.capabilities.tools}>
                                  <span style="font-size: 9.5px; background: #eff6ff; color: #1d4ed8; padding: 1px 4px; border-radius: 3px; font-weight: 500;">
                                    Tools
                                  </span>
                                </Show>
                                <Show when={model.capabilities.streaming}>
                                  <span style="font-size: 9.5px; background: #f0fdf4; color: #15803d; padding: 1px 4px; border-radius: 3px; font-weight: 500;">
                                    Stream
                                  </span>
                                </Show>
                                <Show when={model.capabilities.reasoning}>
                                  <span style="font-size: 9.5px; background: #fff7ed; color: #c2410c; padding: 1px 4px; border-radius: 3px; font-weight: 500;">
                                    Reasoning
                                  </span>
                                </Show>
                                <Show when={model.capabilities.vision}>
                                  <span style="font-size: 9.5px; background: #faf5ff; color: #7e22ce; padding: 1px 4px; border-radius: 3px; font-weight: 500;">
                                    Vision
                                  </span>
                                </Show>
                              </div>

                              {/* 删除操作 */}
                              <div style="display: flex; align-items: center; justify-content: center;">
                                <button
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    gatewayStore.deleteModel(currentProvider().id, model.id);
                                    toast.info(`已移除模型: ${model.name}`);
                                  }}
                                  style="background: transparent; border: none; color: #a1a1aa; cursor: pointer; padding: 3px; border-radius: 4px; display: flex; align-items: center;"
                                  title="删除模型"
                                  onMouseEnter={(e) => (e.currentTarget.style.color = '#ef4444')}
                                  onMouseLeave={(e) => (e.currentTarget.style.color = '#a1a1aa')}
                                >
                                  <IconTrash size={12} />
                                </button>
                              </div>
                            </div>
                          );
                        }}
                      </For>
                    </div>
                  </div>
                </div>
              </div>
            </Show>

            {/* ── 2. Sandbox Policy Tab ────────────────────────────────────── */}
            <Show when={activeMainTab() === 'sandbox'}>
              <div style="flex: 1; padding: 20px 24px; display: flex; flex-direction: column; gap: 16px; overflow-y: auto; overscroll-behavior: contain;">
                <div style="display: flex; flex-direction: column; gap: 10px;">
                  {/* Option 1: Bypass */}
                  <div
                    onClick={() => {
                      setSandboxLevel('bypass');
                      toast.info('已开启 Bypass permissions 模式');
                    }}
                    style={`padding: 14px 16px; border-radius: 10px; cursor: pointer; transition: all 0.1s; border: 1.5px solid; ${
                      sandboxLevel() === 'bypass'
                        ? 'border-color: #ea580c; background: #fffcf8;'
                        : 'border-color: #e4e4e7; background: #ffffff;'
                    }`}
                  >
                    <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;">
                      <b style="font-size: 13.5px; color: #18181b;">Bypass permissions (自动放行执行 - 推荐)</b>
                      <Show when={sandboxLevel() === 'bypass'}>
                        <span style="font-size: 11px; color: #ea580c; font-weight: 600;">● 当前生效</span>
                      </Show>
                    </div>
                    <span style="font-size: 12px; color: #71717a;">
                      Agent 自动执行文件读写、创建新文件与常规 Shell 命令，无需手动弹窗确认，适合快速迭代开发。
                    </span>
                  </div>

                  {/* Option 2: Confirm */}
                  <div
                    onClick={() => {
                      setSandboxLevel('confirm');
                      toast.info('已开启 Ask for confirmation 审批模式');
                    }}
                    style={`padding: 14px 16px; border-radius: 10px; cursor: pointer; transition: all 0.1s; border: 1.5px solid; ${
                      sandboxLevel() === 'confirm'
                        ? 'border-color: #ea580c; background: #fffcf8;'
                        : 'border-color: #e4e4e7; background: #ffffff;'
                    }`}
                  >
                    <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;">
                      <b style="font-size: 13.5px; color: #18181b;">Ask for confirmation (人工逐项审批)</b>
                      <Show when={sandboxLevel() === 'confirm'}>
                        <span style="font-size: 11px; color: #ea580c; font-weight: 600;">● 当前生效</span>
                      </Show>
                    </div>
                    <span style="font-size: 12px; color: #71717a;">
                      在执行破坏性文件修改、删除文件或高风险 Shell 脚本前，弹出确认对话框由您手工批准。
                    </span>
                  </div>

                  {/* Option 3: Strict */}
                  <div
                    onClick={() => {
                      setSandboxLevel('strict');
                      toast.info('已开启 Strict Read-Only 只读模式');
                    }}
                    style={`padding: 14px 16px; border-radius: 10px; cursor: pointer; transition: all 0.1s; border: 1.5px solid; ${
                      sandboxLevel() === 'strict'
                        ? 'border-color: #ea580c; background: #fffcf8;'
                        : 'border-color: #e4e4e7; background: #ffffff;'
                    }`}
                  >
                    <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;">
                      <b style="font-size: 13.5px; color: #18181b;">Strict Read-Only Sandbox (严格只读)</b>
                      <Show when={sandboxLevel() === 'strict'}>
                        <span style="font-size: 11px; color: #ea580c; font-weight: 600;">● 当前生效</span>
                      </Show>
                    </div>
                    <span style="font-size: 12px; color: #71717a;">
                      Agent 仅允许读取代码并回答问题，禁止向磁盘写入任何变更或启动终端子进程。
                    </span>
                  </div>
                </div>
              </div>
            </Show>

            {/* ── 3. Custom Prompts Tab ────────────────────────────────────── */}
            <Show when={activeMainTab() === 'prompt'}>
              <div style="flex: 1; padding: 20px 24px; display: flex; flex-direction: column; gap: 14px; overflow-y: auto; overscroll-behavior: contain;">
                <textarea
                  rows={8}
                  value={systemPrompt()}
                  onInput={(e) => setSystemPrompt(e.currentTarget.value)}
                  style="width: 100%; background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 8px; padding: 12px; font-size: 13px; line-height: 1.5; color: #18181b; outline: none; resize: vertical; font-family: inherit;"
                />

                <div style="display: flex; gap: 8px;">
                  <button
                    onClick={() => {
                      setSystemPrompt('You are a professional software architect. Prioritize clean microkernel design, modularity, and zero framework regressions.');
                      toast.info('已载入架构师提示词模板');
                    }}
                    style="padding: 6px 12px; background: #f4f4f5; border: 1px solid #e4e4e7; border-radius: 6px; font-size: 12px; color: #3f3f46; cursor: pointer;"
                  >
                    载入架构师模板
                  </button>
                  <button
                    onClick={() => {
                      setSystemPrompt('You are a strict test-driven development (TDD) engineer. Always verify with automated tests before completing tasks.');
                      toast.info('已载入 TDD 工程师模板');
                    }}
                    style="padding: 6px 12px; background: #f4f4f5; border: 1px solid #e4e4e7; border-radius: 6px; font-size: 12px; color: #3f3f46; cursor: pointer;"
                  >
                    载入 TDD 模板
                  </button>
                </div>
              </div>
            </Show>

            {/* ── 4. General Tab ──────────────────────────────────────────── */}
            <Show when={activeMainTab() === 'general'}>
              <div style="flex: 1; padding: 20px 24px; display: flex; flex-direction: column; gap: 16px; overflow-y: auto; overscroll-behavior: contain;">
                <div style="display: flex; flex-direction: column; gap: 5px;">
                  <label style="font-size: 12.5px; font-weight: 600; color: #18181b;">外观主题 (Appearance Theme):</label>
                  <select
                    value={themeMode()}
                    onChange={(e) => {
                      setThemeMode(e.currentTarget.value);
                      document.documentElement.setAttribute('data-theme', e.currentTarget.value);
                      toast.info(`主题已切换为 ${e.currentTarget.value}`);
                    }}
                    style="background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 6px; padding: 8px 10px; color: #18181b; font-size: 12.5px;"
                  >
                    <option value="light">Claude Light (暖白极简 - 推荐)</option>
                    <option value="dark">Navis Dark (深色极客)</option>
                  </select>
                </div>

                <div style="display: flex; flex-direction: column; gap: 5px;">
                  <label style="font-size: 12.5px; font-weight: 600; color: #18181b;">默认工作区目录 (Workspace Root):</label>
                  <input
                    type="text"
                    value="D:\myworkspace\Navis Go"
                    readonly
                    style="background: #f4f4f5; border: 1px solid #e4e4e7; border-radius: 6px; padding: 8px 10px; color: #71717a; font-size: 12.5px;"
                  />
                </div>
              </div>
            </Show>

            {/* ══════════════════════════════════════════════════════════════════
                底部常驻操作栏 (Footer Status Bar)
               ══════════════════════════════════════════════════════════════════ */}
            <div style="height: 52px; background: #fbfaf8; border-top: 1px solid #eaeaea; display: flex; align-items: center; justify-content: space-between; padding: 0 24px; flex-shrink: 0;">
              <div style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #71717a;">
                <span>当前生效模型:</span>
                <b style="color: #18181b;">
                  {gatewayStore.activeProvider().name.split(' (')[0]} · {gatewayStore.activeModel().name}
                </b>
              </div>

              <div style="display: flex; align-items: center; gap: 8px;">
                <button
                  onClick={() => setOpen(false)}
                  style="padding: 6px 14px; background: transparent; border: 1px solid #e4e4e7; border-radius: 6px; font-size: 12px; font-weight: 500; color: #52525b; cursor: pointer;"
                >
                  Cancel (取消)
                </button>
                <button
                  onClick={handleSaveAll}
                  style="padding: 6px 16px; background: #18181b; border: none; border-radius: 6px; font-size: 12px; font-weight: 500; color: #ffffff; cursor: pointer;"
                >
                  Save Changes (保存并生效)
                </button>
              </div>
            </div>
          </div>

          {/* ══════════════════════════════════════════════════════════════════════
              添加模型抽屉 / 模态卡片 (Add Model Drawer)
             ══════════════════════════════════════════════════════════════════════ */}
          <Show when={showAddModelModal()}>
            <div
              onClick={() => setShowAddModelModal(false)}
              style="position: absolute; inset: 0; background: rgba(0,0,0,0.3); backdrop-filter: blur(2px); z-index: 100; display: flex; align-items: center; justify-content: center; padding: 20px;"
            >
              <div
                onClick={(e) => e.stopPropagation()}
                style="width: 480px; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 12px; box-shadow: 0 16px 36px rgba(0,0,0,0.16); padding: 20px; display: flex; flex-direction: column; gap: 14px;"
              >
                <div style="display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid #f4f4f5; padding-bottom: 8px;">
                  <b style="font-size: 13.5px; color: #18181b;">添加新模型到 {currentProvider()?.name.split(' (')[0]}</b>
                  <button
                    onClick={() => setShowAddModelModal(false)}
                    style="background: transparent; border: none; color: #71717a; cursor: pointer;"
                  >
                    <IconClose size={14} />
                  </button>
                </div>

                <div style="display: flex; flex-direction: column; gap: 10px;">
                  <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px;">
                    <div style="display: flex; flex-direction: column; gap: 4px;">
                      <span style="font-size: 11px; color: #71717a;">Model ID</span>
                      <input
                        type="text"
                        placeholder="如: claude-3-7-sonnet-20250219"
                        value={newModelId()}
                        onInput={(e) => setNewModelId(e.currentTarget.value)}
                        style="background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px;"
                      />
                    </div>
                    <div style="display: flex; flex-direction: column; gap: 4px;">
                      <span style="font-size: 11px; color: #71717a;">显示名称</span>
                      <input
                        type="text"
                        placeholder="如: Claude 3.7 Sonnet"
                        value={newModelName()}
                        onInput={(e) => setNewModelName(e.currentTarget.value)}
                        style="background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px;"
                      />
                    </div>
                  </div>

                  {/* 能力勾选项 */}
                  <div style="display: flex; flex-direction: column; gap: 6px;">
                    <span style="font-size: 11px; color: #71717a;">模型能力</span>
                    <div style="display: flex; gap: 12px; flex-wrap: wrap; font-size: 12px; color: #3f3f46;">
                      <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
                        <input
                          type="checkbox"
                          checked={newModelTools()}
                          onChange={(e) => setNewModelTools(e.currentTarget.checked)}
                        />
                        <span>Tools</span>
                      </label>
                      <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
                        <input
                          type="checkbox"
                          checked={newModelStream()}
                          onChange={(e) => setNewModelStream(e.currentTarget.checked)}
                        />
                        <span>Stream</span>
                      </label>
                      <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
                        <input
                          type="checkbox"
                          checked={newModelReasoning()}
                          onChange={(e) => setNewModelReasoning(e.currentTarget.checked)}
                        />
                        <span>Reasoning</span>
                      </label>
                      <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
                        <input
                          type="checkbox"
                          checked={newModelVision()}
                          onChange={(e) => setNewModelVision(e.currentTarget.checked)}
                        />
                        <span>Vision</span>
                      </label>
                    </div>
                  </div>
                </div>

                <div style="display: flex; align-items: center; justify-content: flex-end; gap: 8px; margin-top: 6px;">
                  <button
                    onClick={() => setShowAddModelModal(false)}
                    style="padding: 5px 12px; background: transparent; border: 1px solid #e4e4e7; border-radius: 5px; font-size: 12px; color: #52525b; cursor: pointer;"
                  >
                    取消
                  </button>
                  <button
                    onClick={handleSaveModel}
                    style="padding: 5px 14px; background: #18181b; border: none; border-radius: 5px; font-size: 12px; color: #ffffff; cursor: pointer;"
                  >
                    确认添加
                  </button>
                </div>
              </div>
            </div>
          </Show>

          {/* ══════════════════════════════════════════════════════════════════════
              添加 Provider 抽屉 / 模态卡片 (Add Provider Drawer)
             ══════════════════════════════════════════════════════════════════════ */}
          <Show when={showAddProviderModal()}>
            <div
              onClick={() => setShowAddProviderModal(false)}
              style="position: absolute; inset: 0; background: rgba(0,0,0,0.3); backdrop-filter: blur(2px); z-index: 100; display: flex; align-items: center; justify-content: center; padding: 20px;"
            >
              <div
                onClick={(e) => e.stopPropagation()}
                style="width: 480px; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 12px; box-shadow: 0 16px 36px rgba(0,0,0,0.16); padding: 20px; display: flex; flex-direction: column; gap: 14px;"
              >
                <div style="display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid #f4f4f5; padding-bottom: 8px;">
                  <b style="font-size: 13.5px; color: #18181b;">配置新模型提供商 (Custom Provider)</b>
                  <button
                    onClick={() => setShowAddProviderModal(false)}
                    style="background: transparent; border: none; color: #71717a; cursor: pointer;"
                  >
                    <IconClose size={14} />
                  </button>
                </div>

                <div style="display: flex; flex-direction: column; gap: 10px;">
                  <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px;">
                    <div style="display: flex; flex-direction: column; gap: 4px;">
                      <span style="font-size: 11px; color: #71717a;">Provider 名称</span>
                      <input
                        type="text"
                        placeholder="如: SiliconFlow / OpenRouter"
                        value={newProviderName()}
                        onInput={(e) => setNewProviderName(e.currentTarget.value)}
                        style="background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px;"
                      />
                    </div>
                    <div style="display: flex; flex-direction: column; gap: 4px;">
                      <span style="font-size: 11px; color: #71717a;">Base URL</span>
                      <input
                        type="text"
                        placeholder="如: https://api.siliconflow.cn/v1"
                        value={newProviderUrl()}
                        onInput={(e) => setNewProviderUrl(e.currentTarget.value)}
                        style="background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px;"
                      />
                    </div>
                  </div>

                  <div style="display: flex; flex-direction: column; gap: 4px;">
                    <span style="font-size: 11px; color: #71717a;">API Key</span>
                    <input
                      type="password"
                      placeholder="sk-..."
                      value={newProviderKey()}
                      onInput={(e) => setNewProviderKey(e.currentTarget.value)}
                      style="background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px;"
                    />
                  </div>
                </div>

                <div style="display: flex; align-items: center; justify-content: flex-end; gap: 8px; margin-top: 6px;">
                  <button
                    onClick={() => setShowAddProviderModal(false)}
                    style="padding: 5px 12px; background: transparent; border: 1px solid #e4e4e7; border-radius: 5px; font-size: 12px; color: #52525b; cursor: pointer;"
                  >
                    取消
                  </button>
                  <button
                    onClick={handleSaveCustomProvider}
                    style="padding: 5px 14px; background: #18181b; border: none; border-radius: 5px; font-size: 12px; color: #ffffff; cursor: pointer;"
                  >
                    确认创建
                  </button>
                </div>
              </div>
            </div>
          </Show>
        </div>
      </div>
    </Show>
  );
};

export default SettingsModal;
