use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::error::{EzError, Result};

use super::model::PluginManifest;
use super::protocol::{HookRequest, HookResponse};

/// Execute a plugin with the given request and return its response.
pub fn execute(
    manifest: &PluginManifest,
    plugin_dir: &std::path::Path,
    request: &HookRequest,
    timeout_secs: u64,
) -> Result<HookResponse> {
    let executable = plugin_dir.join(&manifest.executable);

    if !executable.exists() {
        return Err(EzError::PluginNotFound(format!(
            "Plugin executable not found: {}",
            executable.display()
        )));
    }

    let request_json = serde_json::to_string(request)?;

    let mut child = Command::new(&executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| EzError::PluginFailed(manifest.name.clone(), e.to_string()))?;

    // Write request to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(request_json.as_bytes())
            .map_err(|e| EzError::PluginFailed(manifest.name.clone(), e.to_string()))?;
    }

    // Wait with timeout
    let output = wait_with_timeout(&mut child, Duration::from_secs(timeout_secs))
        .map_err(|_| EzError::PluginTimeout(manifest.name.clone(), timeout_secs))?;

    // Log stderr at debug level
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        eprintln!("[plugin:{}] {}", manifest.name, stderr.trim());
    }

    if !output.status.success() {
        return Err(EzError::PluginFailed(
            manifest.name.clone(),
            format!("exited with status {}", output.status),
        ));
    }

    let response: HookResponse = serde_json::from_slice(&output.stdout).map_err(|e| {
        EzError::PluginFailed(
            manifest.name.clone(),
            format!("invalid JSON response: {e}"),
        )
    })?;

    if !response.success {
        if let Some(err) = &response.error {
            return Err(EzError::PluginFailed(manifest.name.clone(), err.clone()));
        }
    }

    Ok(response)
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> std::result::Result<std::process::Output, ()> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                // Process exited, collect output
                let stdout = child.stdout.take().map(|mut s| {
                    let mut buf = Vec::new();
                    std::io::Read::read_to_end(&mut s, &mut buf).ok();
                    buf
                }).unwrap_or_default();

                let stderr = child.stderr.take().map(|mut s| {
                    let mut buf = Vec::new();
                    std::io::Read::read_to_end(&mut s, &mut buf).ok();
                    buf
                }).unwrap_or_default();

                return Ok(std::process::Output {
                    status: _status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(());
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return Err(()),
        }
    }
}

/// Execute shell commands returned by a plugin.
pub fn run_shell_commands(commands: &[String]) -> Result<()> {
    for cmd in commands {
        let status = Command::new("sh")
            .args(["-c", cmd])
            .status()
            .map_err(|e| EzError::PluginFailed("shell".into(), e.to_string()))?;

        if !status.success() {
            eprintln!("Warning: shell command failed: {cmd}");
        }
    }
    Ok(())
}
