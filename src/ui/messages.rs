use super::model::{BoundedId, BoundedText, Route, UiModel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiIntent {
    Quit,
    Navigate(Route),
    FocusNext,
    FocusPrevious,
    SelectNext,
    SelectPrevious,
    SelectFirst,
    SelectLast,
    SelectIndex(usize),
    OpenDetail,
    ReviewSelected,
    ToggleHelp,
    Back,
    Refresh,
    BeginSearch,
    SearchCharacter(char),
    SearchBackspace,
    AcceptSearch,
    ReviewAction(BoundedId),
    CancelJob,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEvent {
    Input(UiIntent),
    Resize {
        width: u16,
        height: u16,
    },
    SnapshotReady {
        generation: u64,
        model: UiModel,
    },
    SnapshotUnavailable {
        generation: u64,
        reason: BoundedText,
    },
    SnapshotCancelled {
        generation: u64,
        reason: BoundedText,
    },
    ActionReviewReady {
        action_id: BoundedId,
    },
    ActionReviewUnavailable {
        action_id: BoundedId,
        reason: BoundedText,
    },
    JobRunning {
        job_id: BoundedId,
        phase: BoundedText,
    },
    JobSucceeded {
        receipt: BoundedId,
        verification: BoundedId,
    },
    JobCancelled {
        job_id: BoundedId,
        reason: BoundedText,
    },
    RecoveryRequired {
        transaction: BoundedId,
        decision: BoundedText,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventTrace {
    events: Vec<UiEvent>,
}

impl EventTrace {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push(&mut self, event: UiEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[UiEvent] {
        &self.events
    }
}

impl Default for EventTrace {
    fn default() -> Self {
        Self::new()
    }
}
