import { Component, createSignal, onCleanup, Show, For } from 'solid-js';
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
  IconCheck,
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

  // 新增模型表单状态
  const [showAddModelForm, setShowAddModelForm] = createSignal(false);
  const [newModelId, setNewModelId] = createSignal('');
  const [newModelName, setNewModelName] = createSignal('');
  const [newModelContext, setNewModelContext] = createSignal(128000);
  const [newModelMaxOutput, setNewModelMaxOutput] = createSignal(8192);
  const [newModelTools, setNewModelTools] = createSignal(true);
  const [newModelStream, setNewModelStream] = createSignal(true);
  const [newModelVision, setNewModelVision] = createSignal(false);
  const [newModelReasoning, setNewModelReasoning] = createSignal(false);

  // 新增 Provider 表单状态
  const [showAddProviderForm, setShowAddProviderForm] = createSignal(false);
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
    setShowAddModelForm(false);
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
      status: 'unconfigured',
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
    setShowAddProviderForm(false);
    setNewProviderName('');
    setNewProviderUrl('');
    setNewProviderKey('');
    toast.success(`已添加自定义 Provider: ${newProv.name}`);
  };

  const handleSaveAll = () => {
    setOpen(false);
    toast.success('配置已保存并实时生效！');
  };

  return (
    <Show when={open()}>
      <div
        onClick={() => setOpen(false)}
        style="position: fixed; inset: 0; background: rgba(0,0,0,0.45); backdrop-filter: blur(4px); z-index: 9999; display: flex; align-items: center; justify-content: center; padding: 24px; pointer-events: auto;"
      >
        <div
          onClick={(e) => e.stopPropagation()}
          style="width: 920px; max-width: 95vw; height: 620px; max-height: 90vh; background: #ffffff; border: 1px solid #e5e5e5; border-radius: 14px; box-shadow: 0 24px 48px -12px rgba(0,0,0,0.18); overflow: hidden; display: flex; flex-direction: row; animation: navis-pop 0.15s ease-out; pointer-events: auto;"
        >
          {/* 左侧主要导航栏 (Vertical Navigation) */}
          <div style="width: 220px; background: #f8f8f7; border-right: 1px solid #eaeaea; display: flex; flex-direction: column; justify-content: space-between; padding: 16px 10px;">
            <div style="display: flex; flex-direction: column; gap: 4px;">
              <div style="display: flex; align-items: center; gap: 8px; padding: 6px 10px 14px; border-bottom: 1px solid #ebeaea; margin-bottom: 6px;">
                <span style="color: #c2410c; display: flex; align-items: center;">
                  <IconSettings size={18} />
                </span>
                <span style="font-size: 14px; font-weight: 600; color: #18181b;">全局设置 (Settings)</span>
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
                <span>模型与网关 (Models)</span>
              </button>

              <button
                data-tab-id="sandbox"
                onClick={() => setActiveMainTab('sandbox')}
                style={`display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 8px; border: none; font-size: 13px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s; ${
                  activeMainTab() === 'sandbox' ? 'background: #eceae5; color: #18181b; font-weight: 600;' : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconShield size={16} />
                <span>沙箱与权限 (Sandbox)</span>
              </button>

              <button
                data-tab-id="prompt"
                onClick={() => setActiveMainTab('prompt')}
                style={`display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 8px; border: none; font-size: 13px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s; ${
                  activeMainTab() === 'prompt' ? 'background: #eceae5; color: #18181b; font-weight: 600;' : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconPrompt size={16} />
                <span>自定义指令 (Prompt)</span>
              </button>

              <button
                data-tab-id="general"
                onClick={() => setActiveMainTab('general')}
                style={`display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 8px; border: none; font-size: 13px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s; ${
                  activeMainTab() === 'general' ? 'background: #eceae5; color: #18181b; font-weight: 600;' : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconPalette size={16} />
                <span>常规与外观 (General)</span>
              </button>
            </div>

            {/* 左下角关闭/退出按钮 */}
            <div style="border-top: 1px solid #ebeaea; padding-top: 10px;">
              <button
                onClick={() => setOpen(false)}
                style="display: flex; align-items: center; justify-content: center; gap: 6px; width: 100%; padding: 7px; background: transparent; border: 1px solid #e5e5e5; border-radius: 6px; font-size: 12px; color: #71717a; cursor: pointer;"
              >
                <IconClose size={14} />
                <span>关闭设置</span>
              </button>
            </div>
          </div>

          {/* 右侧主工作区 */}
          <div style="flex: 1; display: flex; flex-direction: column; background: #ffffff; overflow: hidden;">
            {/* 1. 模型与网关面板 */}
            <Show when={activeMainTab() === 'models'}>
              <div style="flex: 1; display: flex; flex-direction: column; overflow-y: auto; padding: 20px; gap: 16px; overscroll-behavior: contain;">
                {/* 顶部水平 Provider 标签选择器 */}
                <div style="display: flex; flex-direction: column; gap: 8px;">
                  <div style="display: flex; align-items: center; justify-content: space-between;">
                    <span style="font-size: 11.5px; font-weight: 600; color: #a1a1aa; letter-spacing: 0.5px;">
                      AI PROVIDERS (模型服务商)
                    </span>
                    <button
                      onClick={() => setShowAddProviderForm(!showAddProviderForm())}
                      style="background: transparent; border: none; font-size: 12px; color: #ea580c; cursor: pointer; display: flex; align-items: center; gap: 4px; font-weight: 500;"
                    >
                      <IconPlus size={13} />
                      <span>添加 Provider</span>
                    </button>
                  </div>

                  {/* Provider 胶囊列表 */}
                  <div style="display: flex; gap: 8px; flex-wrap: wrap;">
                    <For each={gatewayStore.providers()}>
                      {(p) => (
                        <div
                          data-provider-id={p.id}
                          onClick={() => setSelectedProviderId(p.id)}
                          style={`padding: 6px 12px; border-radius: 8px; cursor: pointer; display: flex; align-items: center; gap: 8px; font-size: 12.5px; transition: all 0.12s; border: 1px solid; ${
                            selectedProviderId() === p.id
                              ? 'background: #f4f3ef; border-color: #d4d0c7; color: #18181b; font-weight: 600;'
                              : 'background: #fafafa; border-color: #eaeaea; color: #71717a;'
                          }`}
                        >
                          <span
                            style={`width: 7px; height: 7px; border-radius: 50%; ${
                              p.status === 'connected' ? 'background: #16a34a;' : p.status === 'checking' ? 'background: #eab308;' : 'background: #d4d4d8;'
                            }`}
                          />
                          <span>{p.name.split(' (')[0]}</span>
                          <Show when={gatewayStore.activeProviderId() === p.id}>
                            <span style="font-size: 10px; background: #dcfce7; color: #15803d; padding: 1px 5px; border-radius: 4px; font-weight: 600;">
                              活跃
                            </span>
                          </Show>
                        </div>
                      )}
                    </For>
                  </div>
                </div>

                {/* 添加自定义 Provider 折叠表单 */}
                <Show when={showAddProviderForm()}>
                  <div style="background: #fafaf9; border: 1px dashed #d4d4d8; border-radius: 10px; padding: 14px; display: flex; flex-direction: column; gap: 10px;">
                    <div style="font-size: 12px; font-weight: 600; color: #27272a;">配置新模型提供商 (Custom Provider):</div>
                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px;">
                      <input
                        type="text"
                        placeholder="Provider 名称 (如: SiliconFlow / OpenRouter)"
                        value={newProviderName()}
                        onInput={(e) => setNewProviderName(e.currentTarget.value)}
                        style="background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px;"
                      />
                      <input
                        type="text"
                        placeholder="Base URL (如: https://api.siliconflow.cn/v1)"
                        value={newProviderUrl()}
                        onInput={(e) => setNewProviderUrl(e.currentTarget.value)}
                        style="background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px;"
                      />
                    </div>
                    <input
                      type="password"
                      placeholder="API Key (sk-...)"
                      value={newProviderKey()}
                      onInput={(e) => setNewProviderKey(e.currentTarget.value)}
                      style="background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 10px; font-size: 12px;"
                    />
                    <div style="display: flex; justify-content: flex-end; gap: 8px;">
                      <button
                        onClick={() => setShowAddProviderForm(false)}
                        style="padding: 5px 12px; background: transparent; border: 1px solid #e4e4e7; border-radius: 5px; font-size: 12px; cursor: pointer;"
                      >
                        取消
                      </button>
                      <button
                        onClick={handleSaveCustomProvider}
                        style="padding: 5px 14px; background: #18181b; border: none; border-radius: 5px; color: #ffffff; font-size: 12px; font-weight: 500; cursor: pointer;"
                      >
                        确认添加
                      </button>
                    </div>
                  </div>
                </Show>

                {/* 选中 Provider 的详细参数卡片 */}
                <div style="background: #fafaf9; border: 1px solid #eaeaea; border-radius: 12px; padding: 16px; display: flex; flex-direction: column; gap: 14px;">
                  {/* 卡片头部与操作工具条 */}
                  <div style="display: flex; align-items: center; justify-content: space-between;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <b style="font-size: 15px; color: #18181b;">{currentProvider().name}</b>
                      <span style="font-size: 11px; background: #eaeaea; color: #52525b; padding: 2px 6px; border-radius: 4px; font-weight: 600;">
                        {currentProvider().type.toUpperCase()}
                      </span>
                    </div>

                    <div style="display: flex; align-items: center; gap: 8px;">
                      <button
                        onClick={handleTestCurrentProvider}
                        disabled={isTesting()}
                        style="padding: 5px 10px; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; font-size: 12px; font-weight: 500; color: #27272a; cursor: pointer; display: flex; align-items: center; gap: 5px;"
                      >
                        <IconZap size={13} color="#ea580c" />
                        <span>{isTesting() ? '测试中...' : '测试连接 (Ping)'}</span>
                      </button>

                      <button
                        onClick={handleSyncModels}
                        disabled={isSyncing()}
                        style="padding: 5px 10px; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; font-size: 12px; font-weight: 500; color: #27272a; cursor: pointer; display: flex; align-items: center; gap: 5px;"
                      >
                        <IconRefresh size={13} color="#3b82f6" />
                        <span>{isSyncing() ? '同步中...' : '同步模型'}</span>
                      </button>

                      <button
                        onClick={() => {
                          gatewayStore.setActiveProvider(currentProvider().id);
                          toast.success(`已将 ${currentProvider().name} 设为当前激活 Provider`);
                        }}
                        style={`padding: 5px 12px; border-radius: 6px; font-size: 12px; font-weight: 500; cursor: pointer; display: flex; align-items: center; gap: 5px; ${
                          gatewayStore.activeProviderId() === currentProvider().id
                            ? 'background: #16a34a; border: 1px solid #16a34a; color: #ffffff;'
                            : 'background: #18181b; border: none; color: #ffffff;'
                        }`}
                      >
                        <Show when={gatewayStore.activeProviderId() === currentProvider().id} fallback={<span>⭐ 设为激活</span>}>
                          <IconCheck size={13} />
                          <span>正在使用</span>
                        </Show>
                      </button>
                    </div>
                  </div>

                  {/* 端点与密钥输入 */}
                  <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
                    <div style="display: flex; flex-direction: column; gap: 5px;">
                      <label style="font-size: 11.5px; font-weight: 600; color: #52525b;">Base URL (端点服务地址):</label>
                      <input
                        type="text"
                        value={currentProvider().baseUrl}
                        onInput={(e) => gatewayStore.updateProvider(currentProvider().id, { baseUrl: e.currentTarget.value })}
                        style="background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 7px 10px; color: #18181b; font-size: 12px;"
                      />
                    </div>

                    <div style="display: flex; flex-direction: column; gap: 5px;">
                      <div style="display: flex; justify-content: space-between; align-items: center;">
                        <label style="font-size: 11.5px; font-weight: 600; color: #52525b;">API Key (安全引用):</label>
                        <button
                          onClick={() => setShowApiKey(!showApiKey())}
                          style="background: transparent; border: none; font-size: 11px; color: #71717a; cursor: pointer; display: flex; align-items: center; gap: 4px;"
                        >
                          <Show when={showApiKey()} fallback={<><IconEye size={12} /><span>显示</span></>}>
                            <IconEyeOff size={12} /><span>隐藏</span>
                          </Show>
                        </button>
                      </div>
                      <input
                        type={showApiKey() ? 'text' : 'password'}
                        value={currentProvider().apiKey}
                        onInput={(e) => gatewayStore.updateProvider(currentProvider().id, { apiKey: e.currentTarget.value })}
                        placeholder="sk-..."
                        style="background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 7px 10px; color: #18181b; font-size: 12px;"
                      />
                    </div>
                  </div>
                </div>

                {/* 可用模型列表 */}
                <div style="display: flex; flex-direction: column; gap: 10px;">
                  <div style="display: flex; justify-content: space-between; align-items: center;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <span style="font-size: 13px; font-weight: 600; color: #18181b;">
                        可用模型能力列表 ({currentProvider().models.length})
                      </span>
                      <span style="font-size: 11px; color: #a1a1aa;">装载到此 Provider 下的模型能力目录</span>
                    </div>
                    <button
                      onClick={() => setShowAddModelForm(!showAddModelForm())}
                      style="padding: 4px 10px; background: #f4f4f5; border: 1px solid #e4e4e7; border-radius: 6px; font-size: 11.5px; font-weight: 500; color: #27272a; cursor: pointer; display: flex; align-items: center; gap: 4px;"
                    >
                      <IconPlus size={12} />
                      <span>{showAddModelForm() ? '收起表单' : '添加模型'}</span>
                    </button>
                  </div>

                  {/* 添加模型折叠表单 */}
                  <Show when={showAddModelForm()}>
                    <div style="background: #fafaf9; border: 1px dashed #d4d4d8; border-radius: 10px; padding: 14px; display: flex; flex-direction: column; gap: 10px;">
                      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px;">
                        <input
                          type="text"
                          placeholder="Model ID (如: claude-3-7-sonnet-20250219)"
                          value={newModelId()}
                          onInput={(e) => setNewModelId(e.currentTarget.value)}
                          style="background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 8px; font-size: 12px;"
                        />
                        <input
                          type="text"
                          placeholder="显示名称 (如: Claude 3.7 Sonnet)"
                          value={newModelName()}
                          onInput={(e) => setNewModelName(e.currentTarget.value)}
                          style="background: #ffffff; border: 1px solid #e4e4e7; border-radius: 6px; padding: 6px 8px; font-size: 12px;"
                        />
                      </div>

                      {/* 能力多选 */}
                      <div style="display: flex; align-items: center; gap: 16px; font-size: 11.5px; color: #52525b;">
                        <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
                          <input type="checkbox" checked={newModelTools()} onChange={(e) => setNewModelTools(e.currentTarget.checked)} />
                          <span>Tool Calling (工具)</span>
                        </label>
                        <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
                          <input type="checkbox" checked={newModelStream()} onChange={(e) => setNewModelStream(e.currentTarget.checked)} />
                          <span>Streaming (流式)</span>
                        </label>
                        <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
                          <input type="checkbox" checked={newModelVision()} onChange={(e) => setNewModelVision(e.currentTarget.checked)} />
                          <span>Vision (多模态)</span>
                        </label>
                        <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
                          <input type="checkbox" checked={newModelReasoning()} onChange={(e) => setNewModelReasoning(e.currentTarget.checked)} />
                          <span>Reasoning (深度思考)</span>
                        </label>
                      </div>

                      <div style="display: flex; justify-content: flex-end; gap: 8px;">
                        <button
                          onClick={() => setShowAddModelForm(false)}
                          style="padding: 4px 10px; background: transparent; border: 1px solid #e4e4e7; border-radius: 5px; font-size: 11.5px; cursor: pointer;"
                        >
                          取消
                        </button>
                        <button
                          onClick={handleSaveModel}
                          style="padding: 4px 12px; background: #18181b; border: none; border-radius: 5px; color: #ffffff; font-size: 11.5px; font-weight: 500; cursor: pointer;"
                        >
                          确认添加
                        </button>
                      </div>
                    </div>
                  </Show>

                  {/* 模型卡片列表 */}
                  <div style="display: flex; flex-direction: column; gap: 6px;">
                    <For each={currentProvider().models}>
                      {(model) => (
                        <div
                          style={`padding: 10px 14px; border-radius: 8px; border: 1px solid; display: flex; align-items: center; justify-content: space-between; transition: all 0.1s; ${
                            gatewayStore.activeModelId() === model.id ? 'background: #fafaf8; border-color: #d4d0c7;' : 'background: #ffffff; border-color: #ebeaea;'
                          }`}
                        >
                          <div style="display: flex; flex-direction: column; gap: 3px;">
                            <div style="display: flex; align-items: center; gap: 8px;">
                              <span style="font-size: 13px; font-weight: 600; color: #18181b;">{model.name}</span>
                              <span style="font-size: 11px; color: #71717a; background: #f4f4f5; padding: 1px 5px; border-radius: 4px; font-family: monospace;">
                                {model.id}
                              </span>
                            </div>
                            <div style="display: flex; align-items: center; gap: 5px;">
                              <span style="font-size: 10.5px; color: #71717a; background: #f8f8f7; border: 1px solid #e5e5e5; padding: 0 5px; border-radius: 4px;">
                                {Math.round(model.contextWindow / 1000)}k 上下文
                              </span>
                              <Show when={model.capabilities.tools}>
                                <span style="font-size: 10px; color: #1d4ed8; background: #eff6ff; border: 1px solid #dbeafe; padding: 0 5px; border-radius: 4px; font-weight: 500;">
                                  Tools
                                </span>
                              </Show>
                              <Show when={model.capabilities.streaming}>
                                <span style="font-size: 10px; color: #15803d; background: #f0fdf4; border: 1px solid #dcfce7; padding: 0 5px; border-radius: 4px; font-weight: 500;">
                                  Stream
                                </span>
                              </Show>
                              <Show when={model.capabilities.reasoning}>
                                <span style="font-size: 10px; color: #ea580c; background: #fff7ed; border: 1px solid #ffedd5; padding: 0 5px; border-radius: 4px; font-weight: 500;">
                                  Reasoning
                                </span>
                              </Show>
                              <Show when={model.capabilities.vision}>
                                <span style="font-size: 10px; color: #7e22ce; background: #faf5ff; border: 1px solid #f3e8ff; padding: 0 5px; border-radius: 4px; font-weight: 500;">
                                  Vision
                                </span>
                              </Show>
                            </div>
                          </div>

                          <div style="display: flex; align-items: center; gap: 6px;">
                            <button
                              onClick={() => {
                                gatewayStore.setActiveModel(model.id);
                                toast.success(`已设置默认模型: ${model.name}`);
                              }}
                              style={`padding: 4px 10px; border-radius: 5px; font-size: 11.5px; font-weight: 500; cursor: pointer; display: flex; align-items: center; gap: 4px; ${
                                gatewayStore.activeModelId() === model.id
                                  ? 'background: #dcfce7; border: 1px solid #bbf7d0; color: #166534;'
                                  : 'background: #f4f4f5; border: 1px solid #e4e4e7; color: #27272a;'
                              }`}
                            >
                              <Show when={gatewayStore.activeModelId() === model.id} fallback={<span>设为默认</span>}>
                                <IconCheck size={12} />
                                <span>默认模型</span>
                              </Show>
                            </button>
                            <Show when={currentProvider().models.length > 1}>
                              <button
                                onClick={() => {
                                  gatewayStore.deleteModel(currentProvider().id, model.id);
                                  toast.info(`已移除模型 ${model.name}`);
                                }}
                                style="background: transparent; border: none; color: #a1a1aa; cursor: pointer; padding: 4px;"
                                title="删除模型"
                              >
                                <IconTrash size={14} />
                              </button>
                            </Show>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              </div>
            </Show>

            {/* 2. 沙箱权限面板 */}
            <Show when={activeMainTab() === 'sandbox'}>
              <div style="flex: 1; padding: 24px; display: flex; flex-direction: column; gap: 16px; overflow-y: auto; overscroll-behavior: contain;">
                <div style="display: flex; flex-direction: column; gap: 4px;">
                  <h3 style="font-size: 15px; font-weight: 600; color: #18181b; margin: 0;">
                    Agent 执行策略与权限沙箱 (Execution Policy)
                  </h3>
                  <span style="font-size: 12px; color: #71717a;">
                    控制 Agent 在编写代码、修改工作区与执行终端命令时的审批门禁级别。
                  </span>
                </div>

                <div style="display: flex; flex-direction: column; gap: 10px;">
                  <div
                    onClick={() => {
                      setSandboxLevel('bypass');
                      toast.info('已开启 Bypass 自动放行模式');
                    }}
                    style={`padding: 14px; border-radius: 10px; border: 1px solid; cursor: pointer; display: flex; flex-direction: column; gap: 4px; ${
                      sandboxLevel() === 'bypass' ? 'background: #fafaf8; border-color: #ea580c;' : 'background: #ffffff; border-color: #eaeaea;'
                    }`}
                  >
                    <div style="display: flex; align-items: center; justify-content: space-between;">
                      <b style="font-size: 13.5px; color: #18181b;">Bypass permissions (自动放行执行 - 推荐)</b>
                      <Show when={sandboxLevel() === 'bypass'}>
                        <span style="color: #ea580c; font-size: 11.5px; font-weight: 600;">● 当前生效</span>
                      </Show>
                    </div>
                    <span style="font-size: 12px; color: #71717a;">
                      Agent 自动执行文件读写、创建新文件与常规 Shell 命令，无需手动弹窗确认，适合快速迭代开发。
                    </span>
                  </div>

                  <div
                    onClick={() => {
                      setSandboxLevel('confirm');
                      toast.info('已开启 Ask for confirmation 审批模式');
                    }}
                    style={`padding: 14px; border-radius: 10px; border: 1px solid; cursor: pointer; display: flex; flex-direction: column; gap: 4px; ${
                      sandboxLevel() === 'confirm' ? 'background: #fafaf8; border-color: #ea580c;' : 'background: #ffffff; border-color: #eaeaea;'
                    }`}
                  >
                    <div style="display: flex; align-items: center; justify-content: space-between;">
                      <b style="font-size: 13.5px; color: #18181b;">Ask for confirmation (人工逐项审批)</b>
                      <Show when={sandboxLevel() === 'confirm'}>
                        <span style="color: #ea580c; font-size: 11.5px; font-weight: 600;">● 当前生效</span>
                      </Show>
                    </div>
                    <span style="font-size: 12px; color: #71717a;">
                      在执行破坏性文件修改、删除文件或高风险 Shell 脚本前，弹出确认对话框由您手工批准。
                    </span>
                  </div>

                  <div
                    onClick={() => {
                      setSandboxLevel('strict');
                      toast.info('已开启 Read-only 严格只读模式');
                    }}
                    style={`padding: 14px; border-radius: 10px; border: 1px solid; cursor: pointer; display: flex; flex-direction: column; gap: 4px; ${
                      sandboxLevel() === 'strict' ? 'background: #fafaf8; border-color: #ea580c;' : 'background: #ffffff; border-color: #eaeaea;'
                    }`}
                  >
                    <div style="display: flex; align-items: center; justify-content: space-between;">
                      <b style="font-size: 13.5px; color: #18181b;">Strict Read-Only Sandbox (严格只读)</b>
                      <Show when={sandboxLevel() === 'strict'}>
                        <span style="color: #ea580c; font-size: 11.5px; font-weight: 600;">● 当前生效</span>
                      </Show>
                    </div>
                    <span style="font-size: 12px; color: #71717a;">
                      Agent 仅允许读取代码并回答问题，禁止向磁盘写入任何变更或启动终端子进程。
                    </span>
                  </div>
                </div>
              </div>
            </Show>

            {/* 3. 自定义指令面板 */}
            <Show when={activeMainTab() === 'prompt'}>
              <div style="flex: 1; padding: 24px; display: flex; flex-direction: column; gap: 16px; overflow-y: auto; overscroll-behavior: contain;">
                <div style="display: flex; flex-direction: column; gap: 4px;">
                  <h3 style="font-size: 15px; font-weight: 600; color: #18181b; margin: 0;">
                    系统提示词与自定义指令 (Custom Instructions)
                  </h3>
                  <span style="font-size: 12px; color: #71717a;">
                    注入到每一轮 Agent 对话中的顶级指导原则，可定义架构偏好、编码规范与禁忌。
                  </span>
                </div>

                <textarea
                  rows={9}
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

            {/* 4. 通用与外观面板 */}
            <Show when={activeMainTab() === 'general'}>
              <div style="flex: 1; padding: 24px; display: flex; flex-direction: column; gap: 16px; overflow-y: auto; overscroll-behavior: contain;">
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

            {/* 底部确认栏 */}
            <div style="height: 52px; background: #fbfaf8; border-top: 1px solid #eaeaea; display: flex; align-items: center; justify-content: space-between; padding: 0 20px;">
              <div style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #71717a;">
                <span>当前生效模型:</span>
                <b style="color: #18181b;">{gatewayStore.activeProvider().name.split(' (')[0]} · {gatewayStore.activeModel().name}</b>
              </div>

              <div style="display: flex; align-items: center; gap: 8px;">
                <button
                  onClick={() => setOpen(false)}
                  style="padding: 6px 14px; background: transparent; border: 1px solid #e4e4e7; border-radius: 6px; font-size: 12.5px; font-weight: 500; color: #52525b; cursor: pointer;"
                >
                  取消
                </button>
                <button
                  onClick={handleSaveAll}
                  style="padding: 6px 16px; background: #18181b; border: none; border-radius: 6px; font-size: 12.5px; font-weight: 500; color: #ffffff; cursor: pointer;"
                >
                  保存并生效
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Show>
  );
};

export default SettingsModal;
