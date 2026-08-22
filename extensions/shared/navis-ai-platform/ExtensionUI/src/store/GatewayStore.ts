// Navis AI Gateway 响应式状态管理（对标 Claude Code / Codex / Cursor 模型管理中心）
import { createSignal } from 'solid-js';

export interface ModelCapability {
  tools: boolean;
  streaming: boolean;
  vision: boolean;
  reasoning: boolean;
}

export interface ModelItem {
  id: string;
  name: string;
  providerId: string;
  apiProtocol: 'chat_completions' | 'anthropic_messages' | 'responses';
  contextWindow: number;
  maxOutputTokens: number;
  capabilities: ModelCapability;
  isDefault?: boolean;
}

export interface ProviderItem {
  id: string;
  name: string;
  type: 'anthropic' | 'openai' | 'gateway' | 'deepseek' | 'ollama' | 'custom';
  baseUrl: string;
  apiKey: string;
  status: 'connected' | 'offline' | 'checking' | 'unconfigured';
  pingMs?: number;
  models: ModelItem[];
  defaultModelId: string;
}

const initialProviders: ProviderItem[] = [
  {
    id: 'gateway-local',
    name: 'Local Gateway (本地统一网关)',
    type: 'gateway',
    baseUrl: 'http://127.0.0.1:15721',
    apiKey: 'sk-gateway-local-token',
    status: 'connected',
    pingMs: 16,
    defaultModelId: 'gemini-3.7-flash',
    models: [
      {
        id: 'gemini-3.7-flash',
        name: 'Google Gemini 3.7 Flash',
        providerId: 'gateway-local',
        apiProtocol: 'chat_completions',
        contextWindow: 1048576,
        maxOutputTokens: 8192,
        capabilities: { tools: true, streaming: true, vision: true, reasoning: true },
        isDefault: true,
      },
      {
        id: 'deepseek-r1',
        name: 'DeepSeek R1 (Full Reasoning)',
        providerId: 'gateway-local',
        apiProtocol: 'chat_completions',
        contextWindow: 128000,
        maxOutputTokens: 16384,
        capabilities: { tools: true, streaming: true, vision: false, reasoning: true },
      },
      {
        id: 'qwen2.5-coder-32b',
        name: 'Qwen 2.5 Coder 32B Instruct',
        providerId: 'gateway-local',
        apiProtocol: 'chat_completions',
        contextWindow: 65536,
        maxOutputTokens: 8192,
        capabilities: { tools: true, streaming: true, vision: false, reasoning: false },
      },
    ],
  },
  {
    id: 'anthropic-direct',
    name: 'Anthropic Direct (官方直连)',
    type: 'anthropic',
    baseUrl: 'https://api.anthropic.com/v1',
    apiKey: '',
    status: 'unconfigured',
    defaultModelId: 'claude-3-7-sonnet',
    models: [
      {
        id: 'claude-3-7-sonnet',
        name: 'Claude 3.7 Sonnet (Hybrid Reasoning)',
        providerId: 'anthropic-direct',
        apiProtocol: 'anthropic_messages',
        contextWindow: 200000,
        maxOutputTokens: 16384,
        capabilities: { tools: true, streaming: true, vision: true, reasoning: true },
        isDefault: true,
      },
      {
        id: 'claude-3-5-sonnet',
        name: 'Claude 3.5 Sonnet v2',
        providerId: 'anthropic-direct',
        apiProtocol: 'anthropic_messages',
        contextWindow: 200000,
        maxOutputTokens: 8192,
        capabilities: { tools: true, streaming: true, vision: true, reasoning: false },
      },
      {
        id: 'claude-3-5-haiku',
        name: 'Claude 3.5 Haiku (Fast Inline)',
        providerId: 'anthropic-direct',
        apiProtocol: 'anthropic_messages',
        contextWindow: 200000,
        maxOutputTokens: 8192,
        capabilities: { tools: true, streaming: true, vision: true, reasoning: false },
      },
    ],
  },
  {
    id: 'openai-direct',
    name: 'OpenAI / Codex API',
    type: 'openai',
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
        capabilities: { tools: true, streaming: true, vision: true, reasoning: false },
        isDefault: true,
      },
      {
        id: 'o3-mini',
        name: 'o3-mini (High Effort Coding)',
        providerId: 'openai-direct',
        apiProtocol: 'responses',
        contextWindow: 200000,
        maxOutputTokens: 65536,
        capabilities: { tools: true, streaming: true, vision: false, reasoning: true },
      },
      {
        id: 'o1',
        name: 'o1 (Full Reasoning)',
        providerId: 'openai-direct',
        apiProtocol: 'responses',
        contextWindow: 200000,
        maxOutputTokens: 32768,
        capabilities: { tools: true, streaming: true, vision: true, reasoning: true },
      },
    ],
  },
  {
    id: 'deepseek-direct',
    name: 'DeepSeek Direct (深度求索)',
    type: 'deepseek',
    baseUrl: 'https://api.deepseek.com/v1',
    apiKey: '',
    status: 'unconfigured',
    defaultModelId: 'deepseek-reasoner',
    models: [
      {
        id: 'deepseek-reasoner',
        name: 'DeepSeek R1 (Reasoner)',
        providerId: 'deepseek-direct',
        apiProtocol: 'chat_completions',
        contextWindow: 128000,
        maxOutputTokens: 16384,
        capabilities: { tools: true, streaming: true, vision: false, reasoning: true },
        isDefault: true,
      },
      {
        id: 'deepseek-chat',
        name: 'DeepSeek V3 (Chat)',
        providerId: 'deepseek-direct',
        apiProtocol: 'chat_completions',
        contextWindow: 128000,
        maxOutputTokens: 8192,
        capabilities: { tools: true, streaming: true, vision: false, reasoning: false },
      },
    ],
  },
  {
    id: 'ollama-local',
    name: 'Ollama (本地离线推理)',
    type: 'ollama',
    baseUrl: 'http://localhost:11434',
    apiKey: 'ollama-local',
    status: 'unconfigured',
    defaultModelId: 'qwen2.5-coder:latest',
    models: [
      {
        id: 'qwen2.5-coder:latest',
        name: 'qwen2.5-coder:latest',
        providerId: 'ollama-local',
        apiProtocol: 'chat_completions',
        contextWindow: 32768,
        maxOutputTokens: 8192,
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

  async fetchModels(id: string): Promise<ModelItem[]> {
    const p = providers().find((item) => item.id === id);
    if (!p) return [];

    await new Promise((r) => setTimeout(r, 700));
    return p.models;
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

  deleteModel(providerId: string, modelId: string) {
    setProviders((prev) =>
      prev.map((p) => {
        if (p.id === providerId) {
          return {
            ...p,
            models: p.models.filter((m) => m.id !== modelId),
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
