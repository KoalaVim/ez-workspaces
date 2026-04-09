pub mod model;

use std::fs;
use std::io::{self, BufRead, Write};

use crate::cli::ConfigCommand;
use crate::error::{EzError, Result};
use crate::paths;
use model::EzConfig;

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
        println!("Root already configured: {path}");
        return Ok(());
    }
    config.workspace_roots.push(path.to_string());
    save(&config)?;
    println!("Added workspace root: {path}");
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
    println!("Removed workspace root: {path}");
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
        "selector.backend" => config.selector.backend = value.to_string(),
        "selector.fzf_opts" => config.selector.fzf_opts = Some(value.to_string()),
        _ => return Err(EzError::Config(format!("Unknown key: {key}"))),
    }
    save(&config)?;
    println!("Set {key} = {value}");
    Ok(())
}

fn get_value(key: &str) -> Result<()> {
    let config = load()?;
    let value = match key {
        "workspace_roots" => format!("{:?}", config.workspace_roots),
        "default_shell" => config.default_shell.unwrap_or_default(),
        "editor" => config.editor.unwrap_or_default(),
        "plugin_timeout" => config.plugin_timeout.to_string(),
        "selector.backend" => config.selector.backend,
        "selector.fzf_opts" => config.selector.fzf_opts.unwrap_or_default(),
        "plugins.enabled" => format!("{:?}", config.plugins.enabled),
        _ => return Err(EzError::Config(format!("Unknown key: {key}"))),
    };
    println!("{value}");
    Ok(())
}

/// Interactive guided configuration.
fn interactive_init() -> Result<()> {
    let mut config = load()?;
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    println!("ez-workspaces configuration");
    println!("===========================\n");

    // Workspace roots
    println!("Workspace roots are directories that contain your repos.");
    println!("Examples: ~/workspace/personal, ~/workspace/work\n");

    if !config.workspace_roots.is_empty() {
        println!("Current roots:");
        for root in &config.workspace_roots {
            println!("  {root}");
        }
        print!("\nKeep existing roots? [Y/n]: ");
        stdout.flush()?;
        let mut answer = String::new();
        stdin.lock().read_line(&mut answer)?;
        if answer.trim().eq_ignore_ascii_case("n") {
            config.workspace_roots.clear();
        }
    }

    loop {
        print!("Add workspace root (empty to finish): ");
        stdout.flush()?;
        let mut root = String::new();
        stdin.lock().read_line(&mut root)?;
        let root = root.trim().to_string();
        if root.is_empty() {
            break;
        }
        if !config.workspace_roots.contains(&root) {
            config.workspace_roots.push(root.clone());
            println!("  Added: {root}");
        } else {
            println!("  Already configured.");
        }
    }

    // Default shell
    let current_shell = config
        .default_shell
        .as_deref()
        .unwrap_or("(auto-detect)");
    print!("\nDefault shell [{current_shell}]: ");
    stdout.flush()?;
    let mut shell = String::new();
    stdin.lock().read_line(&mut shell)?;
    let shell = shell.trim();
    if !shell.is_empty() {
        config.default_shell = Some(shell.to_string());
    }

    // Selector backend
    print!("Selector backend [{}]: ", config.selector.backend);
    stdout.flush()?;
    let mut backend = String::new();
    stdin.lock().read_line(&mut backend)?;
    let backend = backend.trim();
    if !backend.is_empty() {
        config.selector.backend = backend.to_string();
    }

    // Check fzf availability
    if config.selector.backend == "fzf" {
        if which::which("fzf").is_ok() {
            println!("  fzf found.");
        } else {
            println!("  Warning: fzf not found in PATH. Interactive mode won't work.");
            println!("  Install: https://github.com/junegunn/fzf");
        }
    }

    // Plugins
    println!("\nPlugins extend ez with git worktrees, tmux sessions, and more.");
    let plugins_dir = if let Some(dir) = &config.plugins.plugin_dir {
        dir.clone()
    } else {
        paths::plugins_dir()?
    };

    let available = discover_plugins(&plugins_dir);
    if !available.is_empty() {
        println!("Available plugins:");
        for (i, (name, desc)) in available.iter().enumerate() {
            let enabled = config.plugins.enabled.contains(name);
            let marker = if enabled { "[x]" } else { "[ ]" };
            println!("  {}) {} {} — {}", i + 1, marker, name, desc);
        }
        print!("Toggle plugins (comma-separated numbers, empty to skip): ");
        stdout.flush()?;
        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();
        if !input.is_empty() {
            for part in input.split(',') {
                if let Ok(idx) = part.trim().parse::<usize>() {
                    if idx >= 1 && idx <= available.len() {
                        let name = &available[idx - 1].0;
                        if config.plugins.enabled.contains(name) {
                            config.plugins.enabled.retain(|n| n != name);
                            println!("  Disabled: {name}");
                        } else {
                            config.plugins.enabled.push(name.clone());
                            println!("  Enabled: {name}");
                        }
                    }
                }
            }
        }
    } else {
        println!("No plugins found in {}", plugins_dir.display());
        println!("Copy bundled plugins: cp -r plugins/* {}/", plugins_dir.display());
    }

    // Plugin timeout
    print!(
        "\nPlugin timeout in seconds [{}]: ",
        config.plugin_timeout
    );
    stdout.flush()?;
    let mut timeout = String::new();
    stdin.lock().read_line(&mut timeout)?;
    let timeout = timeout.trim();
    if !timeout.is_empty() {
        if let Ok(t) = timeout.parse() {
            config.plugin_timeout = t;
        } else {
            println!("  Invalid number, keeping {}", config.plugin_timeout);
        }
    }

    // Save
    save(&config)?;
    let path = paths::config_file()?;
    println!("\nConfiguration saved to {}", path.display());

    // Summary
    println!("\nSummary:");
    println!("  Roots:    {:?}", config.workspace_roots);
    println!("  Shell:    {}", config.default_shell.as_deref().unwrap_or("auto"));
    println!("  Selector: {}", config.selector.backend);
    println!("  Plugins:  {:?}", config.plugins.enabled);
    println!("  Timeout:  {}s", config.plugin_timeout);

    // Shell integration hint
    println!("\nNext steps:");
    println!("  Add shell integration to your RC file:");
    println!("    eval \"$(ez init-shell zsh)\"");
    println!("  Then run `ez` to browse your workspaces.");

    Ok(())
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
