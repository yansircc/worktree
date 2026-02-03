//! Executor module for Phases v2.
//!
//! Provides execution infrastructure for steps, workflows, and phase transitions.

pub mod context;
pub mod phase;
pub mod step;
pub mod workflow;

pub use context::ExecutionContext;
pub use phase::{next_phase, prev_phase, PhaseTransition, PhaseTransitionResult};
pub use step::StepExecutor;
pub use workflow::{WorkflowExecutor, WorkflowResult};
