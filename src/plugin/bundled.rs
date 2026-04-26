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
/// Updates files when the bundled version has changed.
pub fn ensure_bundled_plugins(plugins_dir: &Path) -> Result<()> {
    for plugin in BUNDLED_PLUGINS {
        let plugin_dir = plugins_dir.join(plugin.name);
        let manifest_path = plugin_dir.join("manifest.toml");
        let exec_path = plugin_dir.join(plugin.executable_name);

        fs::create_dir_all(&plugin_dir)?;

        // Write manifest if missing or if bundled version differs
        let needs_manifest = !manifest_path.exists()
            || fs::read_to_string(&manifest_path)
                .map(|c| c != plugin.manifest)
                .unwrap_or(true);
        if needs_manifest {
            fs::write(&manifest_path, plugin.manifest)?;
        }

        // Write executable if missing or if bundled version differs
        let needs_exec = !exec_path.exists()
            || fs::read_to_string(&exec_path)
                .map(|c| c != plugin.executable)
                .unwrap_or(true);
        if needs_exec {
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

