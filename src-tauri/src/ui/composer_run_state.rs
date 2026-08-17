// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

//! Session composer run state metadata projection.

use chrono::Utc;

// use [REMOVED: domains reference]

use super::dto::{UiComposerRunState, UiPendingPlanReview};
use super::session_metadata::{normalize_rfc3339_timestamp, ui_metadata};

pub(crate) const UI_COMPOSER_RUN_KEY: &str = "composerRun";

pub(crate) fn default_composer_run_state(session_id: &str) -> UiComposerRunState {
    UiComposerRunState {
        session_id: session_id.to_string(),
        plan_mode_enabled: false,
        plan_execution_started: false,
        multi_agent_enabled: false,
        pending_plan_review: None,
        goal_tracking_enabled: false,
        goal_paused: false,
        active_goal_text: None,
        active_goal_started_at: None,
        running_task: None,
        queued_tasks: Vec::new(),
    }
}

pub(crate) fn normalize_composer_run_state(mut state: UiComposerRunState) -> UiComposerRunState {
    state.pending_plan_review = state
        .pending_plan_review
        .and_then(normalize_pending_plan_review);
    state.active_goal_text = state
        .active_goal_text
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    state.active_goal_started_at = state
        .active_goal_started_at
        .as_deref()
        .and_then(normalize_rfc3339_timestamp);
    state.queued_tasks = Vec::new();
    state.running_task = None;

    if state.active_goal_text.is_some() && state.active_goal_started_at.is_none() {
        state.active_goal_started_at = Some(Utc::now().to_rfc3339());
    }

    if !state.goal_tracking_enabled {
        state.active_goal_text = None;
        state.active_goal_started_at = None;
        state.goal_paused = false;
    }

    if !state.plan_mode_enabled {
        state.plan_execution_started = false;
        state.pending_plan_review = None;
    }

    if state.pending_plan_review.is_some() {
        state.plan_execution_started = false;
    }

    state
}

pub(crate) fn session_composer_run_state(session: &Session) -> UiComposerRunState {
    let mut state = ui_metadata(session)
        .and_then(|ui| ui.get(UI_COMPOSER_RUN_KEY))
        .cloned()
        .and_then(|value| serde_json::from_value::<UiComposerRunState>(value).ok())
        .unwrap_or_else(|| default_composer_run_state(&session.id));
    state.session_id = session.id.clone();
    normalize_composer_run_state(state)
}

fn normalize_pending_plan_review(review: UiPendingPlanReview) -> Option<UiPendingPlanReview> {
    let request_text = review.request_text.trim();
    if request_text.is_empty() {
        return None;
    }

    Some(UiPendingPlanReview {
        id: review.id.trim().to_string(),
        request_text: request_text.to_string(),
        plan_content: review
            .plan_content
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty()),
        created_at: normalize_rfc3339_timestamp(review.created_at.trim())
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
    })
}
