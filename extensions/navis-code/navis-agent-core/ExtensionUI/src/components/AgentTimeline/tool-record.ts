import type { ChatAgentTimelinePart } from '@/lib/stream';

export const partRecord = (value: Record<string, unknown> | null | undefined): Record<string, unknown> =>
  value && typeof value === 'object' ? value : {};

export const recordString = (record: Record<string, unknown>, key: string): string | undefined => {
  const value = record[key];
  return typeof value === 'string' && value.trim() ? value : undefined;
};

export const recordNumber = (record: Record<string, unknown>, key: string): number | undefined => {
  const value = record[key];
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
};

export const recordBoolean = (record: Record<string, unknown>, key: string): boolean | undefined => {
  const value = record[key];
  return typeof value === 'boolean' ? value : undefined;
};

export const recordArray = (record: Record<string, unknown>, key: string): Record<string, unknown>[] => {
  const value = record[key];
  return Array.isArray(value)
    ? value.filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === 'object')
    : [];
};

export const recordStringArray = (record: Record<string, unknown>, key: string): string[] => {
  const value = record[key];
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [];
};

export const firstRecordArray = (record: Record<string, unknown>, keys: readonly string[]): Record<string, unknown>[] => {
  for (const key of keys) {
    const value = recordArray(record, key);
    if (value.length > 0) return value;
  }
  return [];
};

export const firstStringArray = (record: Record<string, unknown>, keys: readonly string[]): string[] => {
  for (const key of keys) {
    const value = recordStringArray(record, key);
    if (value.length > 0) return value;
  }
  return [];
};

export const toolInput = (part: ChatAgentTimelinePart): Record<string, unknown> => partRecord(part.input);
export const toolOutput = (part: ChatAgentTimelinePart): Record<string, unknown> => partRecord(part.output);
export const toolMetadata = (part: ChatAgentTimelinePart): Record<string, unknown> => partRecord(part.metadata);
export const toolProgress = (part: ChatAgentTimelinePart): Record<string, unknown> => partRecord(part.progress);
