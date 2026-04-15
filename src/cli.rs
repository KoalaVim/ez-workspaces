use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ez", about = "Workspace and session manager", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Write the target directory to this file instead of printing it (used by shell wrapper)
    #[arg(long, hide = true, global = true)]
    pub cd_file: Option<PathBuf>,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Enable debug logging (writes to a temp file, prints path on exit)
    #[arg(long, global = true)]
    pub debug: bool,

    /// Show all repos and sessions as a tree (no drill-down)
    #[arg(long)]
    pub tree: bool,

    /// Jump directly to a workspace root (skips root selection)
    #[arg(long, short)]
    pub workspace: Option<String>,

    /// Jump directly to a repo's session picker (skips root + directory selection)
    #[arg(long, short)]
    pub repo: Option<PathBuf>,

    /// Start the browser in a specific view: tree, workspace, repo, owner, label
    #[arg(long)]
    pub view: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Clone a repository and register it
    Clone {
        /// Git URL to clone
        url: String,
        /// Local path to clone into
        path: Option<PathBuf>,
    },

    /// Register an existing repository
    Add {
        /// Path to the repository (default: current directory)
        path: Option<PathBuf>,
    },

    /// Session management
    #[command(alias = "s")]
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },

    /// Repository management
    #[command(alias = "r")]
    Repo {
        #[command(subcommand)]
        command: RepoCommand,
    },

    /// Plugin management
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommand>,
    },

    /// Initialize shell integration (prints shell function to eval)
    InitShell {
        /// Shell type: bash, zsh, fish
        #[arg(default_value = "zsh")]
        shell: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Preview helper for fzf (hidden)
    #[command(hide = true)]
    Preview {
        path: PathBuf,
        /// Show session action keybinds in preview
        #[arg(long, hide = true)]
        session_actions: bool,
    },
}

#[derive(Subcommand)]
pub enum SessionCommand {
    /// Create a new session
    #[command(alias = "n")]
    New {
        /// Session name (prompted if omitted)
        name: Option<String>,
        /// Parent session name (creates a child session)
        #[arg(long, short)]
        parent: Option<String>,
        /// Repository name or path (default: current repo)
        #[arg(long, short)]
        repo: Option<String>,
    },

    /// List sessions for a repository
    #[command(alias = "ls")]
    List {
        /// Repository name or path (default: current repo)
        #[arg(long, short)]
        repo: Option<String>,
        /// Show as flat list instead of tree
        #[arg(long)]
        flat: bool,
    },

    /// Delete a session
    #[command(alias = "rm")]
    Delete {
        /// Session name
        name: String,
        /// Repository name or path (default: current repo)
        #[arg(long, short)]
        repo: Option<String>,
        /// Delete children without prompting
        #[arg(long)]
        force: bool,
    },

    /// Enter a session
    Enter {
        /// Session name
        name: String,
        /// Repository name or path (default: current repo)
        #[arg(long, short)]
        repo: Option<String>,
    },

    /// Exit the current session
    Exit,

    /// Rename a session
    Rename {
        /// Current session name
        name: String,
        /// New name
        new_name: String,
        /// Repository name or path (default: current repo)
        #[arg(long, short)]
        repo: Option<String>,
    },

    /// Manage labels on a session
    Label {
        #[command(subcommand)]
        command: SessionLabelCommand,
    },
}

#[derive(Subcommand)]
pub enum RepoCommand {
    /// List registered repositories
    #[command(alias = "ls")]
    List {
        /// Only show repos carrying this label
        #[arg(long)]
        label: Option<String>,
    },

    /// Unregister a repository
    #[command(alias = "rm")]
    Remove {
        /// Repo name or path
        name: String,
        /// Also delete all session metadata and plugin state
        #[arg(long)]
        purge: bool,
    },

    /// Manage labels on a repository
    Label {
        #[command(subcommand)]
        command: LabelCommand,
    },
}

#[derive(Subcommand)]
pub enum LabelCommand {
    /// Add one or more labels
    Add {
        /// Repo name or path
        target: String,
        /// Labels to add (space-separated)
        #[arg(required = true)]
        labels: Vec<String>,
    },

    /// Remove one or more labels
    #[command(alias = "rm")]
    Remove {
        /// Repo name or path
        target: String,
        /// Labels to remove (space-separated)
        #[arg(required = true)]
        labels: Vec<String>,
    },

    /// List labels on a target, or group entries by label when no target is given
    #[command(alias = "ls")]
    List {
        /// Optional repo name or path — omit to list all labels across repos
        target: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum SessionLabelCommand {
    /// Add one or more labels to a session
    Add {
        /// Session name
        name: String,
        /// Labels to add (space-separated)
        #[arg(required = true)]
        labels: Vec<String>,
        /// Repository name or path (default: current repo)
        #[arg(long, short)]
        repo: Option<String>,
    },

    /// Remove one or more labels from a session
    #[command(alias = "rm")]
    Remove {
        /// Session name
        name: String,
        /// Labels to remove (space-separated)
        #[arg(required = true)]
        labels: Vec<String>,
        /// Repository name or path (default: current repo)
        #[arg(long, short)]
        repo: Option<String>,
    },

    /// List labels for a session, or group sessions by label when no name is given
    #[command(alias = "ls")]
    List {
        /// Optional session name — omit to list all session labels in the repo
        name: Option<String>,
        /// Repository name or path (default: current repo)
        #[arg(long, short)]
        repo: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum PluginCommand {
    /// List available plugins
    #[command(alias = "ls")]
    List,

    /// Enable a plugin
    Enable {
        /// Plugin name
        name: String,
    },

    /// Disable a plugin
    Disable {
        /// Plugin name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Interactive guided setup
    Init,

    /// Show current configuration
    Show,

    /// Open config in editor
    Edit,

    /// Add a workspace root directory
    AddRoot {
        /// Path to add as a workspace root
        path: String,
    },

    /// Remove a workspace root directory
    RemoveRoot {
        /// Path to remove
        path: String,
    },

    /// Set a configuration value
    Set {
        /// Key to set (e.g. "selector.backend", "plugin_timeout")
        key: String,
        /// Value to set
        value: String,
    },

    /// Get a configuration value
    Get {
        /// Key to get
        key: String,
    },
}
