//! 交互式 PTY 会话抽象。
//!
//! 本模块负责操作系统 PTY 资源，以及终端域内的 PTY 生命周期收口：
//! - 创建 shell、读写、调整窗口尺寸和终止进程
//! - 注册/移除活跃 PTY session
//! - 维护输出桥接，但不把 Tauri Channel 长期放进共享状态
//! TerminalManager 只负责实例索引、事件和业务协调。

