import type { NavisContext } from '@/core/context';
import {
  gatewayStore,
  type ProviderItem,
  type ModelItem,
} from '@extensions/shared/navis-ai-platform/ExtensionUI/src/store/GatewayStore';

export interface AgentPromptPayload {
  content: string;
  model: string;
  modelId: string;
  provider?: string;
  permission?: string;
  reasoning?: string;
  timestamp: number;
}

export interface StreamCallbacks {
  onThinkingDelta?: (delta: string) => void;
  onContentDelta?: (delta: string) => void;
  onToolCall?: (toolCall: {
    id: string;
    toolName: string;
    argsSummary: string;
    outputSummary?: string;
    status: 'pending' | 'approved' | 'rejected' | 'completed';
    needsApproval?: boolean;
  }) => void;
  onComplete?: (result: {
    content: string;
    thinking?: string;
    tokensUsage?: { prompt: number; completion: number; total: number; cost: string };
  }) => void;
  onError?: (error: Error) => void;
}

export class AgentService {
  private ctx: NavisContext;

  constructor(ctx: NavisContext) {
    this.ctx = ctx;
  }

  /**
   * 真实发起流式或实时 LLM 调用，通过 SSE 解析逐字输出
   */
  async streamTurn(
    payload: AgentPromptPayload,
    history: Array<{ role: 'user' | 'assistant' | 'system'; content: string }>,
    callbacks: StreamCallbacks,
  ): Promise<void> {
    const provider = gatewayStore.activeProvider();
    const model = gatewayStore.activeModel();

    if (!provider || !provider.baseUrl) {
      const err = new Error('未配置有效的 AI 服务商端点，请在设置中配置服务商');
      callbacks.onError?.(err);
      return;
    }

    const rawUrl = provider.baseUrl.replace(/\/+$/, '');
    const modelId = model?.id || payload.modelId || provider.defaultModelId || 'gemini-3.7-flash';
    const protocol =
      provider.upstreamProtocol ||
      (provider.type === 'anthropic' ? 'anthropic_messages' : 'chat_completions');

    // 格式化历史消息上下文
    const systemPrompt =
      'You are Navis Code, an expert agentic AI software engineer. Provide high quality, concise, and structured answers.';
    const messages = [
      { role: 'system', content: systemPrompt },
      ...history
        .filter((h) => h.content && h.content.trim().length > 0)
        .map((h) => ({ role: h.role, content: h.content })),
      { role: 'user', content: payload.content },
    ];

    try {
      if (protocol === 'anthropic_messages' || provider.type === 'anthropic') {
        await this.streamAnthropic(rawUrl, provider, modelId, messages, callbacks);
      } else {
        await this.streamOpenAI(rawUrl, provider, modelId, messages, callbacks);
      }
    } catch (err: any) {
      console.error('[AgentService] Upstream streaming request failed:', err);
      callbacks.onError?.(
        new Error(
          `无法连接到服务商 ${provider.name} (${rawUrl})：${err?.message || '网络连接失败或服务未响应'}。请检查服务是否开启或在设置中更换端点。`,
        ),
      );
    }
  }

  /**
   * OpenAI / DeepSeek / Ollama / Local Gateway SSE 流式调用
   */
  private async streamOpenAI(
    baseUrl: string,
    provider: ProviderItem,
    modelId: string,
    messages: Array<{ role: string; content: string }>,
    callbacks: StreamCallbacks,
  ): Promise<void> {
    const endpoint = baseUrl.endsWith('/chat/completions')
      ? baseUrl
      : `${baseUrl}/chat/completions`;

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (provider.apiKey && provider.apiKey !== 'sk-gateway-local-token') {
      headers['Authorization'] = `Bearer ${provider.apiKey}`;
    }

    const body = {
      model: modelId,
      messages,
      stream: true,
      temperature: 0.3,
    };

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 5000);

    let res: Response;
    try {
      res = await fetch(endpoint, {
        method: 'POST',
        headers,
        body: JSON.stringify(body),
        signal: controller.signal,
      });
    } catch (e: any) {
      clearTimeout(timeoutId);
      if (e?.name === 'AbortError') {
        throw new Error(`连接超时 (5s)：未能在 ${endpoint} 检测到运行中的服务，请确认本地网关或 LLM 服务已启动。`);
      }
      throw new Error(`网络连接失败 (${e?.message || 'Connection refused'})：无法访问 ${endpoint}`);
    } finally {
      clearTimeout(timeoutId);
    }

    if (!res.ok) {
      let errMsg = `HTTP ${res.status} ${res.statusText}`;
      try {
        const errJson = await res.json();
        errMsg = errJson?.error?.message || errJson?.message || errMsg;
      } catch (_) {}
      throw new Error(errMsg);
    }

