//! CLI End-to-End Tests

#[path = "common.rs"]
mod common;

use common::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "cli/init.rs"]
mod init;
#[path = "cli/create.rs"]
mod create;
#[path = "cli/validate.rs"]
mod validate;
#[path = "cli/list.rs"]
mod list;
#[path = "cli/next.rs"]
mod next;
#[path = "cli/start.rs"]
mod start;
#[path = "cli/done.rs"]
mod done;
#[path = "cli/merged.rs"]
mod merged;
#[path = "cli/help.rs"]
mod help;
#[path = "cli/no_config.rs"]
mod no_config;
#[path = "cli/reset.rs"]
mod reset;
#[path = "cli/status.rs"]
mod status;
#[path = "cli/new.rs"]
mod new;
#[path = "cli/archive.rs"]
mod archive;
#[path = "cli/scratch.rs"]
mod scratch;
#[path = "cli/tail.rs"]
mod tail;
#[path = "cli/logs.rs"]
mod logs;
#[path = "cli/completions.rs"]
mod completions;
