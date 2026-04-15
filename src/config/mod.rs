pub mod model;

use std::fs;

use colored::Colorize;

use crate::browser::selector::{FzfSelector, InteractiveSelector, SelectItem};
use crate::cli::ConfigCommand;
use crate::error::{EzError, Result};
use crate::paths;
use model::EzConfig;

/// Helper: select_one that returns Cancelled on None (Ctrl+C / Escape).
fn require_select(
    selector: &dyn InteractiveSelector,
    items: &[SelectItem],
    prompt: &str,
    preview_cmd: Option<&str>,
) -> Result<usize> {
    selector
        .select_one(items, prompt, preview_cmd)?
        .ok_or(EzError::Cancelled)
}

/// Load the global config, creating a default one if it doesn't exist.
pub fn load() -> Result<EzConfig> {
    let path = paths::config_file()?;
    if !path.exists() {
        let config = EzConfig::default();
        save(&config)?;
        return Ok(config);
    }
    let contents = fs::read_to_string(&path)?;
    let config: EzConfig = toml::from_str(&contents)?;
    Ok(config)
}

/// Save the global config to disk.
pub fn save(config: &EzConfig) -> Result<()> {
    let path = paths::config_file()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(config)?;
    fs::write(&path, contents)?;
    Ok(())
}

/// Dispatch config subcommands.
pub fn dispatch(command: Option<ConfigCommand>) -> Result<()> {
    match command {
        None | Some(ConfigCommand::Init) => interactive_init(),
        Some(ConfigCommand::Show) => show(),
        Some(ConfigCommand::Edit) => edit(),
        Some(ConfigCommand::AddRoot { path }) => add_root(&path),
        Some(ConfigCommand::RemoveRoot { path }) => remove_root(&path),
        Some(ConfigCommand::Set { key, value }) => set_value(&key, &value),
        Some(ConfigCommand::Get { key }) => get_value(&key),
    }
}

fn show() -> Result<()> {
    let path = paths::config_file()?;
    let _ = load()?;
    let contents = fs::read_to_string(&path)?;
    println!("{contents}");
    Ok(())
}

fn edit() -> Result<()> {
    let path = paths::config_file()?;
    let _ = load()?;
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()?;
    if !status.success() {
        eprintln!("Editor exited with status: {status}");
    }
    Ok(())
}

fn add_root(path: &str) -> Result<()> {
    let mut config = load()?;
    if config.workspace_roots.contains(&path.to_string()) {
        println!("{} {path}", "Already configured:".yellow());
        return Ok(());
    }
    config.workspace_roots.push(path.to_string());
    save(&config)?;
    println!("{} {path}", "Added root:".green());
    Ok(())
}

fn remove_root(path: &str) -> Result<()> {
    let mut config = load()?;
    let before = config.workspace_roots.len();
    config.workspace_roots.retain(|r| r != path);
    if config.workspace_roots.len() == before {
        return Err(EzError::Config(format!("Root not found: {path}")));
    }
    save(&config)?;
    println!("{} {path}", "Removed root:".green());
    Ok(())
}

fn set_value(key: &str, value: &str) -> Result<()> {
    let mut config = load()?;
    match key {
        "default_shell" => config.default_shell = Some(value.to_string()),
        "editor" => config.editor = Some(value.to_string()),
        "plugin_timeout" => {
            config.plugin_timeout = value.parse().map_err(|_| {
                EzError::Config(format!("Invalid number: {value}"))
            })?;
        }
        "default_select_by" => {
            // Validate early so we never persist an unusable value.
            crate::browser::views::ViewMode::from_flag(value)?;
            config.default_select_by = value.to_string();
        }
        "selector.backend" => config.selector.backend = value.to_string(),
        "selector.fzf_opts" => config.selector.fzf_opts = Some(value.to_string()),
        _ => return Err(EzError::Config(format!("Unknown key: {key}"))),
    }
    save(&config)?;
    println!("{} {} = {}", "Set".green(), key.bold(), value);
    Ok(())
}

