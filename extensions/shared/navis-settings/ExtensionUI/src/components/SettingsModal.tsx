import { Component, createSignal, onCleanup, Show, For } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import { gatewayStore, type ProviderItem, type ModelItem } from '@extensions/shared/navis-ai-platform/ExtensionUI/src/store/GatewayStore';

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
    toast.success('配置已更新并实时生效！');
  };

  return (
    <Show when={open()}>
      <div
        onClick={() => setOpen(false)}
        style="position: fixed; inset: 0; background: rgba(0,0,0,0.38); backdrop-filter: blur(3px); z-index: 9999; display: flex; align-items: center; justify-content: center;"
      >
        <div
          onClick={(e) => e.stopPropagation()}
          style="width: 860px; max-width: 94vw; height: 600px; max-height: 90vh; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 14px; box-shadow: 0 20px 48px rgba(0,0,0,0.18); overflow: hidden; display: flex; flex-direction: column; animation: navis-pop 0.15s ease-out;"
        >
          {/* 顶栏 */}
          <div style="height: 48px; border-bottom: 1px solid #eae7e1; display: flex; align-items: center; justify-content: space-between; padding: 0 20px; background: #faf9f6;">
            <div style="display: flex; align-items: center; gap: 8px;">
              <span style="font-size: 16px; color: #c2410c;">⚙️</span>
              <h2 style="font-size: 15px; font-weight: 600; color: #2d2b28; margin: 0;">
                设置与模型网关 (Settings & Model Hub)
              </h2>
            </div>
            <button
              onClick={() => setOpen(false)}
              style="background: transparent; border: none; font-size: 16px; color: #8e8b83; cursor: pointer; padding: 4px 6px; border-radius: 4px;"
              title="关闭"
            >
              ✕
            </button>
          </div>

          {/* 主功能 Tab 栏 */}
          <div style="display: flex; gap: 6px; padding: 8px 20px; background: #fbfaf8; border-bottom: 1px solid #eae7e1;">
            <button
              data-tab-id="models"
              onClick={() => setActiveMainTab('models')}
              style={`padding: 6px 14px; border-radius: 6px; border: none; font-size: 13px; font-weight: 500; cursor: pointer; display: flex; align-items: center; gap: 6px; ${
                activeMainTab() === 'models' ? 'background: #eceae4; color: #1e1d1b; font-weight: 600;' : 'background: transparent; color: #76736c;'
              }`}
            >
              <span>🤖</span>
              <span>模型与网关 (Models & Gateway)</span>
            </button>
            <button
              data-tab-id="sandbox"
              onClick={() => setActiveMainTab('sandbox')}
              style={`padding: 6px 14px; border-radius: 6px; border: none; font-size: 13px; font-weight: 500; cursor: pointer; display: flex; align-items: center; gap: 6px; ${
                activeMainTab() === 'sandbox' ? 'background: #eceae4; color: #1e1d1b; font-weight: 600;' : 'background: transparent; color: #76736c;'
              }`}
            >
              <span>🛡️</span>
              <span>沙箱与权限 (Sandbox)</span>
            </button>
            <button
              data-tab-id="prompt"
              onClick={() => setActiveMainTab('prompt')}
              style={`padding: 6px 14px; border-radius: 6px; border: none; font-size: 13px; font-weight: 500; cursor: pointer; display: flex; align-items: center; gap: 6px; ${
                activeMainTab() === 'prompt' ? 'background: #eceae4; color: #1e1d1b; font-weight: 600;' : 'background: transparent; color: #76736c;'
              }`}
            >
              <span>📝</span>
              <span>自定义指令 (System Prompt)</span>
            </button>
            <button
              data-tab-id="general"
              onClick={() => setActiveMainTab('general')}
              style={`padding: 6px 14px; border-radius: 6px; border: none; font-size: 13px; font-weight: 500; cursor: pointer; display: flex; align-items: center; gap: 6px; ${
                activeMainTab() === 'general' ? 'background: #eceae4; color: #1e1d1b; font-weight: 600;' : 'background: transparent; color: #76736c;'
              }`}
            >
              <span>🎨</span>
              <span>外观与常规 (General)</span>
            </button>
          </div>

          {/* 主内容区域 */}
          <div style="flex: 1; display: flex; overflow: hidden;">
            {/* 1. 模型与网关主面板 */}
            <Show when={activeMainTab() === 'models'}>
              {/* 左侧：Provider 提供商列表 */}
              <div style="width: 260px; border-right: 1px solid #eae7e1; background: #fbfaf8; display: flex; flex-direction: column; padding: 12px 10px; gap: 8px;">
                <div style="font-size: 11.5px; font-weight: 600; color: #8e8b83; padding: 0 6px; display: flex; justify-content: space-between; align-items: center;">
                  <span>PROVIDERS (服务商)</span>
                  <button
                    onClick={() => setShowAddProviderForm(true)}
                    style="background: transparent; border: none; font-size: 13px; color: #c2410c; cursor: pointer; padding: 0 4px;"
                    title="添加自定义 Provider"
                  >
                    + 添加
                  </button>
                </div>

                <div style="flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 4px;">
                  <For each={gatewayStore.providers()}>
                    {(p) => (
                      <div
                        data-provider-id={p.id}
                        onClick={() => setSelectedProviderId(p.id)}
                        style={`padding: 9px 10px; border-radius: 8px; cursor: pointer; display: flex; flex-direction: column; gap: 4px; transition: all 0.1s; ${
                          selectedProviderId() === p.id
                            ? 'background: #eceae4; border: 1px solid #e0ddd4;'
                            : 'background: transparent; border: 1px solid transparent;'
                        }`}
                      >
                        <div style="display: flex; align-items: center; justify-content: space-between;">
                          <span style="font-size: 13px; font-weight: 600; color: #1e1d1b; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                            {p.name}
                          </span>
                          <Show when={gatewayStore.activeProviderId() === p.id}>
                            <span style="font-size: 10px; background: #dcfce7; color: #166534; padding: 1px 5px; border-radius: 4px; font-weight: 600;">
                              活跃
                            </span>
                          </Show>
                        </div>
                        <div style="display: flex; align-items: center; justify-content: space-between; font-size: 11px; color: #76736c;">
                          <div style="display: flex; align-items: center; gap: 4px;">
                            <span
                              style={`width: 6px; height: 6px; border-radius: 50%; ${
                                p.status === 'connected'
                                  ? 'background: #16a34a;'
                                  : p.status === 'checking'
                                  ? 'background: #d97706;'
                                  : 'background: #9ca3af;'
                              }`}
                            />
                            <span>{p.status === 'connected' ? `已连接 (${p.pingMs || 16}ms)` : p.status === 'checking' ? '测试中...' : '未测试'}</span>
                          </div>
                          <span>{p.models.length} 个模型</span>
                        </div>
                      </div>
                    )}
                  </For>
                </div>
              </div>

              {/* 右侧：当前 Provider 详细设置与模型卡片 */}
              <div style="flex: 1; display: flex; flex-direction: column; overflow-y: auto; padding: 20px; gap: 18px; background: #ffffff;">
                {/* 选中的 Provider 头部 */}
                <div style="display: flex; align-items: center; justify-content: space-between; padding-bottom: 12px; border-bottom: 1px solid #eae7e1;">
                  <div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <h3 style="font-size: 16px; font-weight: 600; color: #1e1d1b; margin: 0;">
                        {currentProvider().name}
                      </h3>
                      <span style="font-size: 11px; background: #f0eee8; padding: 2px 6px; border-radius: 4px; color: #5a5750;">
                        {currentProvider().type.toUpperCase()}
                      </span>
                    </div>
                    <span style="font-size: 12px; color: #8e8b83;">管理连接端点、API 密钥与模型能力</span>
                  </div>

                  <div style="display: flex; align-items: center; gap: 8px;">
                    <button
                      onClick={handleTestCurrentProvider}
                      disabled={isTesting()}
                      style="padding: 6px 12px; background: #f7f6f2; border: 1px solid #e7e4dc; border-radius: 6px; font-size: 12.5px; font-weight: 500; color: #2d2b28; cursor: pointer; display: flex; align-items: center; gap: 6px;"
                    >
                      <span>⚡</span>
                      <span>{isTesting() ? '测试中...' : '测试连接 (Ping)'}</span>
                    </button>
                    <button
                      onClick={handleSyncModels}
                      disabled={isSyncing()}
                      style="padding: 6px 12px; background: #f7f6f2; border: 1px solid #e7e4dc; border-radius: 6px; font-size: 12.5px; font-weight: 500; color: #2d2b28; cursor: pointer; display: flex; align-items: center; gap: 6px;"
                    >
                      <span>🔄</span>
                      <span>{isSyncing() ? '同步中...' : '同步模型'}</span>
                    </button>
                    <button
                      onClick={() => {
                        gatewayStore.setActiveProvider(currentProvider().id);
                        toast.success(`已将 ${currentProvider().name} 设为当前激活 Provider`);
                      }}
                      style={`padding: 6px 14px; border-radius: 6px; font-size: 12.5px; font-weight: 500; cursor: pointer; ${
                        gatewayStore.activeProviderId() === currentProvider().id
                          ? 'background: #16a34a; border: 1px solid #16a34a; color: #ffffff;'
                          : 'background: #2d2b28; border: none; color: #ffffff;'
                      }`}
                    >
                      {gatewayStore.activeProviderId() === currentProvider().id ? '✓ 正在使用' : '⭐ 设为当前激活'}
                    </button>
                  </div>
                </div>

                {/* 端点与密钥配置 */}
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 14px; background: #faf9f6; padding: 14px; border-radius: 10px; border: 1px solid #eae7e1;">
                  <div style="display: flex; flex-direction: column; gap: 6px;">
                    <label style="font-size: 12px; font-weight: 600; color: #4b4843;">Base URL (端点服务地址):</label>
                    <input
                      type="text"
                      value={currentProvider().baseUrl}
                      onInput={(e) => gatewayStore.updateProvider(currentProvider().id, { baseUrl: e.currentTarget.value })}
                      style="background: #ffffff; border: 1px solid #e7e4dc; border-radius: 6px; padding: 7px 10px; color: #2d2b28; font-size: 12.5px;"
                    />
                  </div>
                  <div style="display: flex; flex-direction: column; gap: 6px;">
                    <div style="display: flex; justify-content: space-between; align-items: center;">
                      <label style="font-size: 12px; font-weight: 600; color: #4b4843;">API Key (安全加密引用):</label>
                      <button
                        onClick={() => setShowApiKey(!showApiKey())}
                        style="background: transparent; border: none; font-size: 11px; color: #76736c; cursor: pointer;"
                      >
                        {showApiKey() ? '🙈 隐藏' : '👁️ 显示'}
                      </button>
                    </div>
                    <input
                      type={showApiKey() ? 'text' : 'password'}
                      value={currentProvider().apiKey}
                      onInput={(e) => gatewayStore.updateProvider(currentProvider().id, { apiKey: e.currentTarget.value })}
                      placeholder="sk-..."
                      style="background: #ffffff; border: 1px solid #e7e4dc; border-radius: 6px; padding: 7px 10px; color: #2d2b28; font-size: 12.5px;"
                    />
                  </div>
                </div>

                {/* 可用模型列表卡片 */}
                <div style="display: flex; flex-direction: column; gap: 10px;">
                  <div style="display: flex; justify-content: space-between; align-items: center;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <span style="font-size: 13px; font-weight: 600; color: #2d2b28;">
                        可用模型列表 ({currentProvider().models.length})
                      </span>
                      <span style="font-size: 11px; color: #8e8b83;">已装载到当前 Provider 的模型能力集</span>
                    </div>
                    <button
                      onClick={() => setShowAddModelForm(!showAddModelForm())}
                      style="padding: 4px 10px; background: #f0eee8; border: 1px solid #e2ded5; border-radius: 6px; font-size: 12px; font-weight: 500; color: #2d2b28; cursor: pointer;"
                    >
                      {showAddModelForm() ? '收起添加' : '+ 添加模型'}
                    </button>
                  </div>

                  {/* 添加模型折叠表单 */}
                  <Show when={showAddModelForm()}>
                    <div style="background: #fbfaf8; border: 1px dashed #d6d3ca; border-radius: 10px; padding: 14px; display: flex; flex-direction: column; gap: 12px;">
                      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px;">
                        <div style="display: flex; flex-direction: column; gap: 4px;">
                          <label style="font-size: 11.5px; font-weight: 500; color: #5a5750;">Model ID (模型标识):</label>
                          <input
                            type="text"
                            placeholder="如: claude-3-7-sonnet-20250219"
                            value={newModelId()}
                            onInput={(e) => setNewModelId(e.currentTarget.value)}
                            style="background: #ffffff; border: 1px solid #e7e4dc; border-radius: 6px; padding: 6px 8px; font-size: 12px;"
                          />
                        </div>
                        <div style="display: flex; flex-direction: column; gap: 4px;">
                          <label style="font-size: 11.5px; font-weight: 500; color: #5a5750;">显示名称 (Display Name):</label>
                          <input
                            type="text"
                            placeholder="如: Claude 3.7 Sonnet"
                            value={newModelName()}
                            onInput={(e) => setNewModelName(e.currentTarget.value)}
                            style="background: #ffffff; border: 1px solid #e7e4dc; border-radius: 6px; padding: 6px 8px; font-size: 12px;"
                          />
                        </div>
                      </div>

                      {/* 能力复选框 */}
                      <div style="display: flex; align-items: center; gap: 16px; font-size: 12px; color: #4b4843;">
                        <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
                          <input type="checkbox" checked={newModelTools()} onChange={(e) => setNewModelTools(e.currentTarget.checked)} />
                          <span>Tool Calling (工具调用)</span>
                        </label>
                        <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
                          <input type="checkbox" checked={newModelStream()} onChange={(e) => setNewModelStream(e.currentTarget.checked)} />
                          <span>Streaming (流式响应)</span>
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
                          style="padding: 5px 12px; background: transparent; border: 1px solid #e7e4dc; border-radius: 5px; font-size: 12px; cursor: pointer;"
                        >
                          取消
                        </button>
                        <button
                          onClick={handleSaveModel}
                          style="padding: 5px 14px; background: #2d2b28; border: none; border-radius: 5px; color: #ffffff; font-size: 12px; font-weight: 500; cursor: pointer;"
                        >
                          确认添加
                        </button>
                      </div>
                    </div>
                  </Show>

                  {/* 模型卡片列表 */}
                  <div style="display: flex; flex-direction: column; gap: 8px;">
                    <For each={currentProvider().models}>
                      {(model) => (
                        <div
                          style={`padding: 12px 14px; border-radius: 10px; border: 1px solid #eae7e1; display: flex; align-items: center; justify-content: space-between; transition: all 0.1s; ${
                            gatewayStore.activeModelId() === model.id ? 'background: #faf8f5; border-color: #e0d9cf;' : 'background: #ffffff;'
                          }`}
                        >
                          <div style="display: flex; flex-direction: column; gap: 4px;">
                            <div style="display: flex; align-items: center; gap: 8px;">
                              <span style="font-size: 13.5px; font-weight: 600; color: #1e1d1b;">
                                {model.name}
                              </span>
                              <span style="font-size: 11px; color: #8e8b83; background: #f0eee8; padding: 1px 5px; border-radius: 4px; font-family: monospace;">
                                {model.id}
                              </span>
                            </div>
                            {/* 能力标签 */}
                            <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap;">
                              <span style="font-size: 11px; color: #76736c; background: #f7f6f2; border: 1px solid #eae7e1; padding: 1px 6px; border-radius: 4px;">
                                {Math.round(model.contextWindow / 1000)}k 上下文
                              </span>
                              <Show when={model.capabilities.tools}>
                                <span style="font-size: 10.5px; color: #1d4ed8; background: #eff6ff; border: 1px solid #dbeafe; padding: 1px 6px; border-radius: 4px; font-weight: 500;">
                                  Tools
                                </span>
                              </Show>
                              <Show when={model.capabilities.streaming}>
                                <span style="font-size: 10.5px; color: #15803d; background: #f0fdf4; border: 1px solid #dcfce7; padding: 1px 6px; border-radius: 4px; font-weight: 500;">
                                  Stream
                                </span>
                              </Show>
                              <Show when={model.capabilities.reasoning}>
                                <span style="font-size: 10.5px; color: #c2410c; background: #fff7ed; border: 1px solid #ffedd5; padding: 1px 6px; border-radius: 4px; font-weight: 500;">
                                  Reasoning
                                </span>
                              </Show>
                              <Show when={model.capabilities.vision}>
                                <span style="font-size: 10.5px; color: #7e22ce; background: #faf5ff; border: 1px solid #f3e8ff; padding: 1px 6px; border-radius: 4px; font-weight: 500;">
                                  Vision
                                </span>
                              </Show>
                            </div>
                          </div>

                          <div style="display: flex; align-items: center; gap: 6px;">
                            <button
                              onClick={() => {
                                gatewayStore.setActiveModel(model.id);
                                toast.success(`已设置默认编码模型: ${model.name}`);
                              }}
                              style={`padding: 5px 12px; border-radius: 6px; font-size: 12px; font-weight: 500; cursor: pointer; ${
                                gatewayStore.activeModelId() === model.id
                                  ? 'background: #dcfce7; border: 1px solid #bbf7d0; color: #166534;'
                                  : 'background: #f7f6f2; border: 1px solid #e7e4dc; color: #2d2b28;'
                              }`}
                            >
                              {gatewayStore.activeModelId() === model.id ? '✓ 默认模型' : '设为默认'}
                            </button>
                            <Show when={currentProvider().models.length > 1}>
                              <button
                                onClick={() => {
                                  gatewayStore.deleteModel(currentProvider().id, model.id);
                                  toast.info(`已移除模型 ${model.name}`);
                                }}
                                style="background: transparent; border: none; font-size: 13px; color: #a8a49c; cursor: pointer; padding: 4px;"
                                title="删除模型"
                              >
                                🗑️
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
              <div style="flex: 1; padding: 24px; display: flex; flex-direction: column; gap: 18px; overflow-y: auto;">
                <div style="display: flex; flex-direction: column; gap: 4px;">
                  <h3 style="font-size: 15px; font-weight: 600; color: #1e1d1b; margin: 0;">
                    Agent 执行策略与权限沙箱 (Execution Policy)
                  </h3>
                  <span style="font-size: 12.5px; color: #76736c;">
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
                      sandboxLevel() === 'bypass' ? 'background: #fbfaf8; border-color: #c2410c;' : 'background: #ffffff; border-color: #eae7e1;'
                    }`}
                  >
                    <div style="display: flex; align-items: center; justify-content: space-between;">
                      <b style="font-size: 13.5px; color: #1e1d1b;">⚡ Bypass permissions (自动执行 - 推荐)</b>
                      <Show when={sandboxLevel() === 'bypass'}>
                        <span style="color: #c2410c; font-size: 12px; font-weight: bold;">● 当前生效</span>
                      </Show>
                    </div>
                    <span style="font-size: 12px; color: #76736c;">
                      Agent 自动执行文件读写、创建新文件与常规 Shell 命令，无需手动弹窗确认，适合快速迭代开发。
                    </span>
                  </div>

                  <div
                    onClick={() => {
                      setSandboxLevel('confirm');
                      toast.info('已开启 Ask for confirmation 审批模式');
                    }}
                    style={`padding: 14px; border-radius: 10px; border: 1px solid; cursor: pointer; display: flex; flex-direction: column; gap: 4px; ${
                      sandboxLevel() === 'confirm' ? 'background: #fbfaf8; border-color: #c2410c;' : 'background: #ffffff; border-color: #eae7e1;'
                    }`}
                  >
                    <div style="display: flex; align-items: center; justify-content: space-between;">
                      <b style="font-size: 13.5px; color: #1e1d1b;">🛡️ Ask for confirmation (人工逐项审批)</b>
                      <Show when={sandboxLevel() === 'confirm'}>
                        <span style="color: #c2410c; font-size: 12px; font-weight: bold;">● 当前生效</span>
                      </Show>
                    </div>
                    <span style="font-size: 12px; color: #76736c;">
                      在执行破坏性文件修改、删除文件或高风险 Shell 脚本前，弹出确认对话框由您手工批准。
                    </span>
                  </div>

                  <div
                    onClick={() => {
                      setSandboxLevel('strict');
                      toast.info('已开启 Read-only 严格只读模式');
                    }}
                    style={`padding: 14px; border-radius: 10px; border: 1px solid; cursor: pointer; display: flex; flex-direction: column; gap: 4px; ${
                      sandboxLevel() === 'strict' ? 'background: #fbfaf8; border-color: #c2410c;' : 'background: #ffffff; border-color: #eae7e1;'
                    }`}
                  >
                    <div style="display: flex; align-items: center; justify-content: space-between;">
                      <b style="font-size: 13.5px; color: #1e1d1b;">🔒 Strict Read-Only Sandbox (严格只读)</b>
                      <Show when={sandboxLevel() === 'strict'}>
                        <span style="color: #c2410c; font-size: 12px; font-weight: bold;">● 当前生效</span>
                      </Show>
                    </div>
                    <span style="font-size: 12px; color: #76736c;">
                      Agent 仅允许读取代码并回答问题，禁止向磁盘写入任何变更或启动终端子进程。
                    </span>
                  </div>
                </div>
              </div>
            </Show>

            {/* 3. 自定义指令面板 */}
            <Show when={activeMainTab() === 'prompt'}>
              <div style="flex: 1; padding: 24px; display: flex; flex-direction: column; gap: 16px; overflow-y: auto;">
                <div style="display: flex; flex-direction: column; gap: 4px;">
                  <h3 style="font-size: 15px; font-weight: 600; color: #1e1d1b; margin: 0;">
                    系统提示词与自定义指令 (Custom Instructions)
                  </h3>
                  <span style="font-size: 12.5px; color: #76736c;">
                    注入到每一轮 Agent 对话中的顶级指导原则，可定义架构偏好、编码规范与禁忌。
                  </span>
                </div>

                <textarea
                  rows={10}
                  value={systemPrompt()}
                  onInput={(e) => setSystemPrompt(e.currentTarget.value)}
                  style="width: 100%; background: #faf9f6; border: 1px solid #e7e4dc; border-radius: 10px; padding: 12px; font-size: 13px; line-height: 1.5; color: #2d2b28; outline: none; resize: vertical; font-family: inherit;"
                />

                <div style="display: flex; gap: 8px;">
                  <button
                    onClick={() => {
                      setSystemPrompt('You are a professional software architect. Prioritize clean microkernel design, modularity, and zero framework regressions.');
                      toast.info('已载入架构师提示词模板');
                    }}
                    style="padding: 6px 12px; background: #f0eee8; border: 1px solid #e2ded5; border-radius: 6px; font-size: 12px; color: #4b4843; cursor: pointer;"
                  >
                    📝 载入架构师模板
                  </button>
                  <button
                    onClick={() => {
                      setSystemPrompt('You are a strict test-driven development (TDD) engineer. Always verify with automated tests before completing tasks.');
                      toast.info('已载入 TDD 工程师模板');
                    }}
                    style="padding: 6px 12px; background: #f0eee8; border: 1px solid #e2ded5; border-radius: 6px; font-size: 12px; color: #4b4843; cursor: pointer;"
                  >
                    🧪 载入 TDD 模板
                  </button>
                </div>
              </div>
            </Show>

            {/* 4. 通用与外观面板 */}
            <Show when={activeMainTab() === 'general'}>
              <div style="flex: 1; padding: 24px; display: flex; flex-direction: column; gap: 18px; overflow-y: auto;">
                <div style="display: flex; flex-direction: column; gap: 6px;">
                  <label style="font-size: 13px; font-weight: 600; color: #2d2b28;">外观主题 (Appearance Theme):</label>
                  <select
                    value={themeMode()}
                    onChange={(e) => {
                      setThemeMode(e.currentTarget.value);
                      document.documentElement.setAttribute('data-theme', e.currentTarget.value);
                      toast.info(`主题已切换为 ${e.currentTarget.value}`);
                    }}
                    style="background: #faf9f6; border: 1px solid #e7e4dc; border-radius: 8px; padding: 8px 12px; color: #2d2b28; font-size: 13px;"
                  >
                    <option value="light">Claude Light (暖白极简 - 推荐)</option>
                    <option value="dark">Navis Dark (深色极客)</option>
                  </select>
                </div>

                <div style="display: flex; flex-direction: column; gap: 6px;">
                  <label style="font-size: 13px; font-weight: 600; color: #2d2b28;">默认工作区目录 (Workspace Root):</label>
                  <input
                    type="text"
                    value="D:\myworkspace\Navis Go"
                    readonly
                    style="background: #fbfaf8; border: 1px solid #e7e4dc; border-radius: 8px; padding: 8px 12px; color: #76736c; font-size: 13px;"
                  />
                </div>
              </div>
            </Show>
          </div>

          {/* 底部确认栏 */}
          <div style="height: 52px; background: #fbfaf8; border-top: 1px solid #eae7e1; display: flex; align-items: center; justify-content: space-between; padding: 0 20px;">
            <div style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #76736c;">
              <span>当前默认模型:</span>
              <b style="color: #1e1d1b;">{gatewayStore.activeProvider().name} · {gatewayStore.activeModel().name}</b>
            </div>

            <div style="display: flex; align-items: center; gap: 10px;">
              <button
                onClick={() => setOpen(false)}
                style="padding: 7px 14px; background: transparent; border: 1px solid #e7e4dc; border-radius: 6px; font-size: 13px; font-weight: 500; color: #5a5750; cursor: pointer;"
              >
                取消
              </button>
              <button
                onClick={handleSaveAll}
                style="padding: 7px 18px; background: #2d2b28; border: none; border-radius: 6px; font-size: 13px; font-weight: 500; color: #ffffff; cursor: pointer;"
              >
                保存并生效
              </button>
            </div>
          </div>
        </div>
      </div>
    </Show>
  );
};

export default SettingsModal;
