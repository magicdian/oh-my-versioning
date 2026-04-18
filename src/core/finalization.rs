use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeType {
    Bugfix,
    Feature,
    Refactor,
    Docs,
    Chore,
}

impl ChangeType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bugfix => "bugfix",
            Self::Feature => "feature",
            Self::Refactor => "refactor",
            Self::Docs => "docs",
            Self::Chore => "chore",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "bugfix" => Some(Self::Bugfix),
            "feature" => Some(Self::Feature),
            "refactor" => Some(Self::Refactor),
            "docs" => Some(Self::Docs),
            "chore" => Some(Self::Chore),
            _ => None,
        }
    }

    pub fn bumps_version(self) -> bool {
        matches!(self, Self::Bugfix | Self::Feature)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Done,
    InProgress,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::InProgress => "in-progress",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "done" => Some(Self::Done),
            "in-progress" => Some(Self::InProgress),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TestsStatus {
    Passed,
    Failed,
}

impl TestsStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "passed" => Some(Self::Passed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FinalizationOutcome {
    Pending,
    Bumped,
    NoOp,
}

impl FinalizationOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Bumped => "bumped",
            Self::NoOp => "noop",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "pending" => Some(Self::Pending),
            "bumped" => Some(Self::Bumped),
            "noop" => Some(Self::NoOp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FinalizationReason {
    SemanticChange,
    TestsNotPassed,
    StatusNotDone,
    NonSemanticChange,
    DuplicateFingerprint,
    PendingRecovered,
}

impl FinalizationReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SemanticChange => "semantic-change",
            Self::TestsNotPassed => "tests-not-passed",
            Self::StatusNotDone => "status-not-done",
            Self::NonSemanticChange => "non-semantic-change",
            Self::DuplicateFingerprint => "duplicate-fingerprint",
            Self::PendingRecovered => "pending-recovered",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "semantic-change" => Some(Self::SemanticChange),
            "tests-not-passed" => Some(Self::TestsNotPassed),
            "status-not-done" => Some(Self::StatusNotDone),
            "non-semantic-change" => Some(Self::NonSemanticChange),
            "duplicate-fingerprint" => Some(Self::DuplicateFingerprint),
            "pending-recovered" => Some(Self::PendingRecovered),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalizationDecision {
    pub outcome: FinalizationOutcome,
    pub reason: FinalizationReason,
}

impl FinalizationDecision {
    pub fn should_bump(self) -> bool {
        self.outcome == FinalizationOutcome::Bumped
    }
}

pub fn decide(
    change_type: ChangeType,
    task_status: TaskStatus,
    tests_status: TestsStatus,
) -> FinalizationDecision {
    if tests_status != TestsStatus::Passed {
        return FinalizationDecision {
            outcome: FinalizationOutcome::NoOp,
            reason: FinalizationReason::TestsNotPassed,
        };
    }

    if task_status != TaskStatus::Done {
        return FinalizationDecision {
            outcome: FinalizationOutcome::NoOp,
            reason: FinalizationReason::StatusNotDone,
        };
    }

    if !change_type.bumps_version() {
        return FinalizationDecision {
            outcome: FinalizationOutcome::NoOp,
            reason: FinalizationReason::NonSemanticChange,
        };
    }

    FinalizationDecision {
        outcome: FinalizationOutcome::Bumped,
        reason: FinalizationReason::SemanticChange,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChangeType, FinalizationOutcome, FinalizationReason, TaskStatus, TestsStatus, decide,
    };

    #[test]
    fn semantic_change_with_done_status_and_passing_tests_bumps() {
        let decision = decide(ChangeType::Bugfix, TaskStatus::Done, TestsStatus::Passed);
        assert_eq!(decision.outcome, FinalizationOutcome::Bumped);
        assert_eq!(decision.reason, FinalizationReason::SemanticChange);
    }

    #[test]
    fn failed_tests_force_noop() {
        let decision = decide(ChangeType::Feature, TaskStatus::Done, TestsStatus::Failed);
        assert_eq!(decision.outcome, FinalizationOutcome::NoOp);
        assert_eq!(decision.reason, FinalizationReason::TestsNotPassed);
    }

    #[test]
    fn in_progress_status_forces_noop() {
        let decision = decide(
            ChangeType::Feature,
            TaskStatus::InProgress,
            TestsStatus::Passed,
        );
        assert_eq!(decision.outcome, FinalizationOutcome::NoOp);
        assert_eq!(decision.reason, FinalizationReason::StatusNotDone);
    }

    #[test]
    fn non_semantic_change_forces_noop() {
        let decision = decide(ChangeType::Docs, TaskStatus::Done, TestsStatus::Passed);
        assert_eq!(decision.outcome, FinalizationOutcome::NoOp);
        assert_eq!(decision.reason, FinalizationReason::NonSemanticChange);
    }
}
