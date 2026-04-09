use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    /// Which hooks this plugin handles
    pub hooks: Vec<HookType>,
    /// Executable filename within the plugin directory
    pub executable: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    OnSessionCreate,
    OnSessionDelete,
    OnSessionEnter,
    OnSessionExit,
    OnSessionRename,
    OnSessionSync,
    OnRepoClone,
    OnRepoRemove,
    OnPluginInit,
    OnPluginDeinit,
}

impl std::fmt::Display for HookType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap_or_default();
        // Remove quotes from JSON string
        write!(f, "{}", s.trim_matches('"'))
    }
}
