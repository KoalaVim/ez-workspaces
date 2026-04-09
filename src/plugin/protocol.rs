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
    /// Shell commands to execute after the hook (e.g. tmux attach)
    #[serde(default)]
    pub shell_commands: Vec<String>,
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
        };
        let json = serde_json::to_string(&response).unwrap();
        let parsed: HookResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(
            parsed.session_mutations.unwrap().path.unwrap(),
            PathBuf::from("/tmp/worktree")
        );
    }
}
