use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "graft", about = "Git worktree + Zellij session orchestrator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Open (or create) a worktree and attach a Zellij session
    Open {
        branch: String,

        #[arg(short, long)]
        ephemeral: bool,

        #[arg(long)]
        delete_branch: bool,
    },

    /// Remove worktree and optionally delete branch
    Rm {
        branch: String,

        #[arg(long)]
        delete_branch: bool,
    },

    /// List worktrees
    Ls {
        #[arg(long)]
        prune_worktrees: bool,

        #[arg(long)]
        prune_sessions: bool,
    },
}
