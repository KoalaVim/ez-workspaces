use std::process::Command;

use colored::Colorize;

use crate::config::model::EzConfig;
use crate::error::{EzError, Result};
use crate::paths;

/// Create the notes directory and empty README.md for a session if they don't exist.
pub fn ensure_notes_dir(repo_id: &str, session_id: &str) -> Result<std::path::PathBuf> {
    let dir = paths::notes_dir(repo_id, session_id)?;
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
        log::debug!("ensure_notes_dir: created {}", dir.display());
    }
    let readme = dir.join("README.md");
    if !readme.exists() {
        std::fs::write(&readme, "")?;
        log::debug!("ensure_notes_dir: created {}", readme.display());
    }
    Ok(dir)
}

/// Resolve the command used to open note files.
/// Returns the command string or an error if $EDITOR is not set.
pub fn resolve_note_open_command(config: &EzConfig) -> Result<String> {
    let raw = &config.note_open_command;
    if raw == "$EDITOR" {
        match std::env::var("EDITOR") {
            Ok(editor) if !editor.is_empty() => Ok(editor),
            _ => Err(EzError::Config(
                "$EDITOR is not set. Set it or configure note_open_command in config.".into(),
            )),
        }
    } else {
        Ok(raw.clone())
    }
}

/// Open a session's README.md in the configured editor (blocking).
pub fn open_note(repo_id: &str, session_id: &str, config: &EzConfig) -> Result<()> {
    let command = resolve_note_open_command(config)?;
    let dir = ensure_notes_dir(repo_id, session_id)?;
    let readme = dir.join("README.md");

    log::debug!("open_note: opening {} with '{}'", readme.display(), command);

    let parts: Vec<&str> = command.split_whitespace().collect();
    let (cmd, args) = parts
        .split_first()
        .ok_or_else(|| EzError::Config("note_open_command is empty".into()))?;

    let status = Command::new(cmd)
        .args(args)
        .arg(&readme)
        .status()
        .map_err(|e| EzError::Config(format!("failed to run '{}': {}", command, e)))?;

    if !status.success() {
        eprintln!(
            "{}",
            format!("Editor exited with status {}", status).yellow()
        );
    }
    Ok(())
}

/// Delete the notes directory for a session (no-op if it doesn't exist).
pub fn delete_notes_dir(repo_id: &str, session_id: &str) -> Result<()> {
    let dir = paths::notes_dir(repo_id, session_id)?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
        log::debug!("delete_notes_dir: removed {}", dir.display());
    }
    Ok(())
}

/// Check whether a session has a notes README.md.
pub fn notes_readme_exists(repo_id: &str, session_id: &str) -> bool {
    paths::notes_readme(repo_id, session_id)
        .map(|p| p.exists())
        .unwrap_or(false)
}
