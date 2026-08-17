//! 终端 PTY 输出流处理。
//!
//! 复用 foundation::stream::StreamChannel 的节流与标准 envelope，
//! 但不把 `tauri::ipc::Channel` 长期存入共享状态。共享状态只保留
//! `std::sync::mpsc::Sender`，真正的 Tauri Channel 仅由独立转发线程持有。

