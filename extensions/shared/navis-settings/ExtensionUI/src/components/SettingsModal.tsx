import { Component, createSignal, onMount, onCleanup, Show, For, Index } from 'solid-js';
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

  // 新增 Provider 抽屉状态
  const [showAddProviderModal, setShowAddProviderModal] = createSignal(false);
  const [newProviderName, setNewProviderName] = createSignal('');
  const [newProviderUrl, setNewProviderUrl] = createSignal('http://127.0.0.1:8046/v1');
  const [newProviderKey, setNewProviderKey] = createSignal('');
  const [newProviderProtocol, setNewProviderProtocol] = createSignal<'responses' | 'chat_completions' | 'anthropic_messages'>('responses');

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
        if (showAddProviderModal()) {
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
      toast.success(`连接测试成功！延迟: ${res.pingMs}ms`);
    } else {
      toast.error(`连接失败，请检查 Base URL 和 API Key`);
    }
  };

  const handleSyncModels = async () => {
    const p = currentProvider();
    if (!p) return;
    setIsSyncing(true);
    toast.info(`正在连接 ${p.baseUrl}/v1/models 获取模型列表...`);
    const list = await gatewayStore.fetchModels(p.id);
    setIsSyncing(false);
    toast.success(`获取成功！已将 ${list.length} 个可用模型同步填充至下拉列表`);
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
      upstreamProtocol: newProviderProtocol(),
      baseUrl: newProviderUrl().trim(),
      apiKey: newProviderKey().trim(),
      status: 'connected',
      defaultModelId: `${customId}-model-1`,
      models: [
        {
          id: `${customId}-model-1`,
          name: `${newProviderName().trim()} Default Model`,
          providerId: customId,
          apiProtocol: newProviderProtocol() === 'anthropic_messages' ? 'anthropic_messages' : 'chat_completions',
          contextWindow: 128000,
          maxOutputTokens: 8192,
          thinkingLevel: 'none',
          capabilities: { tools: true, streaming: true, vision: false, reasoning: false },
          isDefault: true,
        },
      ],
    };

    gatewayStore.addCustomProvider(newProv);
    setSelectedProviderId(customId);
    setShowAddProviderModal(false);
    setNewProviderName('');
    setNewProviderUrl('http://127.0.0.1:8046/v1');
    setNewProviderKey('');
    toast.success(`已添加自定义 Provider: ${newProv.name}`);
  };

  const handleCopyKey = () => {
    const key = currentProvider()?.apiKey || '';
    if (!key) {
      toast.warning('当前 Provider 暂未配置 API Key');
      return;
    }
    navigator.clipboard?.writeText(key);
    toast.success('API Key 已复制到剪贴板');
  };

  const handleSaveAll = () => {
    setOpen(false);
    toast.success('配置已保存并实时生效！');
  };

  return (
    <Show when={open()}>
      <div
        onClick={() => setOpen(false)}
        style="position: fixed; inset: 0; background: rgba(0,0,0,0.45); backdrop-filter: blur(4px); z-index: 9999; display: flex; align-items: center; justify-content: center; padding: 20px; pointer-events: auto;"
      >
        <div
          onClick={(e) => e.stopPropagation()}
          style="width: 1040px; max-width: 96vw; height: 680px; max-height: 92vh; background: #ffffff; border: 1px solid #e5e5e5; border-radius: 14px; box-shadow: 0 24px 48px -12px rgba(0,0,0,0.18); overflow: hidden; display: flex; flex-direction: row; animation: navis-pop 0.15s ease-out; pointer-events: auto; position: relative;"
        >
          {/* ══════════════════════════════════════════════════════════════════════
              第 1 栏：左侧全局大类导航 (180px)
             ══════════════════════════════════════════════════════════════════════ */}
          <div style="width: 180px; background: #f8f8f7; border-right: 1px solid #eaeaea; display: flex; flex-direction: column; justify-content: space-between; padding: 16px 10px; flex-shrink: 0;">
            <div style="display: flex; flex-direction: column; gap: 4px;">
              <div style="display: flex; align-items: center; gap: 8px; padding: 4px 8px 12px; border-bottom: 1px solid #ebeaea; margin-bottom: 6px;">
                <span style="color: #c2410c; display: flex; align-items: center;">
                  <IconSettings size={17} />
                </span>
                <span style="font-size: 13.5px; font-weight: 600; color: #18181b;">Navis Settings</span>
              </div>

              <button
                data-tab-id="models"
                onClick={() => setActiveMainTab('models')}
                style={`display: flex; align-items: center; gap: 8px; padding: 7px 10px; border-radius: 7px; border: none; font-size: 12.5px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s; ${
                  activeMainTab() === 'models' ? 'background: #eceae5; color: #18181b; font-weight: 600;' : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconCpu size={15} />
                <span>AI Providers</span>
              </button>

              <button
                data-tab-id="sandbox"
                onClick={() => setActiveMainTab('sandbox')}
                style={`display: flex; align-items: center; gap: 8px; padding: 7px 10px; border-radius: 7px; border: none; font-size: 12.5px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s; ${
                  activeMainTab() === 'sandbox' ? 'background: #eceae5; color: #18181b; font-weight: 600;' : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconShield size={15} />
                <span>Permissions</span>
              </button>

              <button
                data-tab-id="prompt"
                onClick={() => setActiveMainTab('prompt')}
                style={`display: flex; align-items: center; gap: 8px; padding: 7px 10px; border-radius: 7px; border: none; font-size: 12.5px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s; ${
                  activeMainTab() === 'prompt' ? 'background: #eceae5; color: #18181b; font-weight: 600;' : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconPrompt size={15} />
                <span>Prompts</span>
              </button>

              <button
                data-tab-id="general"
                onClick={() => setActiveMainTab('general')}
                style={`display: flex; align-items: center; gap: 8px; padding: 7px 10px; border-radius: 7px; border: none; font-size: 12.5px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s; ${
                  activeMainTab() === 'general' ? 'background: #eceae5; color: #18181b; font-weight: 600;' : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconPalette size={15} />
                <span>General</span>
              </button>
            </div>

            <div style="border-top: 1px solid #ebeaea; padding-top: 8px;">
              <button
                onClick={() => setOpen(false)}
                style="display: flex; align-items: center; justify-content: center; gap: 5px; width: 100%; padding: 6px; background: transparent; border: 1px solid #e5e5e5; border-radius: 6px; font-size: 11.5px; color: #71717a; cursor: pointer;"
              >
                <IconClose size={13} />
                <span>关闭 (Close)</span>
              </button>
            </div>
          </div>

          {/* ══════════════════════════════════════════════════════════════════════
              第 2 栏：中间 Provider 列表区 (220px，仅在 AI Providers Tab 展现)
             ══════════════════════════════════════════════════════════════════════ */}
          <Show when={activeMainTab() === 'models'}>
            <div style="width: 220px; background: #fafaf9; border-right: 1px solid #eaeaea; display: flex; flex-direction: column; justify-content: space-between; padding: 16px 12px; flex-shrink: 0;">
              <div style="display: flex; flex-direction: column; gap: 8px;">
                <div style="display: flex; align-items: center; justify-content: space-between; padding-bottom: 6px; border-bottom: 1px solid #ebeaea;">
                  <span style="font-size: 12px; font-weight: 600; color: #52525b;">Providers (服务商)</span>
                  <span style="font-size: 11px; color: #a1a1aa;">{gatewayStore.providers().length}</span>
                </div>

                <div style="display: flex; flex-direction: column; gap: 6px; max-height: 520px; overflow-y: auto; overscroll-behavior: contain;">
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
                          style={`padding: 10px 12px; border-radius: 8px; cursor: pointer; display: flex; flex-direction: column; gap: 4px; border: 1px solid; transition: all 0.1s; ${
                            isSelected()
                              ? 'background: #ffffff; border-color: #18181b; box-shadow: 0 2px 6px rgba(0,0,0,0.06);'
                              : 'background: #f4f4f3; border-color: #e5e5e5;'
                          }`}
                        >
                          <div style="display: flex; align-items: center; justify-content: space-between;">
                            <b style={`font-size: 12.5px; ${isSelected() ? 'color: #18181b;' : 'color: #3f3f46;'}`}>
                              {p.name.split(' (')[0]}
                            </b>
                            <Show when={isGlobalActive()}>
                              <span style="font-size: 10px; background: #dcfce7; color: #15803d; padding: 1px 5px; border-radius: 4px; font-weight: 600;">
                                Active
                              </span>
                            </Show>
                          </div>
                          <div style="display: flex; align-items: center; justify-content: space-between; font-size: 11px; color: #71717a;">
                            <span>{p.models.length} 个模型</span>
                            <span>{p.upstreamProtocol || 'responses'}</span>
                          </div>
                        </div>
                      );
                    }}
                  </For>
                </div>
              </div>

              {/* 底部添加服务商按钮 */}
              <div style="border-top: 1px solid #ebeaea; padding-top: 8px;">
                <button
                  onClick={() => setShowAddProviderModal(true)}
                  style="display: flex; align-items: center; justify-content: center; gap: 5px; width: 100%; padding: 7px; background: #ffffff; border: 1px dashed #d4d4d8; border-radius: 6px; font-size: 12px; font-weight: 500; color: #ea580c; cursor: pointer;"
                >
                  <IconPlus size={13} />
                  <span>添加 Provider</span>
                </button>
              </div>
            </div>
          </Show>

          {/* ══════════════════════════════════════════════════════════════════════
              第 3 栏：右侧 Provider 专属配置与模型映射详情面板
             ══════════════════════════════════════════════════════════════════════ */}
          <div style="flex: 1; display: flex; flex-direction: column; background: #ffffff; overflow: hidden; min-height: 0;">
            {/* 顶栏标题与关闭 ✕ */}
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 14px 24px; border-bottom: 1px solid #f0eee8; flex-shrink: 0;">
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="font-size: 15px; font-weight: 600; color: #18181b;">
                  {activeMainTab() === 'models'
                    ? `${currentProvider()?.name.split(' (')[0]} · 配置详情`
                    : activeMainTab() === 'sandbox'
                    ? 'Permissions & Sandbox'
                    : activeMainTab() === 'prompt'
                    ? 'System Instructions & Custom Prompts'
                    : 'General Settings'}
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

            {/* ── 1. Models & Provider Tab 内容区 ─────────────────────────────── */}
            <Show when={activeMainTab() === 'models'}>
              <div style="flex: 1; display: flex; flex-direction: column; overflow-y: auto; padding: 18px 24px; gap: 14px; overscroll-behavior: contain; min-height: 0;">
                {/* 1.1 API 请求地址 (第一行) */}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                  <div style="display: flex; align-items: center; justify-content: space-between;">
                    <label style="font-size: 12.5px; font-weight: 600; color: #18181b;">API 请求地址</label>
                    <button
                      onClick={handleTestCurrentProvider}
                      disabled={isTesting()}
                      style="padding: 3px 8px; background: #ffffff; border: 1px solid #d4d4d8; border-radius: 5px; font-size: 11.5px; color: #3f3f46; cursor: pointer; display: flex; align-items: center; gap: 4px;"
                    >
                      <IconZap size={12} color="#ea580c" />
                      <span>{isTesting() ? '测速中...' : '管理与测速'}</span>
                    </button>
                  </div>
                  <input
                    type="text"
                    value={currentProvider()?.baseUrl || ''}
                    onInput={(e) => gatewayStore.updateProvider(currentProvider().id, { baseUrl: e.currentTarget.value })}
                    style="width: 100%; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 7px 10px; font-size: 13px; color: #18181b; outline: none;"
                  />
                </div>

                {/* 1.2 API Key (第二行，零多余小字) */}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                  <label style="font-size: 12.5px; font-weight: 600; color: #18181b;">API Key</label>
                  <div style="display: flex; align-items: center; gap: 6px; position: relative;">
                    <input
                      type={showApiKey() ? 'text' : 'password'}
                      value={currentProvider()?.apiKey || ''}
                      onInput={(e) => gatewayStore.updateProvider(currentProvider().id, { apiKey: e.currentTarget.value })}
                      placeholder="sk-..."
                      style="flex: 1; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 7px 36px 7px 10px; font-size: 13px; color: #18181b; outline: none;"
                    />
                    <button
                      onClick={() => setShowApiKey(!showApiKey())}
                      style="position: absolute; right: 8px; background: transparent; border: none; font-size: 12px; color: #71717a; cursor: pointer; display: flex; align-items: center;"
                      title={showApiKey() ? '隐藏 Key' : '显示 Key'}
                    >
                      {showApiKey() ? <IconEyeOff size={14} /> : <IconEye size={14} />}
                    </button>
                  </div>
                </div>

                {/* 1.3 默认模型 (第三行，下拉选择框，不选默认使用第一项) */}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                  <label style="font-size: 12.5px; font-weight: 600; color: #18181b;">默认模型</label>
                  <select
                    value={currentProvider()?.defaultModelId || currentProvider()?.models[0]?.id || ''}
                    onChange={(e) => {
                      gatewayStore.updateProvider(currentProvider().id, { defaultModelId: e.currentTarget.value });
                      gatewayStore.setActiveModel(e.currentTarget.value);
                      toast.success(`默认模型已切换为: ${e.currentTarget.value}`);
                    }}
                    style="width: 100%; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 7px 10px; font-size: 12.5px; color: #18181b; outline: none;"
                  >
                    <For each={currentProvider()?.models}>
                      {(m, index) => (
                        <option value={m.id}>
                          {m.name || m.id} {index() === 0 ? '(默认首项)' : ''}
                        </option>
                      )}
                    </For>
                  </select>
                </div>

                {/* 1.4 上游模式 (第四行，下拉选择框) */}
                <div style="display: flex; flex-direction: column; gap: 6px;">
                  <label style="font-size: 12.5px; font-weight: 600; color: #18181b;">上游模式</label>
                  <select
                    value={currentProvider()?.upstreamProtocol || 'responses'}
                    onChange={(e) => {
                      gatewayStore.updateProvider(currentProvider().id, { upstreamProtocol: e.currentTarget.value as any });
                      toast.info(`上游模式已变更为: ${e.currentTarget.value}`);
                    }}
                    style="width: 100%; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 7px 10px; font-size: 12.5px; color: #18181b; outline: none;"
                  >
                    <option value="responses">Responses (原生)</option>
                    <option value="chat_completions">Chat Completions (OpenAI 兼容)</option>
                    <option value="anthropic_messages">Anthropic Messages</option>
                  </select>
                </div>

                {/* 1.5 模型映射 (第五行，数据表格 + 获取/添加按钮) */}
                <div style="display: flex; flex-direction: column; gap: 8px; margin-top: 4px;">
                  <div style="display: flex; align-items: center; justify-content: space-between;">
                    <label style="font-size: 12.5px; font-weight: 600; color: #18181b;">
                      模型映射 ({currentProvider()?.models.length || 0})
                    </label>
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <button
                        onClick={handleSyncModels}
                        disabled={isSyncing()}
                        style="padding: 4px 10px; background: #ffffff; border: 1px solid #d4d4d8; border-radius: 5px; font-size: 11.5px; color: #2563eb; cursor: pointer; display: flex; align-items: center; gap: 4px; font-weight: 500;"
                      >
                        <IconRefresh size={12} color="#2563eb" />
                        <span>{isSyncing() ? '获取中...' : '获取模型列表'}</span>
                      </button>
                      <button
                        onClick={() => {
                          gatewayStore.addEmptyModel(currentProvider().id);
                          toast.success('已追加一行新模型条目');
                        }}
                        style="padding: 4px 10px; background: #ffffff; border: 1px solid #d4d4d8; border-radius: 5px; font-size: 11.5px; color: #ea580c; cursor: pointer; display: flex; align-items: center; gap: 4px; font-weight: 500;"
                      >
                        <IconPlus size={12} color="#ea580c" />
                        <span>添加模型</span>
                      </button>
                    </div>
                  </div>

                  {/* 模型映射数据表格 */}
                  <div style="border: 1px solid #e4e4e7; border-radius: 8px; overflow: hidden; background: #ffffff;">
                    {/* 表头 */}
                    <div style="display: grid; grid-template-columns: 1.2fr 1.3fr 90px 85px 36px; background: #fafaf9; border-bottom: 1px solid #e4e4e7; padding: 7px 10px; font-size: 11.5px; font-weight: 600; color: #71717a; gap: 6px;">
                      <span>菜单显示名</span>
                      <span>实际请求模型</span>
                      <span>上下文窗口</span>
                      <span>思考等级</span>
                      <span style="text-align: center;">操作</span>
                    </div>

                    {/* 表体行 */}
                    <div style="max-height: 220px; overflow-y: auto; overscroll-behavior: contain;">
                      <Index each={currentProvider()?.models}>
                        {(model, index) => (
                          <div
                            style="display: grid; grid-template-columns: 1.2fr 1.3fr 90px 85px 36px; align-items: center; padding: 6px 10px; border-bottom: 1px solid #f4f4f5; gap: 6px;"
                          >
                            {/* 菜单显示名 */}
                            <input
                              type="text"
                              value={model().name}
                              onInput={(e) =>
                                gatewayStore.updateModel(currentProvider().id, model().id, { name: e.currentTarget.value })
                              }
                              style="width: 100%; min-width: 0; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 5px; padding: 5px 8px; font-size: 12.5px; color: #18181b; outline: none;"
                            />

                            {/* 实际请求模型 (下拉选择框，获取模型后自动填充) */}
                            <select
                              value={model().id}
                              onChange={(e) => {
                                const selectedId = e.currentTarget.value;
                                const oldId = model().id;
                                gatewayStore.updateModel(currentProvider().id, oldId, {
                                  id: selectedId,
                                  name: model().name === oldId || !model().name ? selectedId : model().name,
                                });
                              }}
                              style="width: 100%; min-width: 0; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 5px; padding: 5px 8px; font-size: 12.5px; color: #18181b; outline: none;"
                            >
                              <For
                                each={Array.from(
                                  new Set([
                                    model().id,
                                    ...(currentProvider()?.fetchedModelIds || []),
                                    ...(currentProvider()?.models.map((m) => m.id) || []),
                                  ]),
                                )}
                              >
                                {(optId) => <option value={optId}>{optId}</option>}
                              </For>
                            </select>

                            {/* 上下文窗口 */}
                            <input
                              type="number"
                              value={model().contextWindow || 128000}
                              onInput={(e) =>
                                gatewayStore.updateModel(currentProvider().id, model().id, {
                                  contextWindow: Number(e.currentTarget.value) || 128000,
                                })
                              }
                              style="width: 100%; min-width: 0; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 5px; padding: 5px 8px; font-size: 12.5px; color: #18181b; outline: none;"
                            />

                            {/* 思考等级 */}
                            <select
                              value={model().thinkingLevel || 'none'}
                              onChange={(e) =>
                                gatewayStore.updateModel(currentProvider().id, model().id, {
                                  thinkingLevel: e.currentTarget.value as any,
                                })
                              }
                              style="width: 100%; min-width: 0; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 5px; padding: 5px 6px; font-size: 12px; color: #18181b; outline: none;"
                            >
                              <option value="none">未设置</option>
                              <option value="low">low</option>
                              <option value="medium">medium</option>
                              <option value="high">high</option>
                            </select>

                            {/* 删除操作 */}
                            <div style="display: flex; align-items: center; justify-content: center;">
                              <button
                                onClick={() => {
                                  gatewayStore.deleteModel(currentProvider().id, model().id);
                                  toast.info(`已移除模型: ${model().name}`);
                                }}
                                style="background: transparent; border: none; color: #a1a1aa; cursor: pointer; padding: 3px; border-radius: 4px; display: flex; align-items: center;"
                                title="删除此映射"
                                onMouseEnter={(e) => (e.currentTarget.style.color = '#ef4444')}
                                onMouseLeave={(e) => (e.currentTarget.style.color = '#a1a1aa')}
                              >
                                <IconTrash size={13} />
                              </button>
                            </div>
                          </div>
                        )}
                      </Index>
                    </div>
                  </div>
                </div>
              </div>
            </Show>

            {/* ── 2. Sandbox Policy Tab ────────────────────────────────────── */}
            <Show when={activeMainTab() === 'sandbox'}>
              <div style="flex: 1; padding: 20px 24px; display: flex; flex-direction: column; gap: 14px; overflow-y: auto; overscroll-behavior: contain;">
                <div
                  onClick={() => {
                    setSandboxLevel('bypass');
                    toast.info('已开启 Bypass permissions 模式');
                  }}
                  style={`padding: 14px 16px; border-radius: 10px; cursor: pointer; transition: all 0.1s; border: 1.5px solid; ${
                    sandboxLevel() === 'bypass' ? 'border-color: #ea580c; background: #fffcf8;' : 'border-color: #e4e4e7; background: #ffffff;'
                  }`}
                >
                  <b style="font-size: 13.5px; color: #18181b;">Bypass permissions (自动放行执行 - 推荐)</b>
                  <div style="font-size: 12px; color: #71717a; margin-top: 3px;">
                    Agent 自动执行文件读写、创建新文件与常规 Shell 命令，无需手动逐项确认。
                  </div>
                </div>

                <div
                  onClick={() => {
                    setSandboxLevel('confirm');
                    toast.info('已开启 Ask for confirmation 审批模式');
                  }}
                  style={`padding: 14px 16px; border-radius: 10px; cursor: pointer; transition: all 0.1s; border: 1.5px solid; ${
                    sandboxLevel() === 'confirm' ? 'border-color: #ea580c; background: #fffcf8;' : 'border-color: #e4e4e7; background: #ffffff;'
                  }`}
                >
                  <b style="font-size: 13.5px; color: #18181b;">Ask for confirmation (人工逐项审批)</b>
                  <div style="font-size: 12px; color: #71717a; margin-top: 3px;">
                    在执行高风险文件修改或 Shell 脚本前，弹出确认对话框由人工审批。
                  </div>
                </div>

                <div
                  onClick={() => {
                    setSandboxLevel('strict');
                    toast.info('已开启 Strict Read-Only 只读模式');
                  }}
                  style={`padding: 14px 16px; border-radius: 10px; cursor: pointer; transition: all 0.1s; border: 1.5px solid; ${
                    sandboxLevel() === 'strict' ? 'border-color: #ea580c; background: #fffcf8;' : 'border-color: #e4e4e7; background: #ffffff;'
                  }`}
                >
                  <b style="font-size: 13.5px; color: #18181b;">Strict Read-Only Sandbox (严格只读)</b>
                  <div style="font-size: 12px; color: #71717a; margin-top: 3px;">
                    Agent 仅允许读取代码并回答问题，禁止向磁盘写入任何变更或启动终端子进程。
                  </div>
                </div>
              </div>
            </Show>

            {/* ── 3. Custom Prompts Tab ────────────────────────────────────── */}
            <Show when={activeMainTab() === 'prompt'}>
              <div style="flex: 1; padding: 20px 24px; display: flex; flex-direction: column; gap: 14px; overflow-y: auto; overscroll-behavior: contain;">
                <textarea
                  rows={9}
                  value={systemPrompt()}
                  onInput={(e) => setSystemPrompt(e.currentTarget.value)}
                  style="width: 100%; background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 8px; padding: 12px; font-size: 13px; line-height: 1.5; color: #18181b; outline: none; resize: vertical;"
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
                  取消 (Cancel)
                </button>
                <button
                  onClick={handleSaveAll}
                  style="padding: 6px 16px; background: #18181b; border: none; border-radius: 6px; font-size: 12px; font-weight: 500; color: #ffffff; cursor: pointer;"
                >
                  保存并生效 (Save Changes)
                </button>
              </div>
            </div>
          </div>

          {/* ══════════════════════════════════════════════════════════════════════
              添加 Provider 抽屉 / 模态卡片 (Add Provider Modal)
             ══════════════════════════════════════════════════════════════════════ */}
          <Show when={showAddProviderModal()}>
            <div
              onClick={() => setShowAddProviderModal(false)}
              style="position: absolute; inset: 0; background: rgba(0,0,0,0.35); backdrop-filter: blur(2px); z-index: 100; display: flex; align-items: center; justify-content: center; padding: 20px;"
            >
              <div
                onClick={(e) => e.stopPropagation()}
                style="width: 480px; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 12px; box-shadow: 0 16px 36px rgba(0,0,0,0.16); padding: 20px; display: flex; flex-direction: column; gap: 14px;"
              >
                <div style="display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid #f4f4f5; padding-bottom: 8px;">
                  <b style="font-size: 13.5px; color: #18181b;">添加新模型服务商 (Add Provider)</b>
                  <button
                    onClick={() => setShowAddProviderModal(false)}
                    style="background: transparent; border: none; color: #71717a; cursor: pointer;"
                  >
                    <IconClose size={14} />
                  </button>
                </div>

                <div style="display: flex; flex-direction: column; gap: 10px;">
                  <div style="display: flex; flex-direction: column; gap: 4px;">
                    <span style="font-size: 11.5px; color: #71717a;">Provider 名称</span>
                    <input
                      type="text"
                      placeholder="如: SiliconFlow / OpenRouter / Ollama"
                      value={newProviderName()}
                      onInput={(e) => setNewProviderName(e.currentTarget.value)}
                      style="background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px;"
                    />
                  </div>

                  <div style="display: flex; flex-direction: column; gap: 4px;">
                    <span style="font-size: 11.5px; color: #71717a;">API 请求地址 (Base URL)</span>
                    <input
                      type="text"
                      value={newProviderUrl()}
                      onInput={(e) => setNewProviderUrl(e.currentTarget.value)}
                      style="background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 6px; padding: 7px 10px; font-size: 12.5px; color: #18181b; outline: none;"
                    />
                  </div>

                  <div style="display: flex; flex-direction: column; gap: 4px;">
                    <span style="font-size: 11.5px; color: #71717a;">API Key</span>
                    <input
                      type="password"
                      placeholder="sk-..."
                      value={newProviderKey()}
                      onInput={(e) => setNewProviderKey(e.currentTarget.value)}
                      style="background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 6px; padding: 7px 10px; font-size: 12.5px; color: #18181b; outline: none;"
                    />
                  </div>

                  <div style="display: flex; flex-direction: column; gap: 4px;">
                    <span style="font-size: 11.5px; color: #71717a;">上游模式</span>
                    <select
                      value={newProviderProtocol()}
                      onChange={(e) => setNewProviderProtocol(e.currentTarget.value as any)}
                      style="background: #fafaf9; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px;"
                    >
                      <option value="responses">Responses (原生)</option>
                      <option value="chat_completions">Chat Completions (OpenAI 兼容)</option>
                      <option value="anthropic_messages">Anthropic Messages</option>
                    </select>
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
                    确认添加
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
