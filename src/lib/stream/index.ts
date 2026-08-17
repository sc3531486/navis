/**
 * 前端流基础设施统一导出
 * 来源：02b-stream.md §5.1
 */
export { useChannel } from './useChannel';
export type { UseChannelOptions, UseChannelReturn } from './useChannel';

export { runChannelStream } from './runChannelStream';

export { useEvent } from './useEvent';

export type {
  ChatAgentTimelinePart,
  AgentTimelinePartDeltaStreamChunk,
  AgentTimelinePartStreamChunk,
  ToolApprovalDecision,
  ToolApprovalRequest,
  ToolApprovalStreamChunk,
  ChatMessagesStreamChunk,
  SessionMessageStreamChunk,
  StandardStreamChunk,
  StreamCompletion,
  StreamRunController,
  StreamRunOptions,
  StreamTermination,
  ExtensionStreamChunk,
} from './types';
