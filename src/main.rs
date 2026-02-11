use clap::{Parser, Subcommand};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
};

const REMOTE: &str = "origin";
const WORKTREE_DIR: &str = ".worktrees";
const SESSION_PREFIX: &str = "wt-";
const ZELLIJ_LAYOUT: &str = "worktree";

#[derive(Parser, Debug)]
#[command(name = "graft", about = "Git worktree + Zellij session orchestrator")]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,

    /// Branch name to open (default command when no subcommand is used)
    branch: Option<String>,

    /// Ephemeral mode: after Zellij exits, delete session + remove worktree
    #[arg(short = 'e', long = "ephemeral")]
    ephemeral: bool,

    /// Also delete the local branch (dangerous; use with care)
    #[arg(long = "delete-branch")]
    delete_branch: bool,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Remove worktree + Zellij session for a branch
    Rm {
        branch: String,
        #[arg(long = "delete-branch")]
        delete_branch: bool,
    },

    /// List worktrees and Zellij sessions
    Ls {
        #[arg(long = "prune-worktrees")]
        prune_worktrees: bool,

        #[arg(long = "prune-sessions")]
        prune_sessions: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match (&cli.command, &cli.branch) {
        (Some(cmd), _) => run_subcommand(cmd),
        (None, Some(branch)) => open_branch(branch, cli.ephemeral, cli.delete_branch),
        (None, None) => Err(anyhow("Usage: graft <branch> | graft ls | graft rm <branch>")),
    };

    if let Err(e) = result {
        eprintln!("[graft] ERROR: {e}");
        std::process::exit(1);
    }
}

fn run_subcommand(cmd: &Cmd) -> Result<(), String> {
    match cmd {
        Cmd::Rm { branch, delete_branch } => rm_branch(branch, *delete_branch),
        Cmd::Ls { prune_worktrees, prune_sessions } => ls(*prune_worktrees, *prune_sessions),
    }
}

fn open_branch(branch: &str, ephemeral: bool, delete_branch: bool) -> Result<(), String> {
    log(&format!("open branch = '{branch}' (ephemeral={ephemeral})"));

    let repo_root = git_repo_root()?;
    let worktree_base = PathBuf::from(&repo_root).join(WORKTREE_DIR);
    let worktree_path = worktree_base.join(branch);
    let session = session_name(branch);

    fs::create_dir_all(&worktree_base)
        .map_err(|e| anyhow(&format!("Failed to create {}: {e}", worktree_base.display())))?;

    ensure_branch_exists(branch)?;
    ensure_worktree(&worktree_path, branch)?;

    log(&format!("cd {}", worktree_path.display()));
    let status = launch_zellij(&worktree_path, &session)?;

    if ephemeral {
        log("ephemeral cleanup: deleting session + removing worktree");
        let _ = delete_zellij_session(&session);
        let _ = remove_worktree(&worktree_path);
        if delete_branch {
            let _ = delete_local_branch(branch);
        }
    }

    if !status.success() {
        return Err(anyhow("Zellij exited with non-zero status"));
    }

    Ok(())
}

fn rm_branch(branch: &str, delete_branch_flag: bool) -> Result<(), String> {
    let repo_root = git_repo_root()?;
    let worktree_path = PathBuf::from(&repo_root).join(WORKTREE_DIR).join(branch);
    let session = session_name(branch);

    log(&format!(
        "rm branch='{branch}' session='{session}' worktree='{}'",
        worktree_path.display()
    ));

    let _ = delete_zellij_session(&session);
    remove_worktree(&worktree_path)?;

    if delete_branch_flag {
        delete_local_branch(branch)?;
    }

    Ok(())
}

