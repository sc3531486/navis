//! 跨领域状态语义协议。
//!
//! 本模块只定义稳定的语义投影，不拥有任何业务领域状态，也不依赖 UI。
//! 具体领域通过 `StatusClassify` 将自己的事实状态投影到这些语义类型。

mod classification;

pub use classification::{
    StatusAttention, StatusClass, StatusClassify, StatusOutcome, StatusPhase, StatusPresentation,
};
