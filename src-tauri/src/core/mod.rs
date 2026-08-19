// 通用运行时外壳核心模块：进程管理器、多路复用 IPC 网关、动态沙箱。
// 该层不包含任何业务逻辑，仅提供扩展所需的通用机制。
pub mod ipc_bridge;
pub mod process_supervisor;
pub mod sandbox;