use clap::Parser;
use colored::Colorize;

mod browser;
mod cli;
mod config;
mod error;
mod paths;
mod plugin;
mod repo;
mod session;

use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    let debug_log_path = if cli.debug {
        let path = std::env::temp_dir().join(format!("ez-debug-{}.log", std::process::id()));
        let file = std::fs::File::create(&path).expect("failed to create debug log file");
        env_logger::Builder::new()
            .filter_level(log::LevelFilter::Debug)
            .target(env_logger::Target::Pipe(Box::new(file)))
            .init();
        // Plugins read EZ_DEBUG to decide whether to emit their own debug logs.
        std::env::set_var("EZ_DEBUG", "1");
        log::debug!("ez debug session started: {:?}", std::env::args().collect::<Vec<_>>());
        Some(path)
    } else {
        env_logger::init();
        None
    };

    if cli.no_color {
        colored::control::set_override(false);
    }

    let result = match cli.command {
        None => browser::browse(
            cli.cd_file.as_deref(),
            cli.post_cmd_file.as_deref(),
            cli.workspace.as_deref(),
            cli.repo.as_deref(),
            cli.select_by.as_deref(),
        ),
        Some(Command::Clone { url, path }) => repo::clone_repo(&url, path.as_deref()),
        Some(Command::Add { path }) => repo::add_repo(path.as_deref()),
        Some(Command::Session { command }) => session::dispatch(command, cli.cd_file.as_deref()),
        Some(Command::Repo { command }) => repo::dispatch(command),
        Some(Command::Plugin { command }) => plugin::dispatch(command),
        Some(Command::Config { command }) => config::dispatch(command),
        Some(Command::CdToSession) => cd_to_session(cli.cd_file.as_deref()),
        Some(Command::InitShell { shell }) => print_shell_init(&shell),
        Some(Command::Completions { shell }) => {
            generate_completions(shell);
            Ok(())
        }
        Some(Command::Preview {
            path,
            session_actions,
        }) => {
            // fzf pipes preview output — force colors on (unless --no-color)
            if !cli.no_color {
                colored::control::set_override(true);
            }
            browser::preview(&path, session_actions)
        }
    };

    if let Some(ref log_path) = debug_log_path {
        eprintln!("{} {}", "debug log:".dimmed(), log_path.display());
    }

    if let Err(e) = result {
        if matches!(e, error::EzError::Cancelled) {
            std::process::exit(130);
        }
        eprintln!("{} {e}", "ez:".red().bold());
        std::process::exit(1);
    }
}

fn cd_to_session(cd_file: Option<&std::path::Path>) -> error::Result<()> {
    // Must be inside tmux: $TMUX is set by the server when a client is attached.
    if std::env::var_os("TMUX").is_none() {
        return Err(error::EzError::Config(
            "ez cd-to-session must be run from inside a tmux session".into(),
        ));
    }

    // Ask tmux for the @ez_session_path user option on the current session.
    // -v prints the value only; -q stays quiet if the option is unset.
    let output = std::process::Command::new("tmux")
        .args(["show-options", "-v", "-q", "@ez_session_path"])
        .output()
        .map_err(|e| error::EzError::Config(format!("failed to run tmux: {e}")))?;

    if !output.status.success() {
        return Err(error::EzError::Config(format!(
            "tmux show-options failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return Err(error::EzError::Config(
            "current tmux session has no @ez_session_path (not an ez-managed session, or session was created before this feature)".into(),
        ));
    }

    if let Some(cd_path) = cd_file {
        std::fs::write(cd_path, path.as_bytes())?;
    } else {
        println!("{path}");
    }
    Ok(())
}

fn print_shell_init(shell: &str) -> error::Result<()> {
    let func = match shell {
        "bash" | "zsh" => {
            r#"ez() {
    local tmp=$(mktemp)
    local post_cmd=$(mktemp)
    command ez "$@" --cd-file="$tmp" --post-cmd-file="$post_cmd"
    local ret=$?
    if [ -s "$tmp" ]; then
        cd "$(cat "$tmp")"
    fi
    if [ -s "$post_cmd" ]; then
        source "$post_cmd"
    fi
    rm -f "$tmp" "$post_cmd"
    return $ret
}"#
        }
        "fish" => {
            r#"function ez
    set tmp (mktemp)
    set post_cmd (mktemp)
    command ez $argv --cd-file="$tmp" --post-cmd-file="$post_cmd"
    set ret $status
    if test -s "$tmp"
        cd (cat "$tmp")
    end
    if test -s "$post_cmd"
        source "$post_cmd"
    end
    rm -f "$tmp" "$post_cmd"
    return $ret
end"#
        }
        _ => {
            return Err(error::EzError::Config(format!(
                "Unsupported shell: {shell}. Supported: bash, zsh, fish"
            )));
        }
    };
    println!("{func}");
    Ok(())
}

fn generate_completions(shell: clap_complete::Shell) {
    use clap::CommandFactory;
    let mut cmd = cli::Cli::command();
    clap_complete::generate(shell, &mut cmd, "ez", &mut std::io::stdout());
}
