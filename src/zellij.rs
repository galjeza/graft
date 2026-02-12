pub fn sessions() -> Vec<String> {
    let output = std::process::Command::new("zellij")
        .arg("list-sessions")
        .arg("--short")
        .output()
        .expect("Failed to execute zellij command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().map(|line| line.trim().to_string()).collect()
}

pub fn start_session(session_name: &str, dir: &str) {
    std::process::Command::new("zellij")
        .arg("attach-session")
        .arg("--session")
        .current_dir(dir)
        .arg(session_name)
        .status()
        .expect("Failed to start zellij session");
}

pub fn kill_session(session_name: &str) {
    std::process::Command::new("zellij")
        .arg("kill-session")
        .arg("--session")
        .arg(session_name)
        .status()
        .expect("Failed to kill zellij session");
}
