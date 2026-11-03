use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub success: bool,
    pub ticker: String,
    pub name: String,
    pub csv_path: Option<String>,
    pub rows: usize,
    pub cached: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDownloadResult {
    pub success: bool,
    pub count: usize,
    pub ok_count: usize,
    pub failed_count: usize,
    pub results: Vec<DownloadResult>,
}

fn get_runner() -> (String, Vec<String>) {
    // 1. Check if uv is in PATH
    if let Ok(out) = Command::new("uv").arg("--version").output()
        && out.status.success()
    {
        return (
            "uv".to_string(),
            vec!["run".to_string(), "data_downloader.py".to_string()],
        );
    }

    // 2. Common macOS and Linux locations for uv
    let uv_candidates = ["/opt/homebrew/bin/uv", "/usr/local/bin/uv"];
    for candidate in uv_candidates {
        if std::path::Path::new(candidate).exists() {
            return (
                candidate.to_string(),
                vec!["run".to_string(), "data_downloader.py".to_string()],
            );
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let cargo_uv = format!("{home}/.cargo/bin/uv");
        if std::path::Path::new(&cargo_uv).exists() {
            return (
                cargo_uv,
                vec!["run".to_string(), "data_downloader.py".to_string()],
            );
        }
        let local_uv = format!("{home}/.local/bin/uv");
        if std::path::Path::new(&local_uv).exists() {
            return (
                local_uv,
                vec!["run".to_string(), "data_downloader.py".to_string()],
            );
        }
    }

    // 3. Fallback to local virtualenv python
    if std::path::Path::new(".venv/bin/python").exists() {
        return (
            ".venv/bin/python".to_string(),
            vec!["data_downloader.py".to_string()],
        );
    }

    // 4. Fallback to system python3
    return ("python3".to_string(), vec!["data_downloader.py".to_string()]);
}

pub struct PythonBridge;

impl PythonBridge {
    /// Batch download multiple tickers with a hard timeout of 35 seconds.
    pub fn batch_ensure_tickers(
        tickers: &[String],
        force: bool,
    ) -> Result<BatchDownloadResult, String> {
        if tickers.is_empty() {
            return Ok(BatchDownloadResult {
                success: true,
                count: 0,
                ok_count: 0,
                failed_count: 0,
                results: Vec::new(),
            });
        }

        let (runner_bin, base_args) = get_runner();
        let mut cmd = Command::new(runner_bin);
        for arg in base_args {
            cmd.arg(arg);
        }
        cmd.arg("batch");

        for t in tickers {
            cmd.arg(t);
        }

        cmd.arg("--period").arg("2y");
        cmd.arg("--json");
        if force {
            cmd.arg("--force");
        }

        // Dynamic timeout based on ticker count: min 60s, +3s per ticker (e.g. 100 tickers = 345s)
        let timeout_secs = (45 + tickers.len() as u64 * 3).max(60);
        let (tx, rx) = mpsc::channel();
        let thread_handle = thread::spawn(move || {
            let output = cmd.output();
            let _ = tx.send(output);
        });

        let output = match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => {
                return Err(format!("Failed to execute data_downloader.py: {e}"));
            }
            Err(_) => {
                // Drop receiver; thread will finish in background without blocking caller
                drop(thread_handle);
                return Err(format!(
                    "Download timed out after {timeout_secs}s. Yahoo Finance may be unresponsive or network blocked. Please retry."
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Find JSON line in stdout (last line starting with { and ending with })
        let json_line = stdout
            .lines()
            .rev()
            .find(|line| line.trim().starts_with('{') && line.trim().ends_with('}'))
            .ok_or_else(|| {
                format!(
                    "No valid JSON output from data_downloader. Stdout: {}, Stderr: {}",
                    stdout, stderr
                )
            })?;

        let res: BatchDownloadResult = serde_json::from_str(json_line).map_err(|e| {
            format!(
                "Failed to parse JSON response: {e}. Raw output: {json_line}"
            )
        })?;

        return Ok(res);
    }

    /// Single ticker download helper
    pub fn ensure_ticker(ticker: &str, force: bool) -> Result<DownloadResult, String> {
        let batch_res = Self::batch_ensure_tickers(&[ticker.to_string()], force)?;
        return batch_res
            .results
            .into_iter()
            .next()
            .ok_or_else(|| "No result returned for ticker".to_string());
    }
}
