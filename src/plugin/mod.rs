pub mod bundled;
pub mod model;
pub mod protocol;
pub mod runner;

use std::collections::HashMap;
use std::fs;

use colored::Colorize;

use crate::cli::PluginCommand;
use crate::config::model::EzConfig;
use crate::error::{EzError, Result};
use crate::paths;
use crate::repo::model::{RepoEntry, RepoMeta};
use crate::session::model::{Session, SessionTree};
use model::{HookType, PluginManifest};
use protocol::{HookRequest, HookResponse, PluginConfig, RepoInfo, SessionInfo, ViewContext};

/// Dispatch plugin subcommands.
pub fn dispatch(command: PluginCommand) -> Result<()> {
    match command {
        PluginCommand::List => list_plugins(),
        PluginCommand::Enable { name } => enable_plugin(&name),
        PluginCommand::Disable { name } => disable_plugin(&name),
    }
}

fn list_plugins() -> Result<()> {
    let config = crate::config::load()?;
    let plugins_dir = resolve_plugins_dir(&config)?;
    bundled::ensure_bundled_plugins(&plugins_dir)?;

    let entries = fs::read_dir(&plugins_dir)?;
    let mut found = false;

    for entry in entries.flatten() {
        if entry.file_type()?.is_dir() {
            let manifest_path = entry.path().join("manifest.toml");
            if manifest_path.exists() {
                if let Ok(contents) = fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = toml::from_str::<PluginManifest>(&contents) {
                        let enabled = config.plugins.enabled.contains(&manifest.name);
                        let status = if enabled {
                            "enabled".green().to_string()
                        } else {
                            "disabled".dimmed().to_string()
                        };
                        println!(
                            "{:<20} {:<19} {}",
                            manifest.name.cyan(), status, manifest.description
                        );
                        found = true;
                    }
                }
            }
        }
    }

    if !found {
        println!("{} {}", "No plugins found in".yellow(), plugins_dir.display());
    }
    Ok(())
}

fn enable_plugin(name: &str) -> Result<()> {
    let mut config = crate::config::load()?;
    if config.plugins.enabled.contains(&name.to_string()) {
        println!("{}", format!("Plugin '{name}' is already enabled.").yellow());
        return Ok(());
    }

    // Ensure bundled plugins are extracted
    let plugins_dir = resolve_plugins_dir(&config)?;
    bundled::ensure_bundled_plugins(&plugins_dir)?;
    let plugin_dir = plugins_dir.join(name);
    if !plugin_dir.join("manifest.toml").exists() {
        return Err(EzError::PluginNotFound(name.into()));
    }

    config.plugins.enabled.push(name.to_string());
    crate::config::save(&config)?;
    println!("{} {name}", "Enabled plugin:".green());
    Ok(())
}

fn disable_plugin(name: &str) -> Result<()> {
    let mut config = crate::config::load()?;
    config.plugins.enabled.retain(|n| n != name);
    crate::config::save(&config)?;
    println!("{} {name}", "Disabled plugin:".green());
    Ok(())
}

