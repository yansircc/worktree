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
        Commands::Run { task, all } => commands::run::execute(task, all),
        Commands::Review { name } => commands::review::execute(name),
        Commands::Resume { name } => commands::resume::execute(name),
        Commands::Complete { name } => commands::complete::execute(name),
        Commands::Delete { name, force } => commands::delete::execute(name, force),
        Commands::Next { task } => commands::next::execute(task),
        Commands::Reset { name, to } => commands::reset::execute(name, to),
        Commands::Status { json, verbose, action, task } => commands::status::execute(json, verbose, action, task),
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
        Commands::Hooks { action } => commands::hooks_cmd::execute(action),
        Commands::Pause { name, reason } => commands::pause::execute(name, reason),
        Commands::Pipeline { action } => commands::pipeline_cmd::execute(action),
        // Phases v2 commands
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
