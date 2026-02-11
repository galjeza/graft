use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
};
use git2::{BranchType, FetchOptions, Repository};

const REMOTE: &str = "origin";
const WORKTREE_DIR: &str = ".worktrees";
const SESSION_PREFIX: &str = "wt-";
const ZELLIJ_LAYOUT: &str = "worktree";

#[derive(Parser, Debug)]
#[command(name = "graft", about = "Git worktree + Zellij session orchestrator")]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,

    branch: Option<String>,

    #[arg(short, long)]
    ephemeral: bool,

    #[arg(long)]
    delete_branch: bool,
}

#[derive(Subcommand, Debug)]
enum Cmd {
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match (&cli.command, &cli.branch) {
        (Some(cmd), _) => run_subcommand(cmd)?,
        (None, Some(branch)) => open_branch(branch, cli.ephemeral, cli.delete_branch)?,
        (None, None) => anyhow::bail!("Usage: graft <branch> | graft ls | graft rm <branch>"),
    }

    Ok(())
}

fn run_subcommand(cmd: &Cmd) -> Result<()> {
    match cmd {
        Cmd::Rm { branch, delete_branch } => rm_branch(branch, *delete_branch),
        Cmd::Ls { prune_worktrees, prune_sessions } => ls(*prune_worktrees, *prune_sessions),
    }
}

fn open_branch(branch: &str, ephemeral: bool, delete_branch: bool) -> Result<()> {
    let repo_root = git_repo_root()?;
    let worktree_base = PathBuf::from(&repo_root).join(WORKTREE_DIR);
    let worktree_path = worktree_base.join(branch);
    let session = session_name(branch);

    fs::create_dir_all(&worktree_base)
        .with_context(|| format!("Failed to create {}", worktree_base.display()))?;

    ensure_branch_exists(branch)?;
    ensure_worktree(&worktree_path, branch)?;

    let status = launch_zellij(&worktree_path, &session)?;

    if ephemeral {
        let _ = delete_zellij_session(&session);
        let _ = remove_worktree(&worktree_path);
        if delete_branch {
            let _ = delete_local_branch(branch);
        }
    }

    if !status.success() {
        anyhow::bail!("Zellij exited with non-zero status");
    }

    Ok(())
}

fn rm_branch(branch: &str, delete_branch: bool) -> Result<()> {
    let repo_root = git_repo_root()?;
    let worktree_path = PathBuf::from(&repo_root).join(WORKTREE_DIR).join(branch);
    let session = session_name(branch);

    let _ = delete_zellij_session(&session);
    remove_worktree(&worktree_path)?;

    if delete_branch {
        delete_local_branch(branch)?;
    }

    Ok(())
}

fn ls(prune_worktrees: bool, prune_sessions: bool) -> Result<()> {
    let repo_root = git_repo_root()?;

    if prune_worktrees {
        run_ok("git", ["worktree", "prune"])?;
    }

    let worktrees = list_worktrees()?;

    println!("Worktrees:");
    for wt in &worktrees {
        println!(
            "  - {}  ({})",
            wt.path.display(),
            wt.branch.as_deref().unwrap_or("<detached>")
        );
    }

    let sessions = list_zellij_sessions().unwrap_or_default();

    println!("\nZellij sessions:");
    for s in &sessions {
        println!("  - {s}");
    }

    if prune_sessions {
        prune_stale_sessions(&repo_root, &sessions)?;
    }

    Ok(())
}

fn ensure_branch_exists(branch: &str) -> Result<()> {
    let repo = Repository::discover(".")?;

    if repo.find_branch(branch, BranchType::Local).is_ok() {
        return Ok(());
    }

    if repo
        .find_branch(&format!("{REMOTE}/{branch}"), BranchType::Remote)
        .is_ok()
    {
        // Fetch branch
        let mut remote = repo.find_remote(REMOTE)?;
        let mut fetch_options = FetchOptions::new();
        remote.fetch(&[branch], Some(&mut fetch_options), None)?;

        // Create local branch from fetched reference
        let fetch_head = repo.find_reference(&format!("refs/remotes/{REMOTE}/{branch}"))?;
        let commit = fetch_head.peel_to_commit()?;
        repo.branch(branch, &commit, false)?;
        return Ok(());
    }

    // Create new branch from HEAD
    let head = repo.head()?.peel_to_commit()?;
    repo.branch(branch, &head, false)?;

    Ok(())
}

