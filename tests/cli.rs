//! CLI End-to-End Tests

#[path = "common.rs"]
mod common;

use common::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "cli/completions.rs"]
mod completions;
#[path = "cli/create.rs"]
mod create;
#[path = "cli/delete.rs"]
mod delete;
#[path = "cli/help.rs"]
mod help;
#[path = "cli/init.rs"]
mod init;
#[path = "cli/list.rs"]
mod list;
#[path = "cli/logs.rs"]
mod logs;
#[path = "cli/merge.rs"]
mod merge;
#[path = "cli/next.rs"]
mod next;
#[path = "cli/no_config.rs"]
mod no_config;
#[path = "cli/reset.rs"]
mod reset;
#[path = "cli/resume.rs"]
mod resume;
#[path = "cli/review.rs"]
mod review;
#[path = "cli/scratch.rs"]
mod scratch;
#[path = "cli/status.rs"]
mod status;
#[path = "cli/tail.rs"]
mod tail;
#[path = "cli/validate.rs"]
mod validate;
