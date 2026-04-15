use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Global index of all registered repos.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RepoIndex {
    #[serde(default)]
    pub repos: Vec<RepoEntry>,
}

/// A single registered repository.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RepoEntry {
    /// Unique slug derived from path (e.g. "personal-my-repo")
    pub id: String,
    /// Absolute path to repo root
    pub path: PathBuf,
    /// Display name (directory name by default)
    pub name: String,
    /// When the repo was registered
    pub registered_at: DateTime<Utc>,
}

/// Per-repo metadata stored alongside sessions.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RepoMeta {
    /// Remote URL if known
    #[serde(default)]
    pub remote_url: Option<String>,
    /// Default branch name
    #[serde(default)]
    pub default_branch: Option<String>,
    /// User-defined labels for grouping/filtering
    #[serde(default)]
    pub labels: Vec<String>,
    /// Plugin-specific per-repo state
    #[serde(default)]
    pub plugin_state: std::collections::HashMap<String, toml::Value>,
}

/// Parse the "owner" portion of a git remote URL.
///
/// Supports:
/// - `https://<host>/OWNER/repo(.git)?`
/// - `git@<host>:OWNER/repo(.git)?`
/// - `ssh://git@<host>/OWNER/repo(.git)?`
/// - `git://<host>/OWNER/repo(.git)?`
///
/// Returns `None` when no owner segment can be extracted.
pub fn parse_owner(remote_url: &str) -> Option<String> {
    let url = remote_url.trim();
    if url.is_empty() {
        return None;
    }

    // SSH shorthand: git@host:OWNER/repo(.git)?
    if let Some(after_at) = url.strip_prefix("git@") {
        if let Some((_host, path)) = after_at.split_once(':') {
            return owner_from_path(path);
        }
    }

    // https://, http://, ssh://, git://
    for scheme in ["https://", "http://", "ssh://", "git://"] {
        if let Some(rest) = url.strip_prefix(scheme) {
            // Drop optional user@ before host.
            let without_user = rest.split_once('@').map(|(_, r)| r).unwrap_or(rest);
            if let Some((_host, path)) = without_user.split_once('/') {
                return owner_from_path(path);
            }
        }
    }

    None
}

fn owner_from_path(path: &str) -> Option<String> {
    let trimmed = path.trim_start_matches('/');
    let mut parts = trimmed.splitn(2, '/');
    let owner = parts.next()?.trim();
    // Require at least an owner and a non-empty repo segment to avoid treating
    // "host/repo.git" (no owner) as a valid owner.
    let repo = parts.next()?.trim();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(owner.to_string())
}

impl RepoIndex {
    pub fn find_by_path(&self, path: &std::path::Path) -> Option<&RepoEntry> {
        self.repos.iter().find(|r| r.path == path)
    }

    pub fn find_by_name_or_id(&self, query: &str) -> Option<&RepoEntry> {
        self.repos
            .iter()
            .find(|r| r.id == query || r.name == query)
    }

    pub fn remove_by_id(&mut self, id: &str) -> Option<RepoEntry> {
        if let Some(pos) = self.repos.iter().position(|r| r.id == id) {
            Some(self.repos.remove(pos))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_owner_https() {
        assert_eq!(
            parse_owner("https://github.com/rust-lang/rust.git"),
            Some("rust-lang".to_string())
        );
        assert_eq!(
            parse_owner("https://github.com/ofirg/ez-workspaces"),
            Some("ofirg".to_string())
        );
    }

    #[test]
    fn parse_owner_ssh_shorthand() {
        assert_eq!(
            parse_owner("git@github.com:ofirg/ez-workspaces.git"),
            Some("ofirg".to_string())
        );
        assert_eq!(
            parse_owner("git@gitlab.com:group/sub/project.git"),
            Some("group".to_string())
        );
    }

    #[test]
    fn parse_owner_ssh_scheme() {
        assert_eq!(
            parse_owner("ssh://git@github.com/ofirg/ez-workspaces.git"),
            Some("ofirg".to_string())
        );
    }

    #[test]
    fn parse_owner_git_scheme() {
        assert_eq!(
            parse_owner("git://github.com/ofirg/ez-workspaces"),
            Some("ofirg".to_string())
        );
    }

    #[test]
    fn parse_owner_rejects_empty_or_invalid() {
        assert_eq!(parse_owner(""), None);
        assert_eq!(parse_owner("not-a-url"), None);
        // Missing repo segment
        assert_eq!(parse_owner("https://github.com/owner"), None);
        assert_eq!(parse_owner("git@github.com:"), None);
    }

    #[test]
    fn repo_meta_backward_compat_without_labels() {
        // Older meta.toml files don't have a `labels` field; deserialization must succeed.
        let toml_str = r#"
remote_url = "https://github.com/example/repo.git"
default_branch = "main"
"#;
        let meta: RepoMeta = toml::from_str(toml_str).expect("should parse");
        assert!(meta.labels.is_empty());
    }
}