fn get_value(key: &str) -> Result<()> {
    let config = load()?;
    let value = match key {
        "workspace_roots" => format!("{:?}", config.workspace_roots),
        "default_shell" => config.default_shell.unwrap_or_default(),
        "editor" => config.editor.unwrap_or_default(),
        "plugin_timeout" => config.plugin_timeout.to_string(),
        "default_select_by" => config.default_select_by,
        "selector.backend" => config.selector.backend,
        "selector.fzf_opts" => config.selector.fzf_opts.unwrap_or_default(),
        "plugins.enabled" => format!("{:?}", config.plugins.enabled),
        _ => return Err(EzError::Config(format!("Unknown key: {key}"))),
    };
    println!("{value}");
    Ok(())
}

/// Interactive guided configuration using the InteractiveSelector.
fn interactive_init() -> Result<()> {
    let mut config = load()?;
    let selector = FzfSelector::new(&config.fzf)?;

    println!("{}\n", "ez-workspaces configuration".bold().cyan());

    // Workspace roots — keep or clear existing
    if !config.workspace_roots.is_empty() {
        let keep = selector.confirm(
            &format!("Keep existing roots? ({})", config.workspace_roots.join(", ")),
            true,
        )?;
        if !keep {
            config.workspace_roots.clear();
        }
    }

    // Add workspace roots by browsing directories interactively
    let mut last_root: Option<std::path::PathBuf> = config
        .workspace_roots
        .last()
        .map(|r| std::path::PathBuf::from(r));
    loop {
        let action_items = vec![
            SelectItem {
                display: "Browse for a directory".into(),
                value: "browse".into(),
            },
            SelectItem {
                display: "Type a path manually".into(),
                value: "type".into(),
            },
            SelectItem {
                display: "Done adding roots".into(),
                value: "done".into(),
            },
        ];

        let current_roots = if config.workspace_roots.is_empty() {
            "none".to_string()
        } else {
            config.workspace_roots.join(", ")
        };
        println!("{} {current_roots}", "Current roots:".bold());

        let action_idx = match selector.select_one(&action_items, "add root", None)? {
            Some(idx) => idx,
            None => break, // Escape means done adding roots
        };

        match action_items[action_idx].value.as_str() {
            "browse" => {
                let path = browse_for_directory(&selector, last_root.as_deref())?;
                let path_str = path.to_string_lossy().to_string();
                if !config.workspace_roots.contains(&path_str) {
                    config.workspace_roots.push(path_str.clone());
                    println!("  {} {path_str}", "Added:".green());
                } else {
                    println!("  {}", "Already configured.".yellow());
                }
                last_root = Some(path);
            }
            "type" => {
                let root = selector.input("Workspace root path", None)?;
                if !root.is_empty() && !config.workspace_roots.contains(&root) {
                    config.workspace_roots.push(root.clone());
                    println!("  {} {root}", "Added:".green());
                }
            }
            _ => break,
        }
    }

    // Default shell
    let shells = &["zsh", "bash", "fish"];
    let shell_items: Vec<SelectItem> = shells
        .iter()
        .map(|s| SelectItem {
            display: s.to_string(),
            value: s.to_string(),
        })
        .collect();
    let current_shell = config.default_shell.as_deref().unwrap_or("zsh");
    println!("\n{} {current_shell}", "Current shell:".bold());
    let idx = require_select(&selector, &shell_items, "default shell", None)?;
    config.default_shell = Some(shell_items[idx].value.clone());

    // Selector backend
    let backend_items = vec![
        SelectItem {
            display: "fzf".into(),
            value: "fzf".into(),
        },
    ];
    println!("\n{} {}", "Current selector:".bold(), config.selector.backend);
    let idx = require_select(&selector, &backend_items, "selector backend", None)?;
    config.selector.backend = backend_items[idx].value.clone();

    // fzf check
    if config.selector.backend == "fzf" && which::which("fzf").is_err() {
        println!("  {}", "Warning: fzf not found in PATH.".yellow());
    }

    // Plugins — multi-select from available
    let plugins_dir = config
        .plugins
        .plugin_dir
        .clone()
        .map_or_else(|| paths::plugins_dir(), Ok)?;

    let available = discover_plugins(&plugins_dir);
    if !available.is_empty() {
        let plugin_items: Vec<SelectItem> = available
            .iter()
            .map(|(name, desc)| {
                let enabled = config.plugins.enabled.contains(name);
                let marker = if enabled { "[x]" } else { "[ ]" };
                SelectItem {
                    display: format!("{marker} {name} — {desc}"),
                    value: name.clone(),
                }
            })
            .collect();

        println!("\nSelect plugins to enable (Tab to toggle, Enter to confirm):");
        let selected = selector.select_many(&plugin_items, "plugins")?;

        // Replace enabled list with selection
        config.plugins.enabled = selected
            .iter()
            .map(|&idx| plugin_items[idx].value.clone())
            .collect();
    } else {
        println!("\n{} {}", "No plugins found in".yellow(), plugins_dir.display());
        println!("Copy bundled plugins: cp -r plugins/* {}/", plugins_dir.display());
    }

    // Plugin timeout
    let timeout_str = selector.input(
        "Plugin timeout (seconds)",
        Some(&config.plugin_timeout.to_string()),
    )?;
    if let Ok(t) = timeout_str.parse::<u64>() {
        config.plugin_timeout = t;
    }

    // Save
    save(&config)?;
    let path = paths::config_file()?;
    println!("\n{} {}\n", "Configuration saved to".green(), path.display());

    println!("{}", "Summary:".bold().cyan());
    println!("  {} {:?}", "Roots:   ".bold(), config.workspace_roots);
    println!("  {} {}", "Shell:   ".bold(), config.default_shell.as_deref().unwrap_or("auto"));
    println!("  {} {}", "Selector:".bold(), config.selector.backend);
    println!("  {} {:?}", "Plugins: ".bold(), config.plugins.enabled);
    println!("  {} {}s", "Timeout: ".bold(), config.plugin_timeout);

    println!("\n{}", "Next steps:".bold().cyan());
    println!("  eval \"$(ez init-shell {})\"", config.default_shell.as_deref().unwrap_or("zsh"));
    println!("  ez");

    Ok(())
}

