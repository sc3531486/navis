//! LSP 语言能力目录（Kernel-backed facade）
//!
//! 本模块是 LSP 领域查询层的 facade；能力登记、注销和生命周期
//! 委托给 `kernel::InMemoryRegistry`，不维护平行 HashMap。
//!
//! 基于设计文档 §5 实现，管理语言 → LSP Server 配置映射。
//! 支持内置语言和扩展注册的自定义语言。
//!
//! # 优先级规则
//! - 内置语言优先级最高，扩展不能覆盖
//! - 多个扩展注册同一 languageId 时，先安装的优先
//!
//! # 架构说明
//! `LanguageRegistry` 仅持有领域查询方法（按扩展名、语言名查找）；
//! 实际 `Capability` 生命周期（register / unregister / enable / disable）
//! 全部由底层 `InMemoryRegistry<LanguageCapability>` 承载。

