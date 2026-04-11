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
        None => browser::browse(cli.cd_file.as_deref(), cli.tree),
        Some(Command::Clone { url, path }) => repo::clone_repo(&url, path.as_deref()),
        Some(Command::Add { path }) => repo::add_repo(path.as_deref()),
        Some(Command::Session { command }) => session::dispatch(command, cli.cd_file.as_deref()),
        Some(Command::Repo { command }) => repo::dispatch(command),
        Some(Command::Plugin { command }) => plugin::dispatch(command),
        Some(Command::Config { command }) => config::dispatch(command),
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

fn print_shell_init(shell: &str) -> error::Result<()> {
    let func = match shell {
        "bash" | "zsh" => {
            r#"ez() {
    local tmp=$(mktemp)
    command ez "$@" --cd-file="$tmp"
    local ret=$?
    if [ -s "$tmp" ]; then
        cd "$(cat "$tmp")"
    fi
    rm -f "$tmp"
    return $ret
}"#
        }
        "fish" => {
            r#"function ez
    set tmp (mktemp)
    command ez $argv --cd-file="$tmp"
    set ret $status
    if test -s "$tmp"
        cd (cat "$tmp")
    end
    rm -f "$tmp"
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
