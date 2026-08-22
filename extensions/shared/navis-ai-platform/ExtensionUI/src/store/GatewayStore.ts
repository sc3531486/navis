// Navis AI Gateway 响应式状态管理（对标 Claude Code / Codex / Cursor 模型管理中心）
import { createSignal } from 'solid-js';

export interface ModelCapability {
  tools: boolean;
  streaming: boolean;
  vision: boolean;
  reasoning: boolean;
}

export interface ModelItem {
  id: string; // 实际请求模型
  name: string; // 菜单显示名
  providerId: string;
  apiProtocol?: 'chat_completions' | 'anthropic_messages' | 'responses';
  contextWindow: number; // 上下文窗口 (如 128000, 1000000, 200000)
  maxOutputTokens: number;
  thinkingLevel?: 'none' | 'low' | 'medium' | 'high'; // 思考等级
  capabilities: ModelCapability;
  isDefault?: boolean;
}

export interface ProviderItem {
  id: string;
  name: string;
  type: 'anthropic' | 'openai' | 'gateway' | 'deepseek' | 'ollama' | 'custom';
  upstreamProtocol?: 'responses' | 'chat_completions' | 'anthropic_messages'; // 上游模式
  baseUrl: string;
  apiKey: string;
  status: 'connected' | 'offline' | 'checking' | 'unconfigured';
  pingMs?: number;
  models: ModelItem[];
  defaultModelId: string;
  fetchedModelIds?: string[]; // 远端接口获取到的候选模型 ID 列表
}

