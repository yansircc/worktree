mod cli;
mod commands;
mod constants;
mod display;
mod error;
mod models;
mod services;
mod tui;

use clap::Parser;
use cli::{Cli, Commands, CompletionsAction, StepAction};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => commands::init::execute(),
        Commands::Create { json } => commands::create::execute(json),
        Commands::Validate { name } => commands::validate::execute(name),
        Commands::List { tree, json } => commands::list::execute(tree, json),
        Commands::Delete { name, force } => commands::delete::execute(name, force),
        Commands::Next { task } => commands::next::execute(task),
        Commands::Reset { name, to } => commands::reset::execute(name, to),
        Commands::Status { json, verbose, all, action, task } => commands::status::execute(json, verbose, all, action, task),
        Commands::Tail { name, count } => commands::tail::execute(name, count),
        Commands::Logs => commands::logs::execute(),
        Commands::New { name, print_path } => commands::new::execute(name, print_path),
        Commands::Completions { action } => match action {
            CompletionsAction::Generate { shell } => {
                commands::completions::generate_completions(shell)
            }
            CompletionsAction::Install => commands::completions::install(),
        },
        Commands::Internal { operation, args } => commands::internal::execute(operation, args),
        Commands::Step { action } => match action {
            StepAction::Done => commands::step::execute("done", None),
            StepAction::Block { message } => commands::step::execute("block", message),
            StepAction::Fail { message } => commands::step::execute("fail", message),
        },
        Commands::Prev { task } => commands::prev::execute(task),
        Commands::Stop { task, kill_window } => commands::stop::execute(task, kill_window),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
