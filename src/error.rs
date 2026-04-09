use thiserror::Error;

#[derive(Error, Debug)]
pub enum EzError {
    #[error("Config error: {0}")]
    Config(String),

    #[error("Repo '{0}' not found")]
    RepoNotFound(String),

    #[error("Repo '{0}' already registered")]
    RepoAlreadyRegistered(String),

    #[error("Session '{0}' not found")]
    SessionNotFound(String),

    #[error("Session '{0}' already exists in this repo")]
    SessionAlreadyExists(String),

    #[error("Session '{name}' has children: {children:?}. Use --force to delete.")]
    SessionHasChildren {
        name: String,
        children: Vec<String>,
    },

    #[error("Plugin '{0}' failed: {1}")]
    PluginFailed(String, String),

    #[error("Plugin '{0}' timed out after {1}s")]
    PluginTimeout(String, u64),

    #[error("Plugin '{0}' not found")]
    PluginNotFound(String),

    #[error("Interactive selector not available: {0}")]
    SelectorUnavailable(String),

    #[error("Aborted.")]
    Cancelled,

    #[error("Git error: {0}")]
    Git(String),

    #[error("Path error: {0}")]
    Path(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, EzError>;