/// Run hooks on all enabled plugins for a given event.
pub fn run_hooks(
    hook: HookType,
    repo_entry: &RepoEntry,
    repo_meta: &RepoMeta,
    session: Option<&Session>,
    config: &EzConfig,
    tree: &mut SessionTree,
) -> Result<()> {
    let plugins_dir = resolve_plugins_dir(config)?;
    bundled::ensure_bundled_plugins(&plugins_dir)?;

    log::debug!("run_hooks: hook={:?} repo={} enabled_plugins={:?}", hook, repo_entry.name, config.plugins.enabled);

    for plugin_name in &config.plugins.enabled {
        let plugin_dir = plugins_dir.join(plugin_name);
        let manifest_path = plugin_dir.join("manifest.toml");

        if !manifest_path.exists() {
            log::debug!("plugin [{}]: manifest not found at {}", plugin_name, manifest_path.display());
            continue;
        }

        let manifest_contents = fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&manifest_contents)?;

        // Skip if this plugin doesn't handle this hook
        if !manifest.hooks.contains(&hook) {
            log::debug!("plugin [{}]: skipping, does not handle {:?}", plugin_name, hook);
            continue;
        }

        log::debug!("plugin [{}]: running {:?} hook", plugin_name, hook);

        let request = build_request(&hook, repo_entry, repo_meta, session, plugin_name, config);

        match runner::execute(&manifest, &plugin_dir, &request, config.plugin_timeout) {
            Ok(response) => {
                // Apply session mutations
                if let (Some(mutations), Some(sess)) = (&response.session_mutations, session) {
                    if let Some(s) = tree.sessions.iter_mut().find(|s| s.id == sess.id) {
                        if let Some(path) = &mutations.path {
                            s.path = Some(path.clone());
                        }
                        s.env.extend(mutations.env.clone());
                        s.plugin_state.extend(
                            mutations
                                .plugin_state
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone())),
                        );
                    }
                }

                // Execute shell commands
                if !response.shell_commands.is_empty() {
                    runner::run_shell_commands(&response.shell_commands)?;
                }
            }
            Err(e) => {
                // For enter/exit hooks, warn but don't abort
                match hook {
                    HookType::OnSessionEnter | HookType::OnSessionExit => {
                        eprintln!("Warning: {e}");
                    }
                    _ => return Err(e),
                }
            }
        }
    }

    Ok(())
}

fn build_request(
    hook: &HookType,
    repo_entry: &RepoEntry,
    repo_meta: &RepoMeta,
    session: Option<&Session>,
    plugin_name: &str,
    config: &EzConfig,
) -> HookRequest {
    let repo_info = RepoInfo {
        id: repo_entry.id.clone(),
        path: repo_entry.path.clone(),
        remote_url: repo_meta.remote_url.clone(),
        default_branch: repo_meta.default_branch.clone(),
    };

    let session_info = session.map(|s| SessionInfo {
        id: s.id.clone(),
        name: s.name.clone(),
        parent_id: s.parent_id.clone(),
        path: s.path.clone(),
        env: s.env.clone(),
        plugin_state: s.plugin_state.clone(),
        is_default: s.is_default,
    });

    let user_config = config
        .plugin_settings
        .get(plugin_name)
        .cloned()
        .unwrap_or_default();

    let plugin_config = PluginConfig {
        plugin_state: repo_meta
            .plugin_state
            .get(plugin_name)
            .and_then(|v| {
                if let toml::Value::Table(t) = v {
                    Some(
                        t.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                    )
                } else {
                    None
                }
            })
            .unwrap_or_default(),
        user_config,
    };

    HookRequest {
        hook: hook.clone(),
        repo: repo_info,
        session: session_info,
        config: plugin_config,
        bind_context: None,
        view_context: None,
    }
}

fn resolve_plugins_dir(config: &EzConfig) -> Result<std::path::PathBuf> {
    if let Some(dir) = &config.plugins.plugin_dir {
        Ok(dir.clone())
    } else {
        paths::plugins_dir()
    }
}

/// Info about a plugin view collected from manifests, used by the browser.
#[derive(Clone, Debug)]
pub struct PluginViewInfo {
    pub view_name: String,
    pub plugin_name: String,
    pub key: String,
    pub label: String,
}

/// Look up a plugin view by name across all enabled plugins.
/// Used by `--select-by` and `default_select_by` to resolve plugin view names.
pub fn find_plugin_view(name: &str, config: &EzConfig) -> Result<Option<PluginViewInfo>> {
    let plugins_dir = resolve_plugins_dir(config)?;
    bundled::ensure_bundled_plugins(&plugins_dir)?;

    for plugin_name in &config.plugins.enabled {
        let manifest_path = plugins_dir.join(plugin_name).join("manifest.toml");
        if !manifest_path.exists() {
            continue;
        }
        let manifest_contents = fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&manifest_contents)?;

        for view in &manifest.views {
            if view.name == name {
                return Ok(Some(PluginViewInfo {
                    view_name: view.name.clone(),
                    plugin_name: plugin_name.clone(),
                    key: view.key.clone(),
                    label: view.label.clone(),
                }));
            }
        }
    }

    Ok(None)
}