fn ls(prune_worktrees: bool, prune_sessions: bool) -> Result<(), String> {
    let repo_root = git_repo_root()?;

    if prune_worktrees {
        log("git worktree prune");
        let _ = run_ok("git", ["worktree", "prune"])?;
    }

    let worktrees = list_worktrees()?;
    println!("Worktrees:");
    for wt in &worktrees {
        println!(
            "  - {}  ({})",
            wt.path.display(),
            wt.branch.clone().unwrap_or_else(|| "<detached>".into())
        );
    }

    let sessions = list_zellij_sessions().unwrap_or_default();
    println!("\nZellij sessions:");
    for s in &sessions {
        println!("  - {s}");
    }

    if prune_sessions {
        prune_stale_sessions(&repo_root, &worktrees, &sessions)?;
    }

    Ok(())
}

fn ensure_branch_exists(branch: &str) -> Result<(), String> {
    if git_local_branch_exists(branch)? {
        log(&format!("local branch '{branch}' exists"));
        return Ok(());
    }

    if git_remote_branch_exists(branch)? {
        log(&format!("remote branch '{branch}' exists on {REMOTE}, fetching"));
        let spec = format!("{branch}:{branch}");
        run_ok("git", ["fetch", REMOTE, &spec])?;
        return Ok(());
    }

    log(&format!("branch '{branch}' does not exist anywhere, creating locally"));
    run_ok("git", ["branch", branch])?;
    Ok(())
}

fn ensure_worktree(worktree_path: &Path, branch: &str) -> Result<(), String> {
    let exists_in_git = git_worktree_known(worktree_path)?;
    let dir_exists = worktree_path.is_dir();

    if exists_in_git && dir_exists {
        log("worktree exists and directory is present");
        return Ok(());
    }

    log("worktree missing or stale -> prune + add");
    run_ok("git", ["worktree", "prune"])?;

    if worktree_path.exists() && !worktree_path.is_dir() {
        return Err(anyhow(&format!(
            "Worktree path exists but is not a directory: {}",
            worktree_path.display()
        )));
    }

    if exists_in_git && !dir_exists {
        let _ = run_ok(
            "git",
            ["worktree", "remove", "--force", worktree_path.to_string_lossy().as_ref()],
        );
    }

    run_ok(
        "git",
        ["worktree", "add", worktree_path.to_string_lossy().as_ref(), branch],
    )?;

    Ok(())
}

fn launch_zellij(cwd: &Path, session: &str) -> Result<ExitStatus, String> {
    let server_running = Command::new("zellij")
        .args(["list-sessions"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if server_running {
        log("zellij server running -> attach -c");
        Command::new("zellij")
            .args(["attach", "-c", session])
            .current_dir(cwd)
            .status()
            .map_err(|e| anyhow(&format!("Failed to run zellij attach: {e}")))
    } else {
        // Zellij 0.42.x: must use -n to start a new session with layout
        log("no zellij server -> zellij -n <layout> -s <session>");
        Command::new("zellij")
            .args(["-n", ZELLIJ_LAYOUT, "-s", session])
            .current_dir(cwd)
            .status()
            .map_err(|e| anyhow(&format!("Failed to run zellij new session: {e}")))
    }
}

fn delete_zellij_session(session: &str) -> Result<(), String> {
    let status = Command::new("zellij")
        .args(["delete-session", session])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| anyhow(&format!("Failed to execute zellij delete-session: {e}")))?;

    if !status.success() {
        log(&format!("zellij delete-session '{session}' failed (ignored)"));
    }
    Ok(())
}

fn remove_worktree(worktree_path: &Path) -> Result<(), String> {
    if !worktree_path.exists() {
        log("worktree path does not exist on disk (still trying git prune)");
        let _ = run_ok("git", ["worktree", "prune"])?;
        return Ok(());
    }

    run_ok(
        "git",
        ["worktree", "remove", "--force", worktree_path.to_string_lossy().as_ref()],
    )?;

    Ok(())
}

fn delete_local_branch(branch: &str) -> Result<(), String> {
    log(&format!("deleting local branch '{branch}'"));
    run_ok("git", ["branch", "-D", branch])?;
    Ok(())
}

fn session_name(branch: &str) -> String {
    format!("{SESSION_PREFIX}{}", branch.replace('/', "-"))
}

fn git_repo_root() -> Result<String, String> {
    let out = run_capture("git", ["rev-parse", "--show-toplevel"])?;
    Ok(out.trim().to_string())
}

fn git_local_branch_exists(branch: &str) -> Result<bool, String> {
    let status = Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .status()
        .map_err(|e| anyhow(&format!("Failed to run git show-ref: {e}")))?;
    Ok(status.success())
}

fn git_remote_branch_exists(branch: &str) -> Result<bool, String> {
    let status = Command::new("git")
        .args(["ls-remote", "--exit-code", "--heads", REMOTE, branch])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| anyhow(&format!("Failed to run git ls-remote: {e}")))?;
    Ok(status.success())
}

