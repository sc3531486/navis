//! 应用状态持久化
//!
//! 基于设计文档 §3.3 实现，管理应用状态的持久化
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

use serde::{Deserialize, Serialize};

/// 窗口状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowState {
    /// 窗口 X 坐标
    pub x: i32,
    /// 窗口 Y 坐标
    pub y: i32,
    /// 窗口宽度
    pub width: u32,
    /// 窗口高度
    pub height: u32,
    /// 是否最大化
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1200,
            height: 800,
            maximized: false,
        }
    }
}

/// 应用状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    /// 窗口状态
    pub window_state: WindowState,
    /// 上次活跃会话 ID
    pub last_active_session: Option<String>,
    /// 上次打开的worktree 路径
    pub last_worktree: Option<String>,
    /// 是否首次启动
    pub first_launch: bool,
    /// 应用版本
    pub app_version: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            window_state: WindowState::default(),
            last_active_session: None,
            last_worktree: None,
            first_launch: true,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl AppState {
    /// 创建新的应用状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新窗口状态
    pub fn update_window_state(&mut self, window_state: WindowState) {
        tracing::debug!(
            x = window_state.x,
            y = window_state.y,
            width = window_state.width,
            height = window_state.height,
            maximized = window_state.maximized,
            "Updating window state"
        );

        self.window_state = window_state;
    }

    /// 设置上次活跃会话
    pub fn set_last_active_session(&mut self, session_id: Option<String>) {
        tracing::debug!(session_id = ?session_id, "Setting last active session");

        self.last_active_session = session_id;
    }

    /// 设置上次 worktree
    pub fn set_last_worktree(&mut self, worktree: Option<String>) {
        tracing::debug!(worktree = ?worktree, "Setting last worktree");

        self.last_worktree = worktree;
    }

    /// 标记为非首次启动
    pub fn mark_launched(&mut self) {
        tracing::debug!("Marking as launched");

        self.first_launch = false;
    }

    /// 序列化为 JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            tracing::error!(error = %e, "Failed to serialize app state");
            "{}".to_string()
        })
    }

    /// 从 JSON 反序列化
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to deserialize app state, using default");
            Self::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_state_default() {
        let state = WindowState::default();

        assert_eq!(state.x, 100);
        assert_eq!(state.y, 100);
        assert_eq!(state.width, 1200);
        assert_eq!(state.height, 800);
        assert!(!state.maximized);
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();

        assert!(state.first_launch);
        assert!(state.last_active_session.is_none());
        assert!(state.last_worktree.is_none());
    }

    #[test]
    fn test_app_state_update_window_state() {
        let mut state = AppState::new();

        let window_state = WindowState {
            x: 200,
            y: 200,
            width: 1920,
            height: 1080,
            maximized: true,
        };

        state.update_window_state(window_state.clone());

        assert_eq!(state.window_state, window_state);
    }

    #[test]
    fn test_app_state_set_last_active_session() {
        let mut state = AppState::new();

        state.set_last_active_session(Some("sess_001".to_string()));
        assert_eq!(state.last_active_session, Some("sess_001".to_string()));

        state.set_last_active_session(None);
        assert!(state.last_active_session.is_none());
    }

    #[test]
    fn test_app_state_set_last_worktree() {
        let mut state = AppState::new();

        state.set_last_worktree(Some("/path/to/worktree".to_string()));
        assert_eq!(state.last_worktree, Some("/path/to/worktree".to_string()));
    }

    #[test]
    fn test_app_state_mark_launched() {
        let mut state = AppState::new();

        assert!(state.first_launch);

        state.mark_launched();

        assert!(!state.first_launch);
    }

    #[test]
    fn test_app_state_serialization() {
        let mut state = AppState::new();
        state.set_last_active_session(Some("sess_001".to_string()));
        state.set_last_worktree(Some("/path/to/worktree".to_string()));

        // 序列化
        let json = state.to_json();
        assert!(json.contains("sess_001"));
        assert!(json.contains("/path/to/worktree"));

        // 反序列化
        let loaded = AppState::from_json(&json);
        assert_eq!(loaded.last_active_session, Some("sess_001".to_string()));
        assert_eq!(loaded.last_worktree, Some("/path/to/worktree".to_string()));
    }

    #[test]
    fn test_app_state_from_invalid_json() {
        let state = AppState::from_json("invalid json");
        assert!(state.first_launch);
    }
}