/// Collect all plugin views from enabled plugins for a given context.
/// Returns (key, label, plugin_name, view_name) tuples.
/// Filters out views whose key conflicts with a core keybind.
pub fn collect_plugin_views(
    context: &str,
    config: &EzConfig,
) -> Result<Vec<PluginViewInfo>> {
    let plugins_dir = resolve_plugins_dir(config)?;
    bundled::ensure_bundled_plugins(&plugins_dir)?;

    let core_keys = core_keybind_keys(config);
    let mut views = Vec::new();

    for plugin_name in &config.plugins.enabled {
        let manifest_path = plugins_dir.join(plugin_name).join("manifest.toml");
        if !manifest_path.exists() {
            continue;
        }
        let manifest_contents = fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&manifest_contents)?;

        for view in &manifest.views {
            if !view.contexts.iter().any(|c| c == context) {
                continue;
            }
            if core_keys.contains(&view.key.as_str()) {
                log::debug!(
                    "plugin [{}]: view '{}' key '{}' conflicts with core keybind, skipping",
                    plugin_name, view.name, view.key
                );
                continue;
            }
            views.push(PluginViewInfo {
                view_name: view.name.clone(),
                plugin_name: plugin_name.clone(),
                key: view.key.clone(),
                label: view.label.clone(),
            });
        }
    }

    Ok(views)
}

/// Execute a plugin's OnView hook and return the response (contains view_items).
pub fn run_view_hook(
    plugin_name: &str,
    view_name: &str,
    config: &EzConfig,
) -> Result<HookResponse> {
    let plugins_dir = resolve_plugins_dir(config)?;
    let plugin_dir = plugins_dir.join(plugin_name);
    let manifest_path = plugin_dir.join("manifest.toml");
    let manifest_contents = fs::read_to_string(&manifest_path)?;
    let manifest: PluginManifest = toml::from_str(&manifest_contents)?;

    // Build a minimal request — OnView doesn't need repo/session context.
    let user_config = config
        .plugin_settings
        .get(plugin_name)
        .cloned()
        .unwrap_or_default();

    let request = HookRequest {
        hook: HookType::OnView,
        repo: RepoInfo {
            id: String::new(),
            path: std::path::PathBuf::new(),
            remote_url: None,
            default_branch: None,
        },
        session: None,
        config: PluginConfig {
            plugin_state: HashMap::new(),
            user_config,
        },
        bind_context: None,
        view_context: Some(ViewContext {
            view_name: view_name.to_string(),
            selected_value: None,
            selected_display: None,
        }),
    };

    runner::execute(&manifest, &plugin_dir, &request, config.plugin_timeout)
}

/// Execute a plugin's OnViewSelect hook and return the response.
pub fn run_view_select_hook(
    plugin_name: &str,
    view_name: &str,
    selected_value: &str,
    selected_display: &str,
    config: &EzConfig,
) -> Result<HookResponse> {
    let plugins_dir = resolve_plugins_dir(config)?;
    let plugin_dir = plugins_dir.join(plugin_name);
    let manifest_path = plugin_dir.join("manifest.toml");
    let manifest_contents = fs::read_to_string(&manifest_path)?;
    let manifest: PluginManifest = toml::from_str(&manifest_contents)?;

    let user_config = config
        .plugin_settings
        .get(plugin_name)
        .cloned()
        .unwrap_or_default();

    let request = HookRequest {
        hook: HookType::OnViewSelect,
        repo: RepoInfo {
            id: String::new(),
            path: std::path::PathBuf::new(),
            remote_url: None,
            default_branch: None,
        },
        session: None,
        config: PluginConfig {
            plugin_state: HashMap::new(),
            user_config,
        },
        bind_context: None,
        view_context: Some(ViewContext {
            view_name: view_name.to_string(),
            selected_value: Some(selected_value.to_string()),
            selected_display: Some(selected_display.to_string()),
        }),
    };

    runner::execute(&manifest, &plugin_dir, &request, config.plugin_timeout)
}

