//! Thread-safe observer wrapper for parallel execution.
//!
//! Provides synchronized access to observers during parallel step execution.

use std::sync::{Arc, Mutex};

use crate::models::step::StepResult;

use super::log::LogObserver;
use super::terminal::TerminalObserver;

/// Thread-safe wrapper for observers.
///
/// Wraps TerminalObserver (immutable, no sync needed) and LogObserver (needs mutex)
/// to allow safe concurrent notifications from parallel step execution.
pub struct SyncObservers {
    /// Terminal observer (immutable, thread-safe via eprintln)
    terminal: Option<TerminalObserver>,
    /// Log observer wrapped in mutex for synchronized writes
    log: Arc<Mutex<Option<LogObserver>>>,
}

impl SyncObservers {
    /// Create a new synchronized observer wrapper.
    pub fn new(terminal: Option<TerminalObserver>, log: Option<LogObserver>) -> Self {
        Self {
            terminal,
            log: Arc::new(Mutex::new(log)),
        }
    }

    /// Notify observers that a step has started.
    ///
    /// Thread-safe: terminal writes to stderr (atomic), log uses mutex.
    pub fn on_step_start(&self, index: usize, step_id: Option<&str>, step_name: &str) {
        // Terminal observer: eprintln is thread-safe
        if let Some(ref obs) = self.terminal {
            obs.on_step_start(index, step_name);
        }

        // Log observer: needs mutex
        if let Ok(mut guard) = self.log.lock() {
            if let Some(ref mut obs) = *guard {
                let _ = obs.on_step_start(index, step_id);
            }
        }
    }

    /// Notify observers that a step has completed.
    ///
    /// Thread-safe: terminal writes to stderr (atomic), log uses mutex.
    pub fn on_step_complete(&self, index: usize, step_name: &str, result: &StepResult) {
        // Terminal observer
        if let Some(ref obs) = self.terminal {
            obs.on_step_complete(index, step_name, &result.state);
        }

        // Log observer
        if let Ok(mut guard) = self.log.lock() {
            if let Some(ref mut obs) = *guard {
                let _ = obs.on_step_complete(result);
            }
        }
    }

    /// Extract the log observer for final workflow context saving.
    ///
    /// Consumes the wrapper and returns the inner LogObserver.
    pub fn into_log_observer(self) -> Option<LogObserver> {
        Arc::try_unwrap(self.log)
            .ok()
            .and_then(|mutex| mutex.into_inner().ok())
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::step::StepState;
    use crate::services::observer::terminal::TerminalSettings;

    #[test]
    fn test_sync_observers_creation() {
        let terminal = TerminalObserver::new(TerminalSettings::default());
        let observers = SyncObservers::new(Some(terminal), None);
        assert!(observers.terminal.is_some());
    }

    #[test]
    fn test_sync_observers_no_panic_on_notify() {
        let observers = SyncObservers::new(None, None);

        // Should not panic even with no observers
        observers.on_step_start(0, Some("test"), "test-step");
        observers.on_step_complete(
            0,
            "test-step",
            &StepResult {
                state: StepState::Success,
                ..Default::default()
            },
        );
    }

    #[test]
    fn test_sync_observers_into_log_observer() {
        let observers = SyncObservers::new(None, None);
        let log = observers.into_log_observer();
        assert!(log.is_none());
    }
}
