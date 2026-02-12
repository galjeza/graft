mod cli;
mod git;
mod zellij;
use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use git::Git;

fn main() -> Result<()> {
    // let cli = Cli::parse();
    let git = Git::new(".");
    println!("Starting ");

    // demo ticket for testing
    let ticket = "demo-123";

    println!("Ensuring branch and worktree for ticket: {}", ticket);
    git.ensure_branch(&ticket);
    println!("Ensured branch for ticket: {}", ticket);
    println!("Ensuring worktree for ticket: {}", ticket);
    git.ensure_worktree(&ticket);
    println!("Ensured worktree for ticket: {}", ticket);

    // let zellij_sessions = zellij::sessions();

    // dbg!(git.branches());
    // dbg!(git.worktrees());
    // dbg!(zellij_sessions);

    // match cli.command {
    //     Command::Open {
    //         ticket,
    //         ephemeral,
    //         delete_branch,
    //     } => {
    //         git.ensure_branch(&ticket);
    //         git.ensure_worktree(&ticket);
    //         // start zellij session for the ticket
    //         todo!("Implement open command");
    //     }
    //
    //     Command::Rm {
    //         ticket,
    //         delete_branch,
    //     } => {
    //         todo!("Implement rm command");
    //     }
    //
    //     Command::Ls { .. } => {
    //         let git_branches = git.branches();
    //         let zellij_sessions = zellij::sessions();
    //         todo!("Implement ls");
    //     }
    // }

    Ok(())
}
