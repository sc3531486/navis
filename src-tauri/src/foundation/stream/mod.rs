//! Stream 流式数据通道模块（统一入口）
//!
//! Navis 所有流式数据通信的唯一出口。
//! 其他模块（Terminal/Agent/Gateway/Extension）统一通过此模块接入流。
//!
//! # 三种使用场景
//!
//! ```ignore
//! // 场景 1：后端进程内（Gateway → Agent）
//! let (sender, receiver) = stream::mpsc_pair("gateway", 64);
//! sender.send_delta("Hello").await;
//! let chunk = receiver.recv().await;
//!
//! // 场景 2：跨进程推送（Agent/Terminal → 前端）
//! let channel = StreamChannel::builder(channel)
//!     .source(StreamSource::new("agent", "sess_001"))
//!     .label("LLM streaming")
//!     .throttle(Duration::from_millis(50))
//!     .build();
//!
//! // 场景 3：扩展自定义流
//! let extension_stream = StreamChannel::builder(channel)
//!     .source(StreamSource::new("jira-live-feed", "issue-123"))
//!     .build();
//! ```
//!
//! # 设计原则
//! - EventBus = "发生了什么事"（离散状态变更）
//! - Stream   = "这是数据的一段"（连续数据流）
//!
//! 详见 design/02b-stream.md

pub mod channel;
pub mod emitter;
pub mod index;
pub mod sender;
pub mod types;

// 核心类型重导出
pub use types::stream_kind;
pub use types::{
    StreamCancelToken, StreamChunk, StreamChunkKind, StreamError, StreamId, StreamSource,
};

// 进程内流式通道
pub use sender::{mpsc_pair, StreamReceiver, StreamSender};

// 跨进程流式推送
pub use channel::{
    forward_receiver_to_channel, forward_receiver_to_channel_with_index, send_channel_value,
    StreamChannel, StreamChannelBuilder,
};
pub use emitter::{BufferedItem, ThrottledEmitter, DEFAULT_THROTTLE_INTERVAL};

// 流活动索引（不是 Kernel Registry）
pub use index::{StreamIndex, StreamInfo, StreamSubscriptionFilter};