/// Browse directories interactively starting from home.
/// Returns the selected directory path, or Cancelled on Ctrl+C.
fn browse_for_directory(
    selector: &dyn InteractiveSelector,
    start_from: Option<&std::path::Path>,
) -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
    let mut current = start_from.map(|p| p.to_path_buf()).unwrap_or(home);

    loop {
        let mut entries: Vec<(String, std::path::PathBuf)> = Vec::new();

        // Add "select this directory" option
        entries.push((
            format!(">>> Use this directory: {}", current.display()),
            current.clone(),
        ));

        // Add parent directory option
        if let Some(parent) = current.parent() {
            entries.push(("..".to_string(), parent.to_path_buf()));
        }

        // List subdirectories
        if let Ok(read_dir) = fs::read_dir(&current) {
            let mut subdirs: Vec<(String, std::path::PathBuf)> = read_dir
                .flatten()
                .filter(|e| {
                    e.path().is_dir()
                        && !e
                            .file_name()
                            .to_string_lossy()
                            .starts_with('.')
                })
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    (format!("{name}/"), e.path())
                })
                .collect();
            subdirs.sort_by(|a, b| a.0.cmp(&b.0));
            entries.extend(subdirs);
        }

        let items: Vec<SelectItem> = entries
            .iter()
            .map(|(display, path)| SelectItem {
                display: display.clone(),
                value: path.to_string_lossy().to_string(),
            })
            .collect();

        let prompt = current.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| current.to_string_lossy().to_string());

        let idx = match selector.select_one(&items, &prompt, None)? {
            Some(idx) => idx,
            None => {
                // Escape: go back to parent directory
                if let Some(parent) = current.parent() {
                    current = parent.to_path_buf();
                    continue;
                }
                return Err(EzError::Cancelled);
            }
        };

        if idx == 0 {
            // Selected "use this directory"
            return Ok(current);
        }

        current = entries[idx].1.clone();
    }
}

/// Discover available plugins by scanning the plugins directory.
fn discover_plugins(plugins_dir: &std::path::Path) -> Vec<(String, String)> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(plugins_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map_or(false, |t| t.is_dir()) {
                let manifest_path = entry.path().join("manifest.toml");
                if let Ok(contents) = fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) =
                        toml::from_str::<crate::plugin::model::PluginManifest>(&contents)
                    {
                        result.push((manifest.name, manifest.description));
                    }
                }
            }
        }
    }
    result.sort();
    result
}