/// Info about a plugin action bind collected from manifests.
#[derive(Clone, Debug)]
pub struct PluginBindInfo {
    pub bind_name: String,
    pub plugin_name: String,
    pub key: String,
    pub label: String,
}

/// Collect all plugin action binds from enabled plugins for a given context.
/// Filters out binds whose key conflicts with a core keybind.
pub fn collect_plugin_binds(
    context: &str,
    config: &EzConfig,
) -> Result<Vec<PluginBindInfo>> {
    let plugins_dir = resolve_plugins_dir(config)?;
    bundled::ensure_bundled_plugins(&plugins_dir)?;

    let core_keys = core_keybind_keys(config);
    let mut binds = Vec::new();

    for plugin_name in &config.plugins.enabled {
        let manifest_path = plugins_dir.join(plugin_name).join("manifest.toml");
        if !manifest_path.exists() {
            continue;
        }
        let manifest_contents = fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&manifest_contents)?;

        for bind in &manifest.binds {
            if !bind.contexts.iter().any(|c| c == context) {
                continue;
            }
            if core_keys.contains(&bind.key.as_str()) {
                log::debug!(
                    "plugin [{}]: bind '{}' key '{}' conflicts with core keybind, skipping",
                    plugin_name, bind.name, bind.key
                );
                continue;
            }
            binds.push(PluginBindInfo {
                bind_name: bind.name.clone(),
                plugin_name: plugin_name.clone(),
                key: bind.key.clone(),
                label: bind.label.clone(),
            });
        }
    }

    Ok(binds)
}

/// Execute a plugin's OnBind hook and return the response.
pub fn run_bind_hook(
    plugin_name: &str,
    bind_name: &str,
    key: &str,
    view: &str,
    selected_value: &str,
    selected_display: &str,
    repo_entry: &RepoEntry,
    session: Option<&Session>,
    config: &EzConfig,
) -> Result<HookResponse> {
    let plugins_dir = resolve_plugins_dir(config)?;
    let plugin_dir = plugins_dir.join(plugin_name);
    let manifest_path = plugin_dir.join("manifest.toml");
    let manifest_contents = fs::read_to_string(&manifest_path)?;
    let manifest: PluginManifest = toml::from_str(&manifest_contents)?;

    let repo_meta = crate::repo::store::load_repo_meta(&repo_entry.id).unwrap_or_default();
    let mut request = build_request(
        &HookType::OnBind,
        repo_entry,
        &repo_meta,
        session,
        plugin_name,
        config,
    );
    request.bind_context = Some(protocol::BindContext {
        name: bind_name.to_string(),
        key: key.to_string(),
        view: view.to_string(),
        selected_value: selected_value.to_string(),
        selected_display: selected_display.to_string(),
    });

    runner::execute(&manifest, &plugin_dir, &request, config.plugin_timeout)
}

/// Collect all core keybind keys for conflict detection.
fn core_keybind_keys(config: &EzConfig) -> Vec<&str> {
    let kb = &config.keybinds;
    vec![
        kb.view_tree.as_str(),
        kb.view_workspace.as_str(),
        kb.view_repo.as_str(),
        kb.view_owner.as_str(),
        kb.view_label.as_str(),
        kb.new_session.as_str(),
        kb.delete_session.as_str(),
        kb.rename_session.as_str(),
        kb.edit_labels.as_str(),
    ]
}
