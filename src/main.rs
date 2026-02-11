mod cli;
mod git;
mod zellij;
use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use git2::{Branch, Repository};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = Repository::discover(".")?;
    let worktrees = repo.worktrees()?;
    let worktrees: Vec<_> = worktrees.iter().flatten().collect();
    let sessions = zellij::get_sessions();
    let branches = repo.branches(None).unwrap();

    match cli.command {
        Command::Open {
            branch,
            ephemeral,
            delete_branch,
        } => {
            todo!("Implement open command");
        }

        Command::Rm {
            branch,
            delete_branch,
        } => {
            todo!("Implement rm command");
        }

        Command::Ls { .. } => {
            println!("Branches:");
            for wt in worktrees {
                println!("{wt}");
            }
        }
    }

    Ok(())
}

fn cleanup(repo: &Repo, branch: &str, delete_branch: bool) {
    if let Err(e) = session::delete(branch) {
        eprintln!("Failed to delete session: {e}");
    }

    if let Err(e) = repo.remove_worktree(branch) {
        eprintln!("Failed to remove worktree: {e}");
    }

    if delete_branch {
        if let Err(e) = repo.delete_local_branch(branch) {
            eprintln!("Failed to delete branch: {e}");
        }
    }
}
