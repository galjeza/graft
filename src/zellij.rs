pub fn get_sessions() -> Vec<String> {
    let output = std::process::Command::new("zellij")
        .arg("list-sessions")
        .output()
        .expect("Failed to execute zellij command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().map(|line| line.trim().to_string()).collect()
}
