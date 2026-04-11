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

    /// Plugin configuration
    #[serde(default)]
    pub plugins: PluginsConfig,

    /// Plugin execution timeout in seconds
    #[serde(default = "default_plugin_timeout")]
    pub plugin_timeout: u64,
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
            plugins: PluginsConfig::default(),
            plugin_timeout: default_plugin_timeout(),
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

fn default_plugin_timeout() -> u64 {
    30
}
