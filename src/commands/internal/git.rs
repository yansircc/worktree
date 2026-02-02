use crate::error::{Result, WtError};
use crate::services::git;

/// Execute a git atomic operation.
pub fn execute(operation: &str, args: Vec<String>) -> Result<()> {
    match operation {
        "fetch" => {
            require_args(&args, 2, "fetch <repo_root> <remote>")?;
            git::fetch(&args[0], &args[1])
        }
        "rebase" => {
            require_args(&args, 2, "rebase <worktree_path> <target>")?;
            let result = git::rebase(&args[0], &args[1])?;
            match result {
                git::RebaseResult::Success => {
                    println!("Rebase successful");
                    Ok(())
                }
                git::RebaseResult::AlreadyUpToDate => {
                    println!("Already up to date");
                    Ok(())
                }
                git::RebaseResult::Conflicts => {
                    eprintln!("Rebase failed due to conflicts");
                    std::process::exit(1);
                }
            }
        }
        "squash-merge" => {
            require_args(&args, 2, "squash-merge <repo_root> <branch>")?;
            git::squash_merge(&args[0], &args[1])
        }
        "commit" => {
            require_args(&args, 2, "commit <path> <message>")?;
            git::commit(&args[0], &args[1])
        }
        "push" => {
            require_args(&args, 2, "push <repo_root> <branch> [remote]")?;
            let remote = args.get(2).map(|s| s.as_str()).unwrap_or("origin");
            git::push(&args[0], &args[1], remote)
        }
        "has-changes" => {
            require_args(&args, 1, "has-changes <path>")?;
            let has = git::has_changes(&args[0])?;
            if has {
                println!("true");
                std::process::exit(0);
            } else {
                println!("false");
                std::process::exit(1);
            }
        }
        "has-conflicts" => {
            require_args(&args, 1, "has-conflicts <path>")?;
            let has = git::has_conflicts(&args[0]);
            if has {
                println!("true");
                std::process::exit(0);
            } else {
                println!("false");
                std::process::exit(1);
            }
        }
        "stash" => {
            require_args(&args, 1, "stash <path>")?;
            git::stash(&args[0])
        }
        "stash-pop" => {
            require_args(&args, 1, "stash-pop <path>")?;
            git::stash_pop(&args[0])
        }
        "create-branch" => {
            require_args(&args, 2, "create-branch <repo_root> <branch>")?;
            git::create_branch(&args[0], &args[1])
        }
        "delete-branch" => {
            require_args(&args, 2, "delete-branch <repo_root> <branch>")?;
            git::delete_branch(&args[0], &args[1])
        }
        "checkout" => {
            require_args(&args, 2, "checkout <path> <branch>")?;
            git::checkout(&args[0], &args[1])
        }
        "current-branch" => {
            require_args(&args, 1, "current-branch <path>")?;
            let branch = git::current_branch(&args[0])?;
            println!("{}", branch);
            Ok(())
        }
        _ => Err(WtError::InvalidInput(format!(
            "Unknown git operation: {}",
            operation
        ))),
    }
}

fn require_args(args: &[String], min: usize, usage: &str) -> Result<()> {
    if args.len() < min {
        Err(WtError::InvalidInput(format!(
            "Not enough arguments. Usage: wt internal git:{}",
            usage
        )))
    } else {
        Ok(())
    }
}
