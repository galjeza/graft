pub fn sessions() -> Vec<String> {
    let output = std::process::Command::new("zellij")
        .arg("list-sessions")
        .arg("--short")
        .output()
        .expect("Failed to execute zellij command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().map(|line| line.trim().to_string()).collect()
}

pub fn attach_or_create(session: &str, dir: &str) {
    std::process::Command::new("zellij")
        .arg("attach")
        .arg("--create")
        .arg(session)
        .current_dir(dir)
        .status()
        .expect("Failed to attach or create zellij session");
}

pub fn kill_session(session_name: &str) {
    std::process::Command::new("zellij")
        .arg("kill-session")
        .arg("--session")
        .arg(session_name)
        .status()
        .expect("Failed to kill zellij session");
}
