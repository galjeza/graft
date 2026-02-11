use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

const ZELLIJ_LAYOUT: &str = "worktree";

pub fn launch(cwd: &Path, session: &str) -> Result<ExitStatus> {
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

pub fn delete(session: &str) -> Result<()> {
    let _ = Command::new("zellij")
        .args(["delete-session", session])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    Ok(())
}
