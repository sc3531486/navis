//! 模型路由模块（Kernel-backed facade）
//!
//! 本模块是 Gateway 领域路由层的 facade；Provider 配置本体由
//! Kernel `InMemoryRegistry<ProviderConfig>` 持有，本模块只维护
//! Gateway 领域的 `provider/model` 路由索引和查询接口。
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

