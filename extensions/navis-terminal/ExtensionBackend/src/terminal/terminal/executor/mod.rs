//! 命令执行器
//!
//! 基于设计文档 §terminal 实现，提供命令执行能力：
//! - 同步命令执行（独立进程，捕获输出）
//! - 命令安全校验（通过 Kernel Policy 中的 Sandbox constraint）
//! - 超时控制
//!
//! PolicyEngine 通过构造函数注入，不使用全局静态。

