//! Archive command (deprecated alias for delete).

use crate::error::Result;

/// Execute the archive command.
///
/// This is a deprecated alias for `wt delete`. Users should migrate to using
/// `wt delete` directly.
pub fn execute(task_ref: String) -> Result<()> {
    eprintln!("Warning: 'wt archive' is deprecated, use 'wt delete' instead.");
    super::delete::execute(task_ref, false)
}
