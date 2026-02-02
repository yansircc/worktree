mod cli;
mod commands;
mod constants;
mod display;
mod error;
mod models;
mod services;
mod tui;

use clap::Parser;
use cli::{Cli, Commands, CompletionsAction, InternalCommands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => commands::init::execute(),
        Commands::Create { json } => commands::create::execute(json),
        Commands::Validate { name } => commands::validate::execute(name),
        Commands::List { tree, json } => commands::list::execute(tree, json),
        Commands::Start { name, all } => commands::start::execute(name, all),
        Commands::Review { name } => commands::review::execute(name),
        Commands::Resume { name } => commands::resume::execute(name),
        Commands::Merge { name, agent } => commands::merge::execute(name, agent),
        Commands::Delete { name } => commands::delete::execute(name, false),
        Commands::Next { json } => commands::next::execute(json),
        Commands::Reset { name } => commands::reset::execute(name),
        Commands::Status { json, action, task } => commands::status::execute(json, action, task),
        Commands::Tail { name, count } => commands::tail::execute(name, count),
        Commands::Logs => commands::logs::execute(),
        Commands::New { name, print_path } => commands::new::execute(name, print_path),
        Commands::Completions { action } => match action {
            CompletionsAction::Generate { shell } => {
                commands::completions::generate_completions(shell)
            }
            CompletionsAction::Install => commands::completions::install(),
        },
        Commands::Internal { command } => match command {
            InternalCommands::GitFetch { repo, remote } => {
                commands::internal::git::execute("fetch", vec![repo, remote])
            }
            InternalCommands::GitRebase { worktree, target } => {
                commands::internal::git::execute("rebase", vec![worktree, target])
            }
            InternalCommands::GitSquashMerge { repo, branch } => {
                commands::internal::git::execute("squash-merge", vec![repo, branch])
            }
            InternalCommands::GitCommit { path, message } => {
                commands::internal::git::execute("commit", vec![path, message])
            }
            InternalCommands::GitPush {
                repo,
                branch,
                remote,
            } => commands::internal::git::execute("push", vec![repo, branch, remote]),
            InternalCommands::GitHasChanges { path } => {
                commands::internal::git::execute("has-changes", vec![path])
            }
            InternalCommands::GitHasConflicts { path } => {
                commands::internal::git::execute("has-conflicts", vec![path])
            }
            InternalCommands::GitStash { path } => {
                commands::internal::git::execute("stash", vec![path])
            }
            InternalCommands::GitStashPop { path } => {
                commands::internal::git::execute("stash-pop", vec![path])
            }
            InternalCommands::GitCreateBranch { repo, branch } => {
                commands::internal::git::execute("create-branch", vec![repo, branch])
            }
            InternalCommands::GitDeleteBranch { repo, branch } => {
                commands::internal::git::execute("delete-branch", vec![repo, branch])
            }
            InternalCommands::GitCheckout { path, branch } => {
                commands::internal::git::execute("checkout", vec![path, branch])
            }
            InternalCommands::GitCurrentBranch { path } => {
                commands::internal::git::execute("current-branch", vec![path])
            }
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
