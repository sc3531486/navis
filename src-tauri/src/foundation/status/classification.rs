use serde::{Deserialize, Serialize};

/// 生命周期阶段。它描述“现在处于什么阶段”，不描述最终结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusPhase {
    Queued,
    Active,
    Waiting,
    Inactive,
    Unknown,
}

/// 执行结果。未结束的状态使用 `None`，未知结果不能被当成成功。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
    Unknown,
}

/// 是否需要用户注意或操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusAttention {
    Normal,
    NeedsAction,
    Warning,
}

/// 领域状态的统一语义投影。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPresentation {
    pub phase: StatusPhase,
    pub outcome: Option<StatusOutcome>,
    pub attention: StatusAttention,
    pub live: bool,
    pub terminal: bool,
}

impl StatusPresentation {
    pub const fn queued() -> Self {
        Self::new(StatusPhase::Queued, None, StatusAttention::Normal)
    }

    pub const fn active() -> Self {
        Self::new(StatusPhase::Active, None, StatusAttention::Normal)
    }

    pub const fn waiting() -> Self {
        Self::new(StatusPhase::Waiting, None, StatusAttention::NeedsAction)
    }

    pub const fn inactive() -> Self {
        Self::new(StatusPhase::Inactive, None, StatusAttention::Normal)
    }

    pub const fn succeeded() -> Self {
        Self::new(
            StatusPhase::Inactive,
            Some(StatusOutcome::Succeeded),
            StatusAttention::Normal,
        )
    }

    pub const fn failed() -> Self {
        Self::new(
            StatusPhase::Inactive,
            Some(StatusOutcome::Failed),
            StatusAttention::Warning,
        )
    }

    pub const fn cancelled() -> Self {
        Self::new(
            StatusPhase::Inactive,
            Some(StatusOutcome::Cancelled),
            StatusAttention::Normal,
        )
    }

    pub const fn unknown() -> Self {
        Self::new(
            StatusPhase::Unknown,
            Some(StatusOutcome::Unknown),
            StatusAttention::Warning,
        )
    }

    pub const fn new(
        phase: StatusPhase,
        outcome: Option<StatusOutcome>,
        attention: StatusAttention,
    ) -> Self {
        Self {
            phase,
            outcome,
            attention,
            live: matches!(
                phase,
                StatusPhase::Queued | StatusPhase::Active | StatusPhase::Waiting
            ),
            terminal: outcome.is_some(),
        }
    }
}

/// 将领域事实状态投影为公共语义。
pub trait StatusClassify {
    fn status_presentation(&self) -> StatusPresentation;
}

/// 适合简单 DTO 或外部输入的稳定状态类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusClass {
    Queued,
    Active,
    Waiting,
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
    Inactive,
    Unknown,
}

impl StatusClass {
    pub const fn presentation(self) -> StatusPresentation {
        match self {
            Self::Queued => StatusPresentation::queued(),
            Self::Active => StatusPresentation::active(),
            Self::Waiting => StatusPresentation::waiting(),
            Self::Succeeded => StatusPresentation::succeeded(),
            Self::Failed => StatusPresentation::failed(),
            Self::Cancelled => StatusPresentation::cancelled(),
            Self::Skipped => StatusPresentation::new(
                StatusPhase::Inactive,
                Some(StatusOutcome::Skipped),
                StatusAttention::Normal,
            ),
            Self::Inactive => StatusPresentation::inactive(),
            Self::Unknown => StatusPresentation::unknown(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_outcomes_are_not_live() {
        for presentation in [
            StatusPresentation::succeeded(),
            StatusPresentation::failed(),
            StatusPresentation::cancelled(),
        ] {
            assert!(!presentation.live);
            assert!(presentation.terminal);
        }
    }

    #[test]
    fn unknown_is_safe_and_not_successful() {
        let presentation = StatusPresentation::unknown();
        assert_eq!(presentation.phase, StatusPhase::Unknown);
        assert_eq!(presentation.outcome, Some(StatusOutcome::Unknown));
        assert!(!presentation.live);
        assert!(presentation.terminal);
    }
}
