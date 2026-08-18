//! Agent 单轮上下文构建。
//!
//! 这里把 Code / Cowork / Custom mode 转换成同一条 Agent 执行链路的
//! system context。它不创建新的用户侧概念，只复用 ModeConfig。

