use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::time::{Duration, sleep};

pub struct PostgresManager {
    data_dir: PathBuf,
    log_path: PathBuf,
    bin_dir: PathBuf,
    port: u16,
}

impl PostgresManager {
    pub fn new(data_dir: PathBuf, log_dir: PathBuf, bin_dir: PathBuf, port: u16) -> Self {
        Self {
            log_path: log_dir.join("postgres.log"),
            data_dir,
            bin_dir,
            port,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Initialize the data directory if it doesn't exist yet.
    pub async fn init_db_if_needed(&self) -> Result<()> {
        if self.data_dir.join("PG_VERSION").exists() {
            tracing::info!(
                data_dir = %self.data_dir.display(),
                "PostgreSQL data directory already initialized"
            );
            return Ok(());
        }

        std::fs::create_dir_all(&self.data_dir).with_context(|| {
            format!(
                "Failed to create data directory: {}",
                self.data_dir.display()
            )
        })?;

        let initdb = self.bin_path("initdb");
        tracing::info!(
            initdb = %initdb.display(),
            data_dir = %self.data_dir.display(),
            "Initializing PostgreSQL data directory"
        );

        let output = Command::new(&initdb)
            .arg("-D")
            .arg(&self.data_dir)
            .arg("--auth=trust")
            .arg("--encoding=UTF8")
            .arg("--no-locale")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| format!("Failed to run {}", initdb.display()))?;

        if !output.status.success() {
            bail!(
                "initdb failed (exit {}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        tracing::info!("PostgreSQL data directory initialized");
        Ok(())
    }

    /// Start the PostgreSQL server.
    pub async fn start(&self) -> Result<()> {
        // Ensure log directory exists
        if let Some(parent) = self.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Check if already running
        if self.is_running().await {
            tracing::info!(port = self.port, "PostgreSQL is already running");
            return Ok(());
        }

        let pg_ctl = self.bin_path("pg_ctl");
        tracing::info!(
            port = self.port,
            data_dir = %self.data_dir.display(),
            "Starting PostgreSQL"
        );

        let output = Command::new(&pg_ctl)
            .arg("start")
            .arg("-D")
            .arg(&self.data_dir)
            .arg("-l")
            .arg(&self.log_path)
            .arg("-o")
            .arg(format!("-p {} -h 127.0.0.1", self.port))
            .arg("-w") // wait for startup to complete
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| format!("Failed to run {}", pg_ctl.display()))?;

        if !output.status.success() {
            bail!(
                "pg_ctl start failed (exit {}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        self.wait_for_ready().await?;
        tracing::info!(port = self.port, "PostgreSQL started");
        Ok(())
    }

    /// Stop the PostgreSQL server.
    pub async fn stop(&self) -> Result<()> {
        let pg_ctl = self.bin_path("pg_ctl");
        tracing::info!(
            data_dir = %self.data_dir.display(),
            "Stopping PostgreSQL"
        );

        let output = Command::new(&pg_ctl)
            .arg("stop")
            .arg("-D")
            .arg(&self.data_dir)
            .arg("-m")
            .arg("fast")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| format!("Failed to run {}", pg_ctl.display()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Not an error if already stopped
            if !stderr.contains("not running") && !stderr.contains("No such file") {
                bail!("pg_ctl stop failed (exit {}): {}", output.status, stderr);
            }
        }

        tracing::info!("PostgreSQL stopped");
        Ok(())
    }

    /// Create the `eurora` database if it doesn't exist.
    pub async fn ensure_database(&self) -> Result<()> {
        let createdb = self.bin_path("createdb");
        let output = Command::new(&createdb)
            .arg("-h")
            .arg("127.0.0.1")
            .arg("-p")
            .arg(self.port.to_string())
            .arg("-U")
            .arg(Self::current_user())
            .arg("eurora")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run createdb")?;

        if output.status.success() {
            tracing::info!("Created 'eurora' database");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") {
                tracing::info!("Database 'eurora' already exists");
            } else {
                bail!("createdb failed: {}", stderr);
            }
        }

        Ok(())
    }

    /// Build the PostgreSQL connection URL for be-monolith.
    pub fn connection_url(&self) -> String {
        format!(
            "postgresql://{}@127.0.0.1:{}/eurora",
            Self::current_user(),
            self.port
        )
    }

    /// Check if the PostgreSQL server is accepting connections.
    async fn is_running(&self) -> bool {
        TcpStream::connect(("127.0.0.1", self.port)).await.is_ok()
    }

    /// Wait for PostgreSQL to accept connections.
    async fn wait_for_ready(&self) -> Result<()> {
        for attempt in 1..=30 {
            if self.is_running().await {
                return Ok(());
            }
            tracing::debug!(attempt, "Waiting for PostgreSQL to be ready...");
            sleep(Duration::from_millis(500)).await;
        }
        bail!(
            "PostgreSQL did not become ready within 15 seconds (port {})",
            self.port
        )
    }

    fn bin_path(&self, name: &str) -> PathBuf {
        let path = self.bin_dir.join(name);
        // On Windows, append .exe if not already present
        if cfg!(windows) && path.extension().is_none() {
            path.with_extension("exe")
        } else {
            path
        }
    }

    fn current_user() -> String {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "postgres".to_string())
    }
}
