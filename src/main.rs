mod cli;
mod git;
mod zellij;
use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use git::Git;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let git = Git::new(".");
    println!("Starting ");

    match cli.command {
        Command::Open {
            ticket,
            ephemeral,
            delete_branch,
        } => {
            let _worktree = git.ensure_worktree(&ticket);
            let _session = zellij::attach_or_create(&ticket, _worktree.path().to_str().unwrap());

            // start zellij session for the ticket
            todo!("Implement open command");
        }

        Command::Rm {
            ticket,
            delete_branch,
        } => {
            todo!("Implement rm command");
        }

        Command::Ls { .. } => {
            let zellij_sessions = zellij::sessions();
            println!("\nZellij Sessions:");
            for session in zellij_sessions {
                println!("{session}");
            }
            todo!("Implement ls command to list worktrees and zellij sessions");
        }
    }
}