    if (!res.body) {
      throw new Error('Response body is empty');
    }

    const reader = res.body.getReader();
    const decoder = new TextDecoder('utf-8');
    let buffer = '';
    let fullContent = '';
    let fullThinking = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith(':')) continue; // 注释或空行

        if (trimmed.startsWith('data:')) {
          const dataStr = trimmed.slice(5).trim();
          if (dataStr === '[DONE]') {
            break;
          }

          try {
            const data = JSON.parse(dataStr);
            const choice = data.choices?.[0];
            const delta = choice?.delta;

            // 思考过程增量 (DeepSeek / Qwen / Gemini 思考模型)
            if (delta?.reasoning_content) {
              fullThinking += delta.reasoning_content;
              callbacks.onThinkingDelta?.(delta.reasoning_content);
            }

            // 正文回答增量
            if (delta?.content) {
              fullContent += delta.content;
              callbacks.onContentDelta?.(delta.content);
            }
          } catch (_) {
            // 忽略单行 JSON 格式解析容错
          }
        }
      }
    }

    const estTokens = Math.round((fullContent.length + fullThinking.length) / 3.5);
    callbacks.onComplete?.({
      content: fullContent,
      thinking: fullThinking || undefined,
      tokensUsage: {
        prompt: Math.round(messages.reduce((a, b) => a + b.content.length, 0) / 3.5),
        completion: estTokens,
        total: Math.round(messages.reduce((a, b) => a + b.content.length, 0) / 3.5) + estTokens,
        cost: `$${((estTokens * 0.002) / 1000).toFixed(4)}`,
      },
    });
  }

  /**
   * Anthropic Messages SSE 流式调用
   */
  private async streamAnthropic(
    baseUrl: string,
    provider: ProviderItem,
    modelId: string,
    messages: Array<{ role: string; content: string }>,
    callbacks: StreamCallbacks,
  ): Promise<void> {
    const endpoint = baseUrl.endsWith('/messages') ? baseUrl : `${baseUrl}/messages`;

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'x-api-key': provider.apiKey,
      'anthropic-version': '2023-06-01',
    };

    const body = {
      model: modelId,
      max_tokens: 4096,
      messages: messages.filter((m) => m.role !== 'system'),
      system: messages.find((m) => m.role === 'system')?.content,
      stream: true,
    };

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 5000);

    let res: Response;
    try {
      res = await fetch(endpoint, {
        method: 'POST',
        headers,
        body: JSON.stringify(body),
        signal: controller.signal,
      });
    } catch (e: any) {
      clearTimeout(timeoutId);
      if (e?.name === 'AbortError') {
        throw new Error(`连接超时 (5s)：未能在 ${endpoint} 检测到响应，请确认 API Key 与网络连接正常。`);
      }
      throw new Error(`网络连接失败 (${e?.message || 'Connection refused'})：无法访问 ${endpoint}`);
    } finally {
      clearTimeout(timeoutId);
    }

    if (!res.ok) {
      let errMsg = `HTTP ${res.status} ${res.statusText}`;
      try {
        const errJson = await res.json();
        errMsg = errJson?.error?.message || errMsg;
      } catch (_) {}
      throw new Error(errMsg);
    }

    if (!res.body) {
      throw new Error('Response body is empty');
    }

    const reader = res.body.getReader();
    const decoder = new TextDecoder('utf-8');
    let buffer = '';
    let fullContent = '';
    let fullThinking = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith(':')) continue;

        if (trimmed.startsWith('data:')) {
          const dataStr = trimmed.slice(5).trim();
          try {
            const data = JSON.parse(dataStr);
            if (data.type === 'content_block_delta') {
              if (data.delta?.type === 'thinking_delta') {
                fullThinking += data.delta.thinking;
                callbacks.onThinkingDelta?.(data.delta.thinking);
              } else if (data.delta?.type === 'text_delta') {
                fullContent += data.delta.text;
                callbacks.onContentDelta?.(data.delta.text);
              }
            }
          } catch (_) {}
        }
      }
    }

    callbacks.onComplete?.({
      content: fullContent,
      thinking: fullThinking || undefined,
      tokensUsage: {
        prompt: Math.round(messages.reduce((a, b) => a + b.content.length, 0) / 3.5),
        completion: Math.round((fullContent.length + fullThinking.length) / 3.5),
        total: Math.round(
          (messages.reduce((a, b) => a + b.content.length, 0) +
            fullContent.length +
            fullThinking.length) /
            3.5,
        ),
        cost: '$0.003',
      },
    });
  }
}
