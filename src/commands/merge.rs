//! Merge command - DEPRECATED, use `wt complete` instead.
//!
//! This is kept for backward compatibility. All functionality has been
//! moved to the `complete` command.

use crate::error::Result;

pub fn execute(task_ref: String) -> Result<()> {
    eprintln!("Warning: 'wt merge' is deprecated. Use 'wt complete' instead.");
    super::complete::execute(task_ref)
}