fn git_worktree_known(worktree_path: &Path) -> Result<bool, String> {
    let out = run_capture("git", ["worktree", "list", "--porcelain"])?;
    let needle = format!("worktree {}", worktree_path.display());
    Ok(out.lines().any(|l| l.trim() == needle))
}

#[derive(Debug)]
struct WorktreeInfo {
    path: PathBuf,
    branch: Option<String>,
}

fn list_worktrees() -> Result<Vec<WorktreeInfo>, String> {
    let out = run_capture("git", ["worktree", "list", "--porcelain"])?;
    let mut res = Vec::new();

    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in out.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("worktree ") {
            if let Some(p) = current_path.take() {
                res.push(WorktreeInfo { path: p, branch: current_branch.take() });
            }
            current_path = Some(PathBuf::from(rest));
            current_branch = None;
        } else if let Some(rest) = line.strip_prefix("branch ") {
            current_branch = rest
                .strip_prefix("refs/heads/")
                .map(|s| s.to_string())
                .or_else(|| Some(rest.to_string()));
        }
    }

    if let Some(p) = current_path.take() {
        res.push(WorktreeInfo { path: p, branch: current_branch.take() });
    }

    Ok(res)
}

fn list_zellij_sessions() -> Result<Vec<String>, String> {
    let output = Command::new("zellij")
        .args(["list-sessions"])
        .output()
        .map_err(|e| anyhow(&format!("Failed to run zellij list-sessions: {e}")))?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut sessions = Vec::new();
    for line in stdout.lines() {
        let name = line.split_whitespace().next().unwrap_or("").trim();
        if !name.is_empty() {
            sessions.push(name.to_string());
        }
    }
    Ok(sessions)
}

fn prune_stale_sessions(repo_root: &str, _worktrees: &[WorktreeInfo], sessions: &[String]) -> Result<(), String> {
    let worktree_base = PathBuf::from(repo_root).join(WORKTREE_DIR);

    for s in sessions {
        if !s.starts_with(SESSION_PREFIX) {
            continue;
        }

        let suffix = &s[SESSION_PREFIX.len()..];
        let mut keep = false;

        if let Ok(entries) = fs::read_dir(&worktree_base) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if let Some(dir_name) = p.file_name().and_then(OsStr::to_str) {
                        if dir_name.replace('/', "-") == suffix {
                            keep = true;
                            break;
                        }
                    }
                }
            }
        }

        if !keep {
            log(&format!("pruning stale session: {s}"));
            let _ = delete_zellij_session(s);
        }
    }

    Ok(())
}

fn run_ok<I, S>(program: &str, args: I) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| anyhow(&format!("Failed to execute {program}: {e}")))?;

    if !status.success() {
        return Err(anyhow(&format!("{program} returned non-zero exit code")));
    }
    Ok(())
}

fn run_capture<I, S>(program: &str, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| anyhow(&format!("Failed to execute {program}: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow(&format!("{program} failed: {}", stderr.trim())));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn log(msg: &str) {
    eprintln!("[graft] {msg}");
}

fn anyhow(msg: &str) -> String {
    msg.to_string()
}
