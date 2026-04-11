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
    /// Plugin-specific per-repo state
    #[serde(default)]
    pub plugin_state: std::collections::HashMap<String, toml::Value>,
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
