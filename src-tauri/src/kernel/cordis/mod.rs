//! Cordis 风格的扩展系统核心
//! 参考 DeepSeek Harness Cordis，通用框架，不绑定业务领域。

pub mod context;
pub mod fiber;
pub mod scope;
pub mod service;

pub use context::CordisContext;
pub use fiber::{Fiber, FiberManager, FiberState};
pub use scope::{NamedEntries, ScopedLayers};
pub use service::Service;
