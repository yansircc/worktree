//! Workflow execution strategies.
//!
//! This module provides different execution strategies for workflows:
//! - Sequential: steps run one after another
//! - Parallel: all steps run concurrently
//! - DAG: steps run in batches based on dependency order

pub mod dag;
pub mod parallel;
pub mod sequential;

pub use dag::execute_dag;
pub use parallel::execute_parallel;
pub use sequential::execute_sequential;
