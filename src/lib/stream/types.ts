/**
 * 统一流数据类型定义
 * 来源：02b-stream.md §5.3
 */

// ── 标准 Stream Envelope ─────────────────────────────────

export type StreamChunkKind = 'data' | 'error' | 'done' | 'cancelled';

export interface StandardStreamChunk<T = unknown> {
  streamId: string;
  sequence: number;
  kind: StreamChunkKind;
  data: T;
  isFinal: boolean;
}

export type StreamCompletion = 'channel' | 'invoke' | 'manual';

export type StreamTermination =
  | { kind: 'completed' }
  | { kind: 'stopped'; reason?: string }
  | { kind: 'cancelled'; reason?: string }
  | { kind: 'error'; error: Error; envelope?: StandardStreamChunk<unknown> }
  | { kind: 'creation_error'; error: Error };

export interface StreamRunOptions<T, R = unknown> {
  command: string;
  args?: Record<string, unknown>;
  completion?: StreamCompletion;
  onChunk: (chunk: T, envelope: StandardStreamChunk<T>) => void;
  onCreated?: (result: R) => void;
  disposeLateResource?: (result: R) => void;
  onTermination?: (termination: StreamTermination) => void;
}

export interface StreamRunController {
  stop: (reason?: string) => void;
  complete: () => void;
  streamId: () => string | null;
  finished: () => boolean;
  termination: () => StreamTermination | null;
}

export interface ChatMessagesStreamChunk {
  type: 'messages';
  messages: ChatMessageStreamItem[];
  total: number;
}

export type AgentTimelinePartKind =
  | 'tool'
  | 'text'
  | 'reasoning'
  | 'permission'
  | 'sidechain'
  | 'diff'
  | 'terminal'
  | 'error'
  | 'summary'
  | (string & {});

export type AgentTimelinePartStatus =
  | 'pending'
  | 'running'
  | 'waiting_permission'
  | 'completed'
  | 'error'
  | 'denied'
  | 'retrying'
  | 'aborted'
  | 'interrupted'
  | 'reused'
  | 'compacted'
  | (string & {});

export interface ChatAgentTimelinePart {
  partId: string;
  turnId: string;
  messageId: string;
  attemptId?: string | null;
  sequence: number;
  kind: AgentTimelinePartKind;
  callId?: string | null;
  tool?: string | null;
  gatewayTool?: string | null;
  title?: string | null;
  status?: AgentTimelinePartStatus | null;
  statusPresentation: StatusPresentation;
  summary?: string | null;
  detail?: string | null;
  text?: string | null;
  source?: string | null;
  input?: Record<string, unknown> | null;
  output?: Record<string, unknown> | null;
  metadata?: Record<string, unknown> | null;
  progress?: Record<string, unknown> | null;
  createdAt: string;
  updatedAt?: string | null;
  startedAt?: string | null;
  completedAt?: string | null;
  durationMs?: number | null;
}

export interface ToolRendererHint {
  renderer: string;
  detailView?: string | null;
}

export interface ToolApprovalRequest {
  requestId: string;
  sessionId: string;
  worktreeRoot: string | null;
  callId: string;
  permission: string;
  tool: string;
  gatewayTool: string;
  pattern: string;
  title: string;
  summary?: string | null;
  message: string;
  riskLevel: 'low' | 'medium' | 'high' | string;
  args: Record<string, unknown>;
}

export type ToolApprovalDecision =
  | 'allow_once'
  | 'allow_session'
  | 'allow_project'
  | 'deny_always';

export interface AgentTimelinePartStreamChunk {
  type: 'agentTimelinePart';
  part: ChatAgentTimelinePart;
}

export interface AgentTimelinePartDeltaStreamChunk {
  type: 'agentTimelinePartDelta';
  messageId: string;
  turnId: string;
  partId: string;
  field: 'text' | 'detail' | 'summary';
  delta: string;
}

export interface ToolApprovalStreamChunk {
  type: 'toolApproval';
  request: ToolApprovalRequest;
}

export interface ChatMessageStreamItem {
  id: string;
  sessionId: string;
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  attachments: ChatMessageAttachmentStreamItem[];
  tokenCount?: number | null;
  createdAt: string;
  agentTimelineParts: ChatAgentTimelinePart[];
}

export interface ChatMessageAttachmentStreamItem {
  kind: 'image' | 'file';
  name: string;
  mimeType?: string | null;
  sizeBytes?: number | null;
  dataBase64?: string | null;
  textContent?: string | null;
  isTruncated?: boolean | null;
  modelReadable?: boolean | null;
}

export type SessionMessageStreamChunk =
  | ChatMessagesStreamChunk
  | AgentTimelinePartStreamChunk
  | AgentTimelinePartDeltaStreamChunk
  | ToolApprovalStreamChunk;

// ── Extension 流 ─────────────────────────────────────────────

/** Extension 自定义流数据（泛型，由扩展定义 Schema） */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type ExtensionStreamChunk = Record<string, any>;
import type { StatusPresentation } from '../status';
