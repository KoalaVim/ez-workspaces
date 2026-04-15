use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EzConfig {
    /// Workspace root directories to browse (e.g. ["~/workspace/personal", "~/workspace/work"])
    #[serde(default)]
    pub workspace_roots: Vec<String>,

    /// Default shell for session enter
    #[serde(default)]
    pub default_shell: Option<String>,

    /// Editor for config editing
    #[serde(default)]
    pub editor: Option<String>,

    /// Interactive selector configuration
    #[serde(default)]
    pub selector: SelectorConfig,

    /// fzf-specific configuration
    #[serde(default)]
    pub fzf: FzfConfig,

    /// Keybindings for session actions
    #[serde(default)]
    pub keybinds: KeybindsConfig,

    /// Plugin configuration
    #[serde(default)]
    pub plugins: PluginsConfig,

    /// Plugin execution timeout in seconds
    #[serde(default = "default_plugin_timeout")]
    pub plugin_timeout: u64,

    /// Default selection type for the interactive browser (`tree`, `workspace`,
    /// `repo`, `owner`, `label`). Overridden by `--select-by` on the CLI.
    #[serde(default = "default_select_by")]
    pub default_select_by: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SelectorConfig {
    /// Selector backend: "fzf" (default)
    #[serde(default = "default_selector_backend")]
    pub backend: String,

    /// Extra fzf flags (deprecated, use [fzf] section)
    #[serde(default)]
    pub fzf_opts: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FzfConfig {
    /// fzf height (e.g. "90%", "100%", "20")
    #[serde(default = "default_fzf_height")]
    pub height: String,

    /// Extra fzf flags
    #[serde(default)]
    pub extra_opts: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeybindsConfig {
    /// Create new child session (default: "alt-n")
    #[serde(default = "default_bind_new")]
    pub new_session: String,

    /// Delete session (default: "alt-d")
    #[serde(default = "default_bind_delete")]
    pub delete_session: String,

    /// Rename session (default: "alt-r")
    #[serde(default = "default_bind_rename")]
    pub rename_session: String,

    /// Switch to tree view (default: "ctrl-t")
    #[serde(default = "default_bind_view_tree")]
    pub view_tree: String,

    /// Switch to workspace view (default: "ctrl-w")
    #[serde(default = "default_bind_view_workspace")]
    pub view_workspace: String,

    /// Switch to repo view (default: "ctrl-e"; ctrl-r reserved by fzf history)
    #[serde(default = "default_bind_view_repo")]
    pub view_repo: String,

    /// Switch to owner view (default: "ctrl-o")
    #[serde(default = "default_bind_view_owner")]
    pub view_owner: String,

    /// Switch to label view (default: "ctrl-g"; ctrl-l reserved by terminal clear)
    #[serde(default = "default_bind_view_label")]
    pub view_label: String,

    /// Edit labels on the selected item (default: "alt-l")
    #[serde(default = "default_bind_edit_labels")]
    pub edit_labels: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginsConfig {
    /// List of enabled plugin names
    #[serde(default)]
    pub enabled: Vec<String>,

    /// Override default plugin directory
    #[serde(default)]
    pub plugin_dir: Option<PathBuf>,
}

impl Default for EzConfig {
    fn default() -> Self {
        Self {
            workspace_roots: Vec::new(),
            default_shell: None,
            editor: None,
            selector: SelectorConfig::default(),
            fzf: FzfConfig::default(),
            keybinds: KeybindsConfig::default(),
            plugins: PluginsConfig::default(),
            plugin_timeout: default_plugin_timeout(),
            default_select_by: default_select_by(),
        }
    }
}

impl Default for SelectorConfig {
    fn default() -> Self {
        Self {
            backend: default_selector_backend(),
            fzf_opts: None,
        }
    }
}

impl Default for FzfConfig {
    fn default() -> Self {
        Self {
            height: default_fzf_height(),
            extra_opts: None,
        }
    }
}

impl Default for KeybindsConfig {
    fn default() -> Self {
        Self {
            new_session: default_bind_new(),
            delete_session: default_bind_delete(),
            rename_session: default_bind_rename(),
            view_tree: default_bind_view_tree(),
            view_workspace: default_bind_view_workspace(),
            view_repo: default_bind_view_repo(),
            view_owner: default_bind_view_owner(),
            view_label: default_bind_view_label(),
            edit_labels: default_bind_edit_labels(),
        }
    }
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self {
            enabled: Vec::new(),
            plugin_dir: None,
        }
    }
}

fn default_selector_backend() -> String {
    "fzf".into()
}

fn default_fzf_height() -> String {
    "90%".into()
}

fn default_bind_new() -> String {
    "alt-n".into()
}

fn default_bind_delete() -> String {
    "alt-d".into()
}

fn default_bind_rename() -> String {
    "alt-r".into()
}

fn default_bind_view_tree() -> String {
    "ctrl-t".into()
}

fn default_bind_view_workspace() -> String {
    "ctrl-w".into()
}

fn default_bind_view_repo() -> String {
    "ctrl-e".into()
}

fn default_bind_view_owner() -> String {
    "ctrl-o".into()
}

fn default_bind_view_label() -> String {
    "ctrl-g".into()
}

fn default_bind_edit_labels() -> String {
    "alt-l".into()
}

fn default_plugin_timeout() -> u64 {
    30
}

fn default_select_by() -> String {
    "workspace".into()
}
