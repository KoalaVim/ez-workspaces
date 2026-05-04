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

    log::debug!(
        "plugin [{}]: executing {} hook={}",
        manifest.name,
        executable.display(),
        request.hook
    );
    log::debug!("plugin [{}]: request={}", manifest.name, request_json);

    let mut cmd = Command::new(&executable);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Expose ez paths so plugins can read repo/session metadata
    if let Ok(config_dir) = crate::paths::config_dir() {
        cmd.env("EZ_CONFIG_DIR", config_dir);
    }

    // In --debug mode, give each plugin invocation its own log file so the
    // plugin can write rich diagnostics without polluting stderr/stdout.
    // Plugins should append to $EZ_PLUGIN_DEBUG_LOG only when it's set.
    if std::env::var_os("EZ_DEBUG").is_some() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let log_path = std::env::temp_dir().join(format!(
            "ez-plugin-{}-{}-{}-{}.log",
            manifest.name,
            request.hook,
            std::process::id(),
            ts,
        ));
        log::debug!(
            "plugin [{}]: debug log -> {}",
            manifest.name,
            log_path.display()
        );
        cmd.env("EZ_PLUGIN_DEBUG_LOG", &log_path);
    }

    let mut child = cmd
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

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    log::debug!(
        "plugin [{}]: exit={} stdout={:?} stderr={:?}",
        manifest.name,
        output.status,
        stdout_str,
        stderr_str
    );

    if !stderr_str.is_empty() {
        eprintln!("[plugin:{}] {}", manifest.name, stderr_str.trim());
    }

    if !output.status.success() {
        let mut detail = format!("exited with status {}", output.status);
        if !stdout_str.is_empty() {
            detail.push_str(&format!("\n  stdout: {}", stdout_str.trim()));
        }
        if !stderr_str.is_empty() {
            detail.push_str(&format!("\n  stderr: {}", stderr_str.trim()));
        }
        return Err(EzError::PluginFailed(manifest.name.clone(), detail));
    }

    // Plugins may print non-JSON output (e.g. git progress) before the JSON response.
    // Extract the first valid JSON object from stdout.
    let json_str = extract_json(&stdout_str).ok_or_else(|| {
        EzError::PluginFailed(
            manifest.name.clone(),
            format!("no JSON found in stdout:\n  {}", stdout_str.trim()),
        )
    })?;

    let response: HookResponse = serde_json::from_str(json_str).map_err(|e| {
        EzError::PluginFailed(
            manifest.name.clone(),
            format!("invalid JSON response: {e}\n  raw: {json_str}"),
        )
    })?;

    log::debug!("plugin [{}]: response success={} error={:?}", manifest.name, response.success, response.error);

    if !response.success {
        if let Some(err) = &response.error {
            return Err(EzError::PluginFailed(manifest.name.clone(), err.clone()));
        }
    }

    Ok(response)
}

/// Extract the first JSON object from a string that may contain non-JSON preamble.
fn extract_json(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let bytes = s[start..].as_bytes();
    let mut depth = 0;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..start + i + 1]);
                }
            }
            _ => {}
        }
    }
    None
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
