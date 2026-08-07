use std::path::PathBuf;

use crate::error::{EzError, Result};

/// Returns the base config directory: ~/.config/ez/
pub fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir()
        .ok_or_else(|| EzError::Path("Could not determine config directory".into()))?;
    Ok(base.join("ez"))
}

/// Returns the path to the global config file: ~/.config/ez/config.toml
pub fn config_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Returns the repos index directory: ~/.config/ez/repos/
pub fn repos_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("repos"))
}

/// Returns the repos index file: ~/.config/ez/repos/index.toml
pub fn repos_index_file() -> Result<PathBuf> {
    Ok(repos_dir()?.join("index.toml"))
}

/// Returns the directory for a specific repo's metadata: ~/.config/ez/repos/<id>/
pub fn repo_meta_dir(repo_id: &str) -> Result<PathBuf> {
    Ok(repos_dir()?.join(repo_id))
}

/// Returns the repo metadata file: ~/.config/ez/repos/<id>/repo.toml
pub fn repo_meta_file(repo_id: &str) -> Result<PathBuf> {
    Ok(repo_meta_dir(repo_id)?.join("repo.toml"))
}

/// Returns the sessions file for a repo: ~/.config/ez/repos/<id>/sessions.toml
pub fn sessions_file(repo_id: &str) -> Result<PathBuf> {
    Ok(repo_meta_dir(repo_id)?.join("sessions.toml"))
}

/// Returns the plugins directory: ~/.config/ez/plugins/
pub fn plugins_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("plugins"))
}

/// Returns the base data directory: <data_dir>/ez/
/// macOS: ~/Library/Application Support/ez/
/// Linux: ~/.local/share/ez/
pub fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_dir()
        .ok_or_else(|| EzError::Path("Could not determine data directory".into()))?;
    Ok(base.join("ez"))
}

/// Returns the notes directory for a session: <data_dir>/ez/repos/<repo-id>/notes/<session-id>/
pub fn notes_dir(repo_id: &str, session_id: &str) -> Result<PathBuf> {
    Ok(data_dir()?
        .join("repos")
        .join(repo_id)
        .join("notes")
        .join(session_id))
}

/// Returns the README.md path for a session's notes.
pub fn notes_readme(repo_id: &str, session_id: &str) -> Result<PathBuf> {
    Ok(notes_dir(repo_id, session_id)?.join("README.md"))
}

/// Expand ~ to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// Resolve a path to its canonical form, following symlinks.
///
/// Registered repo paths are always canonical — registration resolves symlinks
/// before writing the index — so any path obtained from a filesystem scan must
/// be normalized before it can be compared against them.
///
/// Falls back to the input path when resolution fails (broken symlink, deleted
/// directory, unreadable parent). Never fails.
pub fn normalize(path: &std::path::Path) -> PathBuf {
    match std::fs::canonicalize(path) {
        Ok(resolved) => resolved,
        Err(e) => {
            log::debug!(
                "normalize: canonicalize failed for {}: {e}, using path as-is",
                path.display()
            );
            path.to_path_buf()
        }
    }
}

/// Collapse the home directory prefix back to ~/
pub fn collapse_tilde(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if let Some(rest) = path.strip_prefix(home_str.as_ref()) {
            return format!("~{rest}");
        }
    }
    path.to_string()
}

/// Generate a repo ID slug from a path.
/// e.g. /home/user/workspace/personal/my-repo -> personal-my-repo
pub fn repo_id_from_path(path: &std::path::Path) -> String {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();

    // Take last 2 meaningful components for a readable slug
    let slug_parts: Vec<&str> = components.iter().rev().take(2).copied().collect();
    let slug: Vec<&str> = slug_parts.into_iter().rev().collect();
    slug.join("-").to_lowercase().replace(' ', "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_repo_id_from_path() {
        let path = Path::new("/home/user/workspace/personal/my-repo");
        assert_eq!(repo_id_from_path(path), "personal-my-repo");
    }

    #[test]
    fn test_repo_id_single_component() {
        let path = Path::new("/repo");
        assert_eq!(repo_id_from_path(path), "repo");
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        let result = expand_tilde("/absolute/path");
        assert_eq!(result, PathBuf::from("/absolute/path"));
    }

    /// Temp dirs live under a symlinked prefix on macOS (`/var` -> `/private/var`),
    /// so expectations must be built from an already-canonical base.
    fn canonical_tempdir() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let base = std::fs::canonicalize(tmp.path()).expect("canonicalize tempdir");
        (tmp, base)
    }

    #[cfg(unix)]
    #[test]
    fn test_normalize_resolves_symlink() {
        let (_tmp, base) = canonical_tempdir();
        let target = base.join("target");
        std::fs::create_dir(&target).expect("create target");
        let link = base.join("link");
        std::os::unix::fs::symlink(&target, &link).expect("create symlink");

        assert_eq!(normalize(&link), target);
    }

    #[test]
    fn test_normalize_direct_path_unchanged() {
        let (_tmp, base) = canonical_tempdir();
        let dir = base.join("plain");
        std::fs::create_dir(&dir).expect("create dir");

        assert_eq!(normalize(&dir), dir);
    }

    #[test]
    fn test_normalize_missing_path_returns_input() {
        let (_tmp, base) = canonical_tempdir();
        let missing = base.join("does-not-exist");

        assert_eq!(normalize(&missing), missing);
    }
}
