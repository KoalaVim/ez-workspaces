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
        log::debug!(
            "ez debug session started: {:?}",
            std::env::args().collect::<Vec<_>>()
        );
        Some(path)
    } else {
        env_logger::init();
        None
    };

    if cli.no_color {
        colored::control::set_override(false);
    }

    let result = match cli.command {
        None => browser::browse(browser::BrowseOptions {
            cd_file: cli.cd_file.as_deref(),
            post_cmd_file: cli.post_cmd_file.as_deref(),
            workspace: cli.workspace.as_deref(),
            repo_flag: cli.repo.as_deref(),
            select_by: cli.select_by.as_deref(),
            all: cli.all,
            on_enter: cli.on_enter.as_deref(),
            on_create: cli.on_create.as_deref(),
        }),
        Some(Command::Clone { url, path }) => repo::clone_repo(&url, path.as_deref()),
        Some(Command::Add { path }) => repo::add_repo(path.as_deref()),
        Some(Command::Remove { name, purge }) => {
            repo::dispatch(cli::RepoCommand::Remove { name, purge })
        }
        Some(Command::Session { command }) => session::dispatch(
            command,
            cli.cd_file.as_deref(),
            cli.post_cmd_file.as_deref(),
            cli.on_enter.as_deref(),
            cli.on_create.as_deref(),
        ),
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
            session_id,
        }) => {
            // fzf pipes preview output — force colors on (unless --no-color)
            if !cli.no_color {
                colored::control::set_override(true);
            }
            browser::preview(&path, session_actions, session_id.as_deref())
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
    // Multiplexer-agnostic: tmux user options, then the zellij session name,
    // then the working directory (see session::current::resolve_current_session).
    let target = session::current::resolve_current_session(None)?;
    let path = target
        .session
        .path
        .clone()
        .unwrap_or_else(|| target.repo_entry.path.clone());
    let path = path.display().to_string();

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
    local extra_args=()
    while true; do
        command ez "$@" "${extra_args[@]}" --cd-file="$tmp" --post-cmd-file="$post_cmd"
        local ret=$?
        extra_args=()
        if [ -s "$post_cmd" ]; then
            if [ -s "$tmp" ]; then
                extra_args=(--repo "$(cat "$tmp")")
            fi
            source "$post_cmd"
            : > "$post_cmd"
            : > "$tmp"
            continue
        fi
        break
    done
    if [ -s "$tmp" ]; then
        cd "$(cat "$tmp")"
    fi
    rm -f "$tmp" "$post_cmd"
    return $ret
}"#
        }
        "fish" => {
            r#"function ez
    set tmp (mktemp)
    set post_cmd (mktemp)
    set extra_args
    while true
        command ez $argv $extra_args --cd-file="$tmp" --post-cmd-file="$post_cmd"
        set ret $status
        set extra_args
        if test -s "$post_cmd"
            if test -s "$tmp"
                set extra_args --repo (cat "$tmp")
            end
            source "$post_cmd"
            echo -n > "$post_cmd"
            echo -n > "$tmp"
            continue
        end
        break
    end
    if test -s "$tmp"
        cd (cat "$tmp")
    end
    rm -f "$tmp" "$post_cmd"
    return $ret
end"#
        }
        "pwsh" => {
            r#"function ez {
    $tmp = Join-Path ([IO.Path]::GetTempPath()) "ez-cd-$PID-$(Get-Random)"
    $postCmd = Join-Path ([IO.Path]::GetTempPath()) "ez-post-$PID-$(Get-Random)"
    $extraArgs = @()
    while ($true) {
        & (Get-Command ez -CommandType Application) @args $extraArgs --cd-file="$tmp" --post-cmd-file="$postCmd"
        $extraArgs = @()
        if ((Test-Path $postCmd) -and (Get-Item $postCmd).Length -gt 0) {
            if ((Test-Path $tmp) -and (Get-Item $tmp).Length -gt 0) {
                $extraArgs = @('--repo', (Get-Content $tmp -Raw).Trim())
            }
            . $postCmd
            [IO.File]::WriteAllText($postCmd, '')
            [IO.File]::WriteAllText($tmp, '')
            continue
        }
        break
    }
    if ((Test-Path $tmp) -and (Get-Item $tmp).Length -gt 0) {
        $dest = (Get-Content $tmp -Raw).Trim() -replace '^\\\\\?\\', ''
        if ($dest) { Set-Location $dest }
    }
    Remove-Item $tmp, $postCmd -Force -ErrorAction SilentlyContinue
}"#
        }
        _ => {
            return Err(error::EzError::Config(format!(
                "Unsupported shell: {shell}. Supported: bash, zsh, fish, pwsh"
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
