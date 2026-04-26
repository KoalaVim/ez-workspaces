use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::model::HookType;

/// Request sent to a plugin on stdin as JSON.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HookRequest {
    pub hook: HookType,
    pub repo: RepoInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionInfo>,
    pub config: PluginConfig,
    /// Present for OnBind hooks — which bind was pressed and the selection context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_context: Option<BindContext>,
    /// Present for OnView / OnViewSelect hooks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_context: Option<ViewContext>,
}

/// Context passed to a plugin when one of its registered binds is pressed.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BindContext {
    /// The internal bind name (from manifest)
    pub name: String,
    /// Which key was pressed
    pub key: String,
    /// Which view the user was in (e.g. "session", "repo", "tree")
    pub view: String,
    /// The value field of the selected item
    pub selected_value: String,
    /// The display field of the selected item
    pub selected_display: String,
}

/// Context passed to a plugin for view-related hooks.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ViewContext {
    /// The plugin view name (from manifest)
    pub view_name: String,
    /// For OnViewSelect: the selected item's value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_value: Option<String>,
    /// For OnViewSelect: the selected item's display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_display: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RepoInfo {
    pub id: String,
    pub path: PathBuf,
    pub remote_url: Option<String>,
    pub default_branch: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub path: Option<PathBuf>,
    pub env: HashMap<String, String>,
    pub plugin_state: HashMap<String, toml::Value>,
    pub is_default: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginConfig {
    /// This plugin's per-repo state from previous invocations
    #[serde(default)]
    pub plugin_state: HashMap<String, toml::Value>,
    /// User-facing settings from [plugin_settings.<name>] in config
    #[serde(default)]
    pub user_config: HashMap<String, toml::Value>,
}

/// Response returned by a plugin on stdout as JSON.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HookResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_mutations: Option<SessionMutations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_mutations: Option<RepoMutations>,
    /// Shell commands to execute inside the ez process (before exit)
    #[serde(default)]
    pub shell_commands: Vec<String>,
    /// Shell commands to execute in the user's shell AFTER ez exits.
    /// Written to the post-cmd-file and sourced by the shell wrapper.
    /// Use for commands that need the user's terminal (e.g. tmux switch-client).
    #[serde(default)]
    pub post_shell_commands: Vec<String>,
    /// Override the cd target written to --cd-file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cd_target: Option<PathBuf>,
    /// Items for a plugin view (returned from OnView hook).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_items: Option<Vec<ViewItem>>,
    /// Prompt string for plugin view's fzf (returned from OnView hook).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_prompt: Option<String>,
    /// Preview command for plugin view's fzf (returned from OnView hook).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_preview_cmd: Option<String>,
}

/// An item provided by a plugin for its view.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ViewItem {
    /// What the user sees in fzf
    pub display: String,
    /// Internal identifier (passed back on selection)
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SessionMutations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub plugin_state: HashMap<String, toml::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RepoMutations {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub plugin_state: HashMap<String, toml::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_response_roundtrip() {
        let response = HookResponse {
            success: true,
            error: None,
            session_mutations: Some(SessionMutations {
                path: Some(PathBuf::from("/tmp/worktree")),
                env: HashMap::from([("KEY".into(), "val".into())]),
                plugin_state: HashMap::new(),
            }),
            repo_mutations: None,
            shell_commands: vec!["tmux attach".into()],
            post_shell_commands: vec!["tmux switch-client -t foo".into()],
            cd_target: None,
            view_items: None,
            view_prompt: None,
            view_preview_cmd: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        let parsed: HookResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(
            parsed.session_mutations.unwrap().path.unwrap(),
            PathBuf::from("/tmp/worktree")
        );
        assert_eq!(parsed.post_shell_commands, vec!["tmux switch-client -t foo"]);
    }

    #[test]
    fn test_hook_response_defaults_from_minimal_json() {
        let json = r#"{"success": true}"#;
        let parsed: HookResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.success);
        assert!(parsed.shell_commands.is_empty());
        assert!(parsed.post_shell_commands.is_empty());
        assert!(parsed.view_items.is_none());
        assert!(parsed.cd_target.is_none());
    }
}
