//! 终端管理 — PTY、Shell、历史、流式输出
//! 迁移过渡期：re-export src-tauri/src/tool/terminal

pub use crate::tool::terminal::env;
pub use crate::tool::terminal::executor;
pub use crate::tool::terminal::history;
pub use crate::tool::terminal::manager;
pub use crate::tool::terminal::pty;
pub use crate::tool::terminal::shell;
pub use crate::tool::terminal::stream;
