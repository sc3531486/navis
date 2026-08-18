import { invoke } from '@tauri-apps/api/core';

export interface SessionContextUsage {
  sessionId: string;
  model?: string | null;
  usedTokens: number;
  totalTokens: number;
  usedPercent: number;
  compressionThresholdPercent: number;
}

export interface ContextUsageDisplaySnapshot {
  usedTokens: number;
  totalTokens: number;
  usedPercent: number;
  compressionThresholdPercent: number;
  usedTokensLabel: string;
  totalTokensLabel: string;
  usedRatioLabel: string;
  compressionThresholdLabel: string;
}

export function formatContextTokenCount(tokens: number): string {
  if (tokens >= 1_000_000) {
    const value = tokens / 1_000_000;
    return `${Number.isInteger(value) ? value.toFixed(0) : value.toFixed(1)}M`;
  }

  if (tokens >= 1_000) {
    const value = tokens / 1_000;
    return `${Number.isInteger(value) ? value.toFixed(0) : value.toFixed(1)}K`;
  }

  return String(tokens);
}

export function contextUsageDisplaySnapshot(
  usage: SessionContextUsage | null | undefined,
): ContextUsageDisplaySnapshot | null {
  if (!usage) return null;

  return {
    usedTokens: usage.usedTokens,
    totalTokens: usage.totalTokens,
    usedPercent: usage.usedPercent,
    compressionThresholdPercent: usage.compressionThresholdPercent,
    usedTokensLabel: formatContextTokenCount(usage.usedTokens),
    totalTokensLabel: formatContextTokenCount(usage.totalTokens),
    usedRatioLabel: `${formatContextTokenCount(usage.usedTokens)} / ${formatContextTokenCount(usage.totalTokens)}`,
    compressionThresholdLabel: `Compression threshold ${usage.compressionThresholdPercent}%`,
  };
}

export async function loadSessionContextUsage(sessionId: string): Promise<SessionContextUsage> {
  return invoke<SessionContextUsage>('ui_get_session_context_usage', {
    payload: { sessionId },
  });
}
