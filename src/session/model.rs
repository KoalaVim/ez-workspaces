use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type SessionId = String;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Session {
    /// Unique session ID (UUID)
    pub id: SessionId,
    /// User-facing name
    pub name: String,
    /// Parent session ID (None = root-level session)
    #[serde(default)]
    pub parent_id: Option<SessionId>,
    /// Optional physical path (set by plugins, e.g. worktree path)
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// Session-specific environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Per-plugin state for this session
    #[serde(default)]
    pub plugin_state: HashMap<String, toml::Value>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// Whether this is the auto-created default session
    #[serde(default)]
    pub is_default: bool,
}

/// The full session collection for one repo.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SessionTree {
    #[serde(default)]
    pub sessions: Vec<Session>,
}