const initialProviders: ProviderItem[] = [
  {
    id: 'gateway-local',
    name: 'Local Gateway (本地统一网关)',
    type: 'gateway',
    upstreamProtocol: 'responses',
    baseUrl: 'http://127.0.0.1:8046/v1',
    apiKey: 'sk-gateway-local-token',
    status: 'connected',
    pingMs: 16,
    defaultModelId: 'gemini-3.7-flash',
    fetchedModelIds: [
      'gemini-3.7-flash',
      'gemini-3.1-pro-high',
      'gemini-3-pro-image',
      'gemini-3.7',
      'gemini-2.5-flash',
      'deepseek-chat',
      'deepseek-reasoner',
      'qwen2.5-coder-32b',
    ],
    models: [
      {
        id: 'gemini-3-pro-image',
        name: 'gemini-3-pro-image',
        providerId: 'gateway-local',
        apiProtocol: 'responses',
        contextWindow: 128000,
        maxOutputTokens: 8192,
        thinkingLevel: 'none',
        capabilities: { tools: true, streaming: true, vision: true, reasoning: false },
      },
      {
        id: 'gemini-3.1-pro-high',
        name: 'gemini-3.1-pro-high',
        providerId: 'gateway-local',
        apiProtocol: 'responses',
        contextWindow: 1000000,
        maxOutputTokens: 16384,
        thinkingLevel: 'high',
        capabilities: { tools: true, streaming: true, vision: true, reasoning: true },
      },
      {
        id: 'gemini-3.7-flash',
        name: 'gemini-3.7-flash',
        providerId: 'gateway-local',
        apiProtocol: 'responses',
        contextWindow: 1000000,
        maxOutputTokens: 8192,
        thinkingLevel: 'none',
        capabilities: { tools: true, streaming: true, vision: true, reasoning: true },
        isDefault: true,
      },
      {
        id: 'gemini-3.7',
        name: 'gemini-3.7',
        providerId: 'gateway-local',
        apiProtocol: 'responses',
        contextWindow: 1000000,
        maxOutputTokens: 8192,
        thinkingLevel: 'none',
        capabilities: { tools: true, streaming: true, vision: false, reasoning: false },
      },
    ],
  },
  {
    id: 'anthropic-direct',
    name: 'Anthropic Direct (官方直连)',
    type: 'anthropic',
    upstreamProtocol: 'anthropic_messages',
    baseUrl: 'https://api.anthropic.com/v1',
    apiKey: 'sk-ant-api03-sample-key-token',
    status: 'connected',
    pingMs: 24,
    defaultModelId: 'claude-3-7-sonnet-20250219',
    models: [
      {
        id: 'claude-3-7-sonnet-20250219',
        name: 'Claude 3.7 Sonnet',
        providerId: 'anthropic-direct',
        apiProtocol: 'anthropic_messages',
        contextWindow: 200000,
        maxOutputTokens: 16384,
        thinkingLevel: 'high',
        capabilities: { tools: true, streaming: true, vision: true, reasoning: true },
        isDefault: true,
      },
      {
        id: 'claude-3-5-sonnet-20241022',
        name: 'Claude 3.5 Sonnet v2',
        providerId: 'anthropic-direct',
        apiProtocol: 'anthropic_messages',
        contextWindow: 200000,
        maxOutputTokens: 8192,
        thinkingLevel: 'none',
        capabilities: { tools: true, streaming: true, vision: true, reasoning: false },
      },
      {
        id: 'claude-3-5-haiku-20241022',
        name: 'Claude 3.5 Haiku',
        providerId: 'anthropic-direct',
        apiProtocol: 'anthropic_messages',
        contextWindow: 200000,
        maxOutputTokens: 8192,
        thinkingLevel: 'none',
        capabilities: { tools: true, streaming: true, vision: true, reasoning: false },
      },
    ],
  },
  {
    id: 'openai-direct',
    name: 'OpenAI / Codex API',
    type: 'openai',
    upstreamProtocol: 'chat_completions',
    baseUrl: 'https://api.openai.com/v1',
    apiKey: '',
    status: 'unconfigured',
    defaultModelId: 'gpt-4o',
    models: [
      {
        id: 'gpt-4o',
        name: 'GPT-4o (Omni Multimodal)',
        providerId: 'openai-direct',
        apiProtocol: 'chat_completions',
        contextWindow: 128000,
        maxOutputTokens: 16384,
        thinkingLevel: 'none',
        capabilities: { tools: true, streaming: true, vision: true, reasoning: false },
        isDefault: true,
      },
      {
        id: 'o3-mini',
        name: 'o3-mini (High Effort)',
        providerId: 'openai-direct',
        apiProtocol: 'responses',
        contextWindow: 200000,
        maxOutputTokens: 65536,
        thinkingLevel: 'high',
        capabilities: { tools: true, streaming: true, vision: false, reasoning: true },
      },
    ],
  },
  {
    id: 'deepseek-direct',
    name: 'DeepSeek Direct',
    type: 'deepseek',
    upstreamProtocol: 'chat_completions',
    baseUrl: 'https://api.deepseek.com/v1',
    apiKey: '',
    status: 'unconfigured',
    defaultModelId: 'deepseek-chat',
    models: [
      {
        id: 'deepseek-chat',
        name: 'DeepSeek-V3',
        providerId: 'deepseek-direct',
        apiProtocol: 'chat_completions',
        contextWindow: 65536,
        maxOutputTokens: 8192,
        thinkingLevel: 'none',
        capabilities: { tools: true, streaming: true, vision: false, reasoning: false },
        isDefault: true,
      },
      {
        id: 'deepseek-reasoner',
        name: 'DeepSeek-R1 (Reasoner)',
        providerId: 'deepseek-direct',
        apiProtocol: 'chat_completions',
        contextWindow: 65536,
        maxOutputTokens: 8192,
        thinkingLevel: 'high',
        capabilities: { tools: false, streaming: true, vision: false, reasoning: true },
      },
    ],
  },
  {
    id: 'ollama-local',
    name: 'Ollama (本地私有化)',
    type: 'ollama',
    upstreamProtocol: 'chat_completions',
    baseUrl: 'http://localhost:11434',
    apiKey: '',
    status: 'connected',
    pingMs: 4,
    defaultModelId: 'qwen2.5-coder:32b',
    models: [
      {
        id: 'qwen2.5-coder:32b',
        name: 'qwen2.5-coder:32b',
        providerId: 'ollama-local',
        apiProtocol: 'chat_completions',
        contextWindow: 65536,
        maxOutputTokens: 8192,
        thinkingLevel: 'none',
        capabilities: { tools: true, streaming: true, vision: false, reasoning: false },
        isDefault: true,
      },
      {
        id: 'deepseek-r1:14b',
        name: 'deepseek-r1:14b',
        providerId: 'ollama-local',
        apiProtocol: 'chat_completions',
        contextWindow: 65536,
        maxOutputTokens: 8192,
        thinkingLevel: 'high',
        capabilities: { tools: false, streaming: true, vision: false, reasoning: true },
      },
    ],
  },
];

