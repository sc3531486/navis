pub mod request; pub mod response; pub mod protocol; pub mod provider;
pub mod multimodal; pub mod router; pub mod cost; pub mod middleware;
pub mod offline; pub mod quota;
pub use request::{ChatMessage, ChatRequest, ToolCall, ToolDefinition, GatewayConfig, ModelConfig, ProviderConfig, MessageRole, ApiProtocol, MessageContent};
pub use response::ChatResponse;
pub use protocol::{CapabilitySet, CustomProtocolConfig};
pub use multimodal::{ContentPart, FileContent, ImageContent, ImageMediaType, ImageSourceType, TextContent};
pub struct Gateway;
impl Gateway { pub fn new() -> Self { Self } }
pub struct CapabilityClipDiagnostic;
pub struct GatewayProviderStatus;
pub struct GatewayCapabilityCatalogProjection;
pub struct GatewayModelProjection;
pub struct GatewayProviderProjection;
pub struct ProtocolAdapterInfo;
