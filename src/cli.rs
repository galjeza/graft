use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "graft", about = "Git worktree + Zellij session orchestrator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Cmd>,

    pub branch: Option<String>,

    #[arg(short, long)]
    pub ephemeral: bool,

    #[arg(long)]
    pub delete_branch: bool,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    Rm {
        branch: String,
        #[arg(long)]
        delete_branch: bool,
    },
    Ls {
        #[arg(long)]
        prune_worktrees: bool,
        #[arg(long)]
        prune_sessions: bool,
    },
}
