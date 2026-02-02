mod cli;
mod commands;
mod constants;
mod display;
mod error;
mod models;
mod services;
mod tui;

use clap::Parser;
use cli::{Cli, Commands, CompletionsAction};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => commands::init::execute(),
        Commands::Create { json } => commands::create::execute(json),
        Commands::Validate { name } => commands::validate::execute(name),
        Commands::List { tree, json } => commands::list::execute(tree, json),
        Commands::Run { task, all } | Commands::Start { name: task, all } => {
            commands::run::execute(task, all)
        }
        Commands::Review { name } => commands::review::execute(name),
        Commands::Resume { name } => commands::resume::execute(name),
        Commands::Merge { name, agent } => commands::merge::execute(name, agent),
        Commands::Delete { name, force } => commands::delete::execute(name, force),
        Commands::Archive { name } => commands::archive::execute(name),
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
        Commands::Internal { operation, args } => commands::internal::execute(operation, args),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
