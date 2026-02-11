mod cli;
mod repo;
mod session;

use anyhow::{Result, bail};
use clap::Parser;
use cli::{Cli, Cmd};
use repo::Repo;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = Repo::open()?; // create repo instance

    match (&cli.command, &cli.branch) {
        (Some(cmd), _) => match cmd {
            Cmd::Rm {
                branch,
                delete_branch,
            } => {
                repo.remove_branch(branch, *delete_branch)?;
            }
            Cmd::Ls { .. } => {
                repo.list_worktrees()?;
            }
        },
        (None, Some(branch)) => {
            repo.open_worktree(branch, cli.ephemeral, cli.delete_branch)?;
        }
        (None, None) => {
            bail!("Usage: graft <branch> | graft ls | graft rm <branch>");
        }
    }

    Ok(())
}