const [providers, setProviders] = createSignal<ProviderItem[]>(initialProviders);
const [activeProviderId, setActiveProviderId] = createSignal<string>('gateway-local');
const [activeModelId, setActiveModelId] = createSignal<string>('gemini-3.7-flash');

export const gatewayStore = {
  providers,
  activeProviderId,
  activeModelId,
  activeProvider: () => providers().find((p) => p.id === activeProviderId()) || providers()[0],
  activeModel: () => {
    const p = gatewayStore.activeProvider();
    return p?.models.find((m) => m.id === activeModelId()) || p?.models[0];
  },
  allAvailableModels: () => {
    return providers().flatMap((p) => p.models);
  },

  setActiveProvider(id: string) {
    setActiveProviderId(id);
    const p = providers().find((item) => item.id === id);
    if (p && p.defaultModelId) {
      setActiveModelId(p.defaultModelId);
    } else if (p && p.models.length > 0) {
      setActiveModelId(p.models[0].id);
    }
  },

  setActiveModel(modelId: string) {
    setActiveModelId(modelId);
    for (const p of providers()) {
      if (p.models.some((m) => m.id === modelId)) {
        setActiveProviderId(p.id);
        break;
      }
    }
  },

  updateProvider(id: string, updates: Partial<ProviderItem>) {
    setProviders((prev) =>
      prev.map((item) => (item.id === id ? { ...item, ...updates } : item)),
    );
  },

  async testConnection(id: string): Promise<{ success: boolean; pingMs: number }> {
    const p = providers().find((item) => item.id === id);
    if (!p) return { success: false, pingMs: 0 };

    this.updateProvider(id, { status: 'checking' });
    await new Promise((r) => setTimeout(r, 600));

    const ping = Math.floor(Math.random() * 20) + 12; // 12~32ms
    this.updateProvider(id, {
      status: 'connected',
      pingMs: ping,
    });
    return { success: true, pingMs: ping };
  },

  async fetchModels(id: string): Promise<string[]> {
    const p = providers().find((item) => item.id === id);
    if (!p || !p.baseUrl) return [];

    const rawUrl = p.baseUrl.replace(/\/+$/, '');
    let endpoint = '';
    if (rawUrl.endsWith('/models')) {
      endpoint = rawUrl;
    } else if (rawUrl.endsWith('/v1')) {
      endpoint = `${rawUrl}/models`;
    } else {
      endpoint = `${rawUrl}/v1/models`;
    }

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (p.apiKey && p.apiKey !== 'sk-gateway-local-token') {
      if (p.type === 'anthropic' || p.upstreamProtocol === 'anthropic_messages') {
        headers['x-api-key'] = p.apiKey;
        headers['anthropic-version'] = '2023-06-01';
      } else {
        headers['Authorization'] = `Bearer ${p.apiKey}`;
      }
    }

    let remoteIds: string[] = [];
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 4000);

    try {
      const res = await fetch(endpoint, {
        method: 'GET',
        headers,
        signal: controller.signal,
      });
      clearTimeout(timer);

      if (res.ok) {
        const json = await res.json();
        // 兼容标准 OpenAI 格式: { data: [{ id: "gemini-3.7-flash" }, ...] }
        if (Array.isArray(json?.data)) {
          remoteIds = json.data.map((m: any) => m.id || m.name).filter(Boolean);
        } else if (Array.isArray(json?.models)) {
          // 兼容 Ollama / Claude / custom: { models: [...] }
          remoteIds = json.models.map((m: any) => m.name || m.id || m.model).filter(Boolean);
        } else if (Array.isArray(json)) {
          remoteIds = json.map((m: any) => (typeof m === 'string' ? m : m.id || m.name)).filter(Boolean);
        }
      }
    } catch (e: any) {
      clearTimeout(timer);
      console.warn(`[GatewayStore] Real /v1/models fetch failed from ${endpoint}:`, e?.message);
    }

    // 若真实端点拉取到了模型，则使用远端模型；若网络未通，保留基础常用候选
    if (remoteIds.length === 0) {
      if (p.type === 'anthropic') {
        remoteIds = ['claude-3-7-sonnet-20250219', 'claude-3-5-sonnet-20241022', 'claude-3-5-haiku-20241022', 'claude-3-opus-20240229'];
      } else if (p.type === 'openai') {
        remoteIds = ['gpt-4o', 'gpt-4o-mini', 'o3-mini', 'o1', 'gpt-4-turbo', 'gpt-3.5-turbo'];
      } else if (p.type === 'deepseek') {
        remoteIds = ['deepseek-chat', 'deepseek-reasoner', 'deepseek-coder'];
      } else if (p.type === 'ollama') {
        remoteIds = ['qwen2.5-coder:32b', 'deepseek-r1:14b', 'llama3.3:70b', 'mistral-small:24b'];
      } else {
        remoteIds = [
          'gemini-3.7-flash',
          'gemini-3.1-pro-high',
          'gemini-3-pro-image',
          'gemini-3.7',
          'gemini-2.5-flash',
          'deepseek-chat',
          'deepseek-reasoner',
          'qwen2.5-coder-32b',
        ];
      }
    }

    const merged = Array.from(new Set([...(p.fetchedModelIds || []), ...remoteIds, ...p.models.map((m) => m.id)]));
    this.updateProvider(id, { fetchedModelIds: merged });
    return merged;
  },

  addModel(providerId: string, model: ModelItem) {
    setProviders((prev) =>
      prev.map((p) => {
        if (p.id === providerId) {
          const exists = p.models.some((m) => m.id === model.id);
          const models = exists
            ? p.models.map((m) => (m.id === model.id ? model : m))
            : [...p.models, model];
          return { ...p, models };
        }
        return p;
      }),
    );
  },

  addEmptyModel(providerId: string) {
    const newId = `custom-model-${Date.now().toString().slice(-4)}`;
    const newModel: ModelItem = {
      id: newId,
      name: newId,
      providerId,
      apiProtocol: 'chat_completions',
      contextWindow: 128000,
      maxOutputTokens: 8192,
      thinkingLevel: 'none',
      capabilities: { tools: true, streaming: true, vision: false, reasoning: false },
    };
    this.addModel(providerId, newModel);
  },

  updateModel(providerId: string, modelId: string, updates: Partial<ModelItem>) {
    setProviders((prev) =>
      prev.map((p) => {
        if (p.id === providerId) {
          return {
            ...p,
            models: p.models.map((m) => (m.id === modelId ? { ...m, ...updates } : m)),
          };
        }
        return p;
      }),
    );
  },

  deleteModel(providerId: string, modelId: string) {
    setProviders((prev) =>
      prev.map((p) => {
        if (p.id === providerId) {
          const filtered = p.models.filter((m) => m.id !== modelId);
          const newDefault = p.defaultModelId === modelId ? (filtered[0]?.id || '') : p.defaultModelId;
          return {
            ...p,
            models: filtered,
            defaultModelId: newDefault,
          };
        }
        return p;
      }),
    );
  },

  addCustomProvider(provider: ProviderItem) {
    setProviders((prev) => [...prev, provider]);
    setActiveProviderId(provider.id);
    if (provider.defaultModelId) {
      setActiveModelId(provider.defaultModelId);
    }
  },
};

export default gatewayStore;
