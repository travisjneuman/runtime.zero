use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CancellationReason {
    UserRequested = 1,
    DeadlineExceeded = 2,
    ParentCancelled = 3,
    HostShutdown = 4,
}

impl CancellationReason {
    pub const fn foundation_code(self) -> rz0_error_contract::FoundationErrorCode {
        match self {
            Self::DeadlineExceeded => rz0_error_contract::FoundationErrorCode::TimedOut,
            Self::UserRequested | Self::ParentCancelled | Self::HostShutdown => {
                rz0_error_contract::FoundationErrorCode::Cancelled
            }
        }
    }

    const fn from_byte(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::UserRequested),
            2 => Some(Self::DeadlineExceeded),
            3 => Some(Self::ParentCancelled),
            4 => Some(Self::HostShutdown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelOutcome {
    Won(CancellationReason),
    AlreadyCancelled(CancellationReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessDeadline {
    started_tick_ms: u64,
    deadline_tick_ms: u64,
}

impl ProcessDeadline {
    pub fn new(
        started_tick_ms: u64,
        timeout_ms: u64,
        maximum_timeout_ms: u64,
    ) -> Result<Self, DeadlineError> {
        if timeout_ms == 0 || timeout_ms > maximum_timeout_ms {
            return Err(DeadlineError::InvalidTimeout);
        }
        let deadline_tick_ms = started_tick_ms
            .checked_add(timeout_ms)
            .ok_or(DeadlineError::Overflow)?;
        Ok(Self {
            started_tick_ms,
            deadline_tick_ms,
        })
    }

    pub const fn started_tick_ms(self) -> u64 {
        self.started_tick_ms
    }

    pub const fn deadline_tick_ms(self) -> u64 {
        self.deadline_tick_ms
    }

    pub const fn expired(self, now_tick_ms: u64) -> bool {
        now_tick_ms >= self.deadline_tick_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineError {
    InvalidTimeout,
    Overflow,
}

#[derive(Debug, Clone)]
pub struct CancellationController {
    state: Arc<AtomicU8>,
}

#[derive(Debug, Clone)]
pub struct CancellationToken {
    state: Arc<AtomicU8>,
}

pub fn cancellation_pair() -> (CancellationController, CancellationToken) {
    let state = Arc::new(AtomicU8::new(0));
    (
        CancellationController {
            state: Arc::clone(&state),
        },
        CancellationToken { state },
    )
}

impl CancellationController {
    pub fn cancel(&self, reason: CancellationReason) -> CancelOutcome {
        cancel_state(&self.state, reason)
    }

    pub fn reason(&self) -> Option<CancellationReason> {
        load_reason(&self.state)
    }
}

impl CancellationToken {
    pub fn reason(&self) -> Option<CancellationReason> {
        load_reason(&self.state)
    }

    /// Returns the first cancellation reason. If the monotonic deadline has
    /// elapsed, this poll atomically records deadline cancellation unless an
    /// earlier reason already won.
    pub fn poll(&self, now_tick_ms: u64, deadline: ProcessDeadline) -> Option<CancellationReason> {
        if let Some(reason) = self.reason() {
            return Some(reason);
        }
        if deadline.expired(now_tick_ms) {
            let _ = cancel_state(&self.state, CancellationReason::DeadlineExceeded);
            self.reason()
        } else {
            None
        }
    }
}

fn cancel_state(state: &AtomicU8, reason: CancellationReason) -> CancelOutcome {
    match state.compare_exchange(0, reason as u8, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => CancelOutcome::Won(reason),
        Err(existing) => CancelOutcome::AlreadyCancelled(
            CancellationReason::from_byte(existing)
                .expect("cancellation state contains only internal enum values"),
        ),
    }
}

fn load_reason(state: &AtomicU8) -> Option<CancellationReason> {
    CancellationReason::from_byte(state.load(Ordering::Acquire))
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn first_cancellation_reason_is_immutable_and_typed() {
        let (controller, token) = cancellation_pair();
        assert_eq!(token.reason(), None);
        assert_eq!(
            controller.cancel(CancellationReason::UserRequested),
            CancelOutcome::Won(CancellationReason::UserRequested)
        );
        assert_eq!(
            controller.cancel(CancellationReason::HostShutdown),
            CancelOutcome::AlreadyCancelled(CancellationReason::UserRequested)
        );
        assert_eq!(token.reason(), Some(CancellationReason::UserRequested));
        assert_eq!(
            token.reason().unwrap().foundation_code(),
            rz0_error_contract::FoundationErrorCode::Cancelled
        );
    }

    #[test]
    fn monotonic_deadline_is_bounded_overflow_safe_and_sticky() {
        assert_eq!(
            ProcessDeadline::new(1, 0, 10),
            Err(DeadlineError::InvalidTimeout)
        );
        assert_eq!(
            ProcessDeadline::new(u64::MAX, 1, 10),
            Err(DeadlineError::Overflow)
        );
        let deadline = ProcessDeadline::new(1_000, 10, 10).unwrap();
        let (_, token) = cancellation_pair();
        assert_eq!(token.poll(1_009, deadline), None);
        assert_eq!(
            token.poll(1_010, deadline),
            Some(CancellationReason::DeadlineExceeded)
        );
        assert_eq!(
            token.poll(900, deadline),
            Some(CancellationReason::DeadlineExceeded)
        );
    }

    #[test]
    fn cloned_controllers_converge_on_exactly_one_winner() {
        let (controller, token) = cancellation_pair();
        let reasons = [
            CancellationReason::UserRequested,
            CancellationReason::ParentCancelled,
            CancellationReason::HostShutdown,
        ];
        let handles = reasons.map(|reason| {
            let controller = controller.clone();
            thread::spawn(move || controller.cancel(reason))
        });
        let outcomes = handles.map(|handle| handle.join().unwrap());
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, CancelOutcome::Won(_)))
                .count(),
            1
        );
        let winner = token.reason().expect("one winner");
        assert!(outcomes.contains(&CancelOutcome::Won(winner)));
    }
}
