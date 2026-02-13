/// Inspect the running `backend` docker compose service to discover its gRPC port.
/// Returns `Some("http://0.0.0.0:<port>")` if found, `None` otherwise.
pub fn detect_local_backend_endpoint() -> Option<String> {
    let ps = std::process::Command::new("docker")
        .args(["compose", "ps", "-q", "backend"])
        .output()
        .ok()?;

    let container_id = std::str::from_utf8(&ps.stdout).ok()?.trim().to_string();
    if container_id.is_empty() {
        return None;
    }

    let inspect = std::process::Command::new("docker")
        .args([
            "inspect",
            "--format",
            "{{range .Config.Env}}{{println .}}{{end}}",
            &container_id,
        ])
        .output()
        .ok()?;

    let env_output = std::str::from_utf8(&inspect.stdout).ok()?;
    for line in env_output.lines() {
        if let Some(addr) = line.strip_prefix("MONOLITH_ADDR=") {
            let port = addr.rsplit(':').next()?;
            return Some(format!("http://0.0.0.0:{}", port));
        }
    }

    None
}