fn ensure_worktree(worktree_path: &Path, branch: &str) -> Result<()> {
    if git_worktree_known(worktree_path)? && worktree_path.is_dir() {
        return Ok(());
    }

    run_ok("git", ["worktree", "prune"])?;

    run_ok(
        "git",
        ["worktree", "add", worktree_path.to_string_lossy().as_ref(), branch],
    )?;

    Ok(())
}

fn launch_zellij(cwd: &Path, session: &str) -> Result<ExitStatus> {
    let server_running = Command::new("zellij")
        .args(["list-sessions"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if server_running {
        Command::new("zellij")
            .args(["attach", "-c", session])
            .current_dir(cwd)
            .status()
            .context("Failed to run zellij attach")
    } else {
        Command::new("zellij")
            .args(["-n", ZELLIJ_LAYOUT, "-s", session])
            .current_dir(cwd)
            .status()
            .context("Failed to run zellij new session")
    }
}

fn delete_zellij_session(session: &str) -> Result<()> {
    let _ = Command::new("zellij")
        .args(["delete-session", session])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    Ok(())
}

fn remove_worktree(path: &Path) -> Result<()> {
    if path.exists() {
        run_ok(
            "git",
            ["worktree", "remove", "--force", path.to_string_lossy().as_ref()],
        )?;
    }
    Ok(())
}

fn delete_local_branch(branch: &str) -> Result<()> {
    let repo = Repository::discover(".")?;
    let mut b = repo.find_branch(branch, BranchType::Local)?;
    b.delete()?;
    Ok(())
}

fn session_name(branch: &str) -> String {
    format!("{SESSION_PREFIX}{}", branch.replace('/', "-"))
}

fn git_repo_root() -> Result<String> {
    let repo = Repository::discover(".")
        .context("Not inside a Git repository")?;

    let path = repo
        .workdir()
        .context("Repository has no working directory")?;

    Ok(path.to_path_buf().to_string_lossy().to_string())
}

fn git_local_branch_exists(branch: &str) -> Result<bool> {
    let repo = Repository::discover(".")?;
    Ok(repo.find_branch(branch, BranchType::Local).is_ok())
}

fn git_remote_branch_exists(branch: &str) -> Result<bool> {
    let repo = Repository::discover(".")?;

    let remote_branch = format!("{REMOTE}/{branch}");
    Ok(repo
        .find_branch(&remote_branch, BranchType::Remote)
        .is_ok())
}

fn git_worktree_known(path: &Path) -> Result<bool> {
    let out = run_capture("git", ["worktree", "list", "--porcelain"])?;
    let needle = format!("worktree {}", path.display());
    Ok(out.lines().any(|l| l.trim() == needle))
}

#[derive(Debug)]
struct WorktreeInfo {
    path: PathBuf,
    branch: Option<String>,
}

fn list_worktrees() -> Result<Vec<WorktreeInfo>> {
    let out = run_capture("git", ["worktree", "list", "--porcelain"])?;
    let mut res = Vec::new();
    let mut current: Option<WorktreeInfo> = None;

    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            if let Some(wt) = current.take() {
                res.push(wt);
            }
            current = Some(WorktreeInfo {
                path: PathBuf::from(rest),
                branch: None,
            });
        } else if let Some(rest) = line.strip_prefix("branch ") {
            if let Some(wt) = current.as_mut() {
                wt.branch = rest
                    .strip_prefix("refs/heads/")
                    .map(|s| s.to_string())
                    .or_else(|| Some(rest.to_string()));
            }
        }
    }

    if let Some(wt) = current {
        res.push(wt);
    }

    Ok(res)
}

fn list_zellij_sessions() -> Result<Vec<String>> {
    let output = Command::new("zellij")
        .args(["list-sessions"])
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.to_string())
        .collect())
}

fn prune_stale_sessions(repo_root: &str, sessions: &[String]) -> Result<()> {
    let worktree_base = PathBuf::from(repo_root).join(WORKTREE_DIR);

    for s in sessions {
        if !s.starts_with(SESSION_PREFIX) {
            continue;
        }

        let suffix = &s[SESSION_PREFIX.len()..];
        let keep = fs::read_dir(&worktree_base)?
            .flatten()
            .any(|e| {
                e.path()
                    .file_name()
                    .and_then(OsStr::to_str)
                    .map(|n| n.replace('/', "-") == suffix)
                    .unwrap_or(false)
            });

        if !keep {
            let _ = delete_zellij_session(s);
        }
    }

    Ok(())
}

fn run_ok<I, S>(program: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new(program).args(args).status()?;
    if !status.success() {
        anyhow::bail!("{program} failed");
    }
    Ok(())
}

fn run_capture<I, S>(program: &str, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        anyhow::bail!("{program} failed");
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
