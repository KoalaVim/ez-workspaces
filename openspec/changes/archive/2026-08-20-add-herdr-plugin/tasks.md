## 1. Add plugin files

- [x] 1.1 Copy `manifest.toml` from `~/.dotfiles/configs/ez-workspaces/plugins/herdr/manifest.toml` to `plugins/herdr/manifest.toml`
- [x] 1.2 Copy `herdr-plugin` from `~/.dotfiles/configs/ez-workspaces/plugins/herdr/herdr-plugin` to `plugins/herdr/herdr-plugin`
- [x] 1.3 Ensure `herdr-plugin` has executable permission (`chmod +x`)

## 2. Register plugin in build

- [x] 2.1 Add `herdr` to the bundled plugins list in the Rust source so it is embedded and auto-extracted alongside tmux, zellij, and other bundled plugins

## 3. Verify

- [x] 3.1 Build the project (`cargo build`) and confirm no compilation errors
- [x] 3.2 Confirm the herdr plugin is auto-extracted to the plugin directory on first run
