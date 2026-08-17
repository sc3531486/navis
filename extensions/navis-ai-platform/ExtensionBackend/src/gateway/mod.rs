//! Gateway 模型网关模块（扩展点）
//!
//! 从 src-tauri/src/ai/gateway/ 迁入，提供统一的模型调用入口，
//! 屏蔽不同 LLM Provider 的差异，提供统一的请求/响应接口、
//! 流式处理、Quota 计量、离线降级、成本统计。
//!
//! # 迁移来源
//!
//! src-tauri/src/ai/gateway/ 下所有文件：
//!   cost.rs, middleware.rs, mod.rs, multimodal.rs, offline.rs,
//!   quota.rs, request.rs, response.rs, router.rs
//!   protocol/ (adapter_trait, capability, chat_completions, custom, mod.rs, registry, responses, transformer)
//!   provider/ (mod.rs, profile.rs)
//!
//! # re-export 桥
//!
//! Phase 0 阶段，所有类型从原始模块重导出。

pub use crate::ai::gateway::cost;
pub use crate::ai::gateway::middleware;
pub use crate::ai::gateway::multimodal;
pub use crate::ai::gateway::offline;
pub use crate::ai::gateway::protocol;
pub use crate::ai::gateway::provider;
pub use crate::ai::gateway::quota;
pub use crate::ai::gateway::request;
pub use crate::ai::gateway::response;
pub use crate::ai::gateway::router;

// 重导出核心类型（保持原 gateway/mod.rs 的公开 API）
pub use crate::ai::gateway::{
    CostStats, CostTracker,
};
pub use crate::ai::gateway::{
    ErrorAction, GatewayError, GatewayMiddleware, GatewayMiddlewareData, GatewayPipelineConfig,
    MiddlewarePhase, PrefixStabilizerMiddleware, RateLimiterMiddleware, TokenCounterMiddleware,
};
pub use crate::ai::gateway::{
    ContentPart, FileContent, ImageContent, ImageMediaType, ImageProcessingConfig, ImageSourceType,
    ImageValidationResult, TextContent,
};
pub use crate::ai::gateway::{NetworkStatus, OfflineConfig, OfflineDetector};
pub use crate::ai::gateway::{
    CapabilityClipDiagnostic, CapabilityEvaluationInput, CapabilitySet,
    GatewayCapabilityEvaluatorPort, GatewayCapabilityPolicies, GatewayCapabilityProjection,
    IntersectionCapabilityEvaluator, ModelIdentity, ProviderIdentity,
    GATEWAY_CAPABILITY_PROJECTION_VERSION,
};
pub use crate::ai::gateway::{
    ProtocolAdapterInfo, ProtocolAdapterRegistry, ProviderAdapter, StreamFrame, StreamFrameDecoder,
};
pub use crate::ai::gateway::{QuotaConstraint, QuotaInfo, QuotaManager, QuotaPolicyInput};
pub use crate::ai::gateway::{
    ApiProtocol, ChatMessage, ChatRequest, FunctionCall, FunctionCallDelta, FunctionDefinition,
    GatewayConfig, MessageContent, MessageRole, ModelConfig, ProviderConfig, ToolCall,
    ToolCallDelta, ToolDefinition,
};
pub use crate::ai::gateway::{
    ChatResponse, OutputItem, ReasoningSummary, SearchResult, StreamChunk, TokenUsage,
};
pub use crate::ai::gateway::{ModelRouter, RouteResult};
pub use crate::ai::gateway::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use crate::ai::gateway::{StreamError, StreamReceiver, StreamSender};
pub use crate::ai::gateway::{CustomProtocolConfig};
