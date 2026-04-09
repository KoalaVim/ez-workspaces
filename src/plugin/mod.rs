pub mod model;
pub mod protocol;
pub mod runner;

use std::fs;

use crate::cli::PluginCommand;
use crate::config::model::EzConfig;
use crate::error::{EzError, Result};
use crate::paths;
use crate::repo::model::{RepoEntry, RepoMeta};
use crate::session::model::{Session, SessionTree};
use model::{HookType, PluginManifest};
use protocol::{HookRequest, PluginConfig, RepoInfo, SessionInfo};

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

    if !plugins_dir.exists() {
        println!("No plugins directory found at {}", plugins_dir.display());
        return Ok(());
    }

    let entries = fs::read_dir(&plugins_dir)?;
    let mut found = false;

    for entry in entries.flatten() {
        if entry.file_type()?.is_dir() {
            let manifest_path = entry.path().join("manifest.toml");
            if manifest_path.exists() {
                if let Ok(contents) = fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = toml::from_str::<PluginManifest>(&contents) {
                        let enabled = config.plugins.enabled.contains(&manifest.name);
                        let status = if enabled { "enabled" } else { "disabled" };
                        println!(
                            "{:<20} {:<10} {}",
                            manifest.name, status, manifest.description
                        );
                        found = true;
                    }
                }
            }
        }
    }

    if !found {
        println!("No plugins found in {}", plugins_dir.display());
    }
    Ok(())
}

fn enable_plugin(name: &str) -> Result<()> {
    let mut config = crate::config::load()?;
    if config.plugins.enabled.contains(&name.to_string()) {
        println!("Plugin '{name}' is already enabled.");
        return Ok(());
    }

    // Verify plugin exists
    let plugins_dir = resolve_plugins_dir(&config)?;
    let plugin_dir = plugins_dir.join(name);
    if !plugin_dir.join("manifest.toml").exists() {
        return Err(EzError::PluginNotFound(name.into()));
    }

    config.plugins.enabled.push(name.to_string());
    crate::config::save(&config)?;
    println!("Enabled plugin: {name}");
    Ok(())
}

fn disable_plugin(name: &str) -> Result<()> {
    let mut config = crate::config::load()?;
    config.plugins.enabled.retain(|n| n != name);
    crate::config::save(&config)?;
    println!("Disabled plugin: {name}");
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
    if !plugins_dir.exists() {
        return Ok(());
    }

    for plugin_name in &config.plugins.enabled {
        let plugin_dir = plugins_dir.join(plugin_name);
        let manifest_path = plugin_dir.join("manifest.toml");

        if !manifest_path.exists() {
            continue;
        }

        let manifest_contents = fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&manifest_contents)?;

        // Skip if this plugin doesn't handle this hook
        if !manifest.hooks.contains(&hook) {
            continue;
        }

        let request = build_request(&hook, repo_entry, repo_meta, session, plugin_name);

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
    };

    HookRequest {
        hook: hook.clone(),
        repo: repo_info,
        session: session_info,
        config: plugin_config,
    }
}

fn resolve_plugins_dir(config: &EzConfig) -> Result<std::path::PathBuf> {
    if let Some(dir) = &config.plugins.plugin_dir {
        Ok(dir.clone())
    } else {
        paths::plugins_dir()
    }
}
