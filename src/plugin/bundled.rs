//! Built-in plugins embedded at compile time.
//! Automatically extracted to the plugins directory on first use.

use std::fs;
use std::path::Path;

use crate::error::Result;

struct BundledPlugin {
    name: &'static str,
    manifest: &'static str,
    executable_name: &'static str,
    executable: &'static str,
}

const BUNDLED_PLUGINS: &[BundledPlugin] = &[
    BundledPlugin {
        name: "git-worktree",
        manifest: include_str!("../../plugins/git-worktree/manifest.toml"),
        executable_name: "git-worktree-plugin",
        executable: include_str!("../../plugins/git-worktree/git-worktree-plugin"),
    },
    BundledPlugin {
        name: "tmux",
        manifest: include_str!("../../plugins/tmux/manifest.toml"),
        executable_name: "tmux-plugin",
        executable: include_str!("../../plugins/tmux/tmux-plugin"),
    },
];

/// Ensure all bundled plugins are extracted to the plugins directory.
/// Only writes files that don't already exist (won't overwrite user edits).
pub fn ensure_bundled_plugins(plugins_dir: &Path) -> Result<()> {
    for plugin in BUNDLED_PLUGINS {
        let plugin_dir = plugins_dir.join(plugin.name);
        let manifest_path = plugin_dir.join("manifest.toml");
        let exec_path = plugin_dir.join(plugin.executable_name);

        // Skip if already exists
        if manifest_path.exists() && exec_path.exists() {
            continue;
        }

        fs::create_dir_all(&plugin_dir)?;

        if !manifest_path.exists() {
            fs::write(&manifest_path, plugin.manifest)?;
        }

        if !exec_path.exists() {
            fs::write(&exec_path, plugin.executable)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&exec_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&exec_path, perms)?;
            }
        }
    }

    Ok(())
}

/// List names of all bundled plugins.
pub fn bundled_plugin_names() -> Vec<&'static str> {
    BUNDLED_PLUGINS.iter().map(|p| p.name).collect()
}
