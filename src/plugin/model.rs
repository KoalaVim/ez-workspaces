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
    /// Keybinds this plugin registers (actions on selected items)
    #[serde(default)]
    pub binds: Vec<PluginBind>,
    /// Views this plugin provides (full-screen fzf pickers)
    #[serde(default)]
    pub views: Vec<PluginView>,
    /// User-facing configuration schema
    #[serde(default)]
    pub config_schema: Vec<ConfigField>,
}

/// A keybind registered by a plugin for actions on the currently selected item.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginBind {
    /// Keybind string (e.g. "ctrl-a", "alt-x")
    pub key: String,
    /// Internal name for this bind (sent back to plugin in OnBind)
    pub name: String,
    /// Human-readable label shown in fzf header
    pub label: String,
    /// Which view contexts this bind is active in
    pub contexts: Vec<String>,
}

/// A full-screen view provided by a plugin.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginView {
    /// Internal name for this view
    pub name: String,
    /// Keybind to switch to this view (e.g. "ctrl-a")
    pub key: String,
    /// Human-readable label shown in view-switch header
    pub label: String,
    /// Which view contexts this view switch is available from
    #[serde(default = "all_contexts")]
    pub contexts: Vec<String>,
}

/// A user-facing configuration field declared by a plugin.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigField {
    pub name: String,
    /// Type hint: "bool", "string", "int"
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub default: Option<toml::Value>,
    #[serde(default)]
    pub description: Option<String>,
}

fn all_contexts() -> Vec<String> {
    vec![
        "session".into(),
        "repo".into(),
        "owner".into(),
        "workspace".into(),
        "tree".into(),
        "label".into(),
    ]
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
    OnBind,
    OnView,
    OnViewSelect,
}

impl std::fmt::Display for HookType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap_or_default();
        // Remove quotes from JSON string
        write!(f, "{}", s.trim_matches('"'))
    }
}
