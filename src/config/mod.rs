pub mod model;

use std::fs;

use crate::error::Result;
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

/// Open config in the user's editor, or print the path.
pub fn show_or_edit(edit: bool) -> Result<()> {
    let path = paths::config_file()?;
    // Ensure config exists
    let _ = load()?;

    if edit {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
        let status = std::process::Command::new(&editor)
            .arg(&path)
            .status()?;
        if !status.success() {
            eprintln!("Editor exited with status: {}", status);
        }
    } else {
        let contents = fs::read_to_string(&path)?;
        println!("{contents}");
    }
    Ok(())
}
