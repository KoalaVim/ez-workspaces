# Design

## High-Level Architecture

```mermaid
graph TB
    User([User]) --> CLI[CLI / clap]
    User --> Browser[Interactive Browser]

    CLI --> Config[Config Module]
    CLI --> Repo[Repo Module]
    CLI --> Session[Session Module]
    CLI --> Plugin[Plugin Module]
    Browser --> Selector[InteractiveSelector Trait]
    Browser --> Domain[browser/domain shared layer]

    subgraph Core
        Config --> ConfigStore[(~/.config/ez/config.toml)]
        Repo --> RepoStore[(~/.config/ez/repos/index.toml)]
        Session --> SessionStore[(~/.config/ez/repos/id/sessions.toml)]
        Session --> Tree[SessionTree]
        Session --> Current[Current session resolver]
        Current --> SessionStore
        Current --> RepoStore
        Current --> |reads @ez_session_name, @ez_session_path| TmuxServer[tmux server]
    end

    subgraph Plugin System
        Plugin --> Runner[Plugin Runner]
        Runner --> |JSON stdin/stdout| GitWorktree[git-worktree plugin]
        Runner --> |JSON stdin/stdout| Tmux[tmux plugin]
        Runner --> |JSON stdin/stdout| Custom[Custom plugins...]
    end

    subgraph Interactive
        Selector --> FzfSelector[FzfSelector]
        FzfSelector --> |spawns| fzf[fzf process]
        Browser --> Repo
        Browser --> Session
        Config --> Selector
    end

    Session --> Plugin
    Repo --> Plugin
```

## Command Flow

```mermaid
sequenceDiagram
    participant U as User
    participant C as CLI
    participant R as Repo
    participant S as Session
    participant P as Plugin Runner
    participant E as External Plugin

    U->>C: ez session new feature-x
    C->>R: resolve_repo(cwd)
    R-->>C: RepoEntry
    C->>S: new_session("feature-x")
    S->>S: load sessions.toml
    opt parent is default and default checkout is not on main/master
        S->>U: Base on '<branch>'? [Y/n]
        U-->>S: yes → env ez_start_point = <branch>
    end
    S->>S: add to SessionTree
    S->>P: run_hooks(OnSessionCreate)
    P->>E: JSON request (stdin)
    E-->>P: JSON response (stdout)
    P->>S: apply session_mutations
    S->>S: save sessions.toml
    S-->>U: Created session: feature-x
```

`ez session register [path]` follows a shorter session flow: it resolves the git worktree root and common repo with `git rev-parse`, matches that repo to the registered repo index, then writes a `Session` with `path` set to the existing worktree. It does not run `OnSessionCreate`, because no worktree should be created.

## Interactive Browse Flow

```mermaid
flowchart TD
    A[ez - no args] --> B{Workspace roots configured?}
    B -->|No| C[Print setup instructions]
    B -->|Yes| D[fzf: Select workspace root]
    D --> E[fzf: Drill into directories]
    E --> F{Is git repo?}
    F -->|No| E
    F -->|Yes| G{Registered?}
    G -->|No| H[Auto-register]
    G -->|Yes| I[Load sessions]
    H --> I
    I --> J{Sessions exist?}
    J -->|No| K[Auto-create 'main' session]
    J -->|Yes| L[fzf: Select session]
    K --> L
    L --> M[Enter session: cd + plugin hooks]
```

## Browser Front Ends

`browser/domain/` holds everything that does not depend on the UI. Two front ends sit on top of it, selected by `selector.backend`.

```mermaid
graph TB
    Browse["browse() — selector.backend"] -->|fzf| FzfFE
    Browse -->|native| NativeFE

    subgraph "fzf front end (pull-style)"
        FzfFE[views/* + session_loop + drill] --> FzfSel[FzfSelector]
        FzfSel --> fzf[fzf process]
        FzfFE --> FzfEff[browser/effects.rs]
    end

    subgraph "native front end (tui/)"
        NativeFE[tui::App event loop] --> NatSel[NativeSelector modals]
        NativeFE --> NatEff[tui/effects.rs]
    end

    subgraph "browser/domain (shared)"
        Rows["rows / session_rows: Vec&lt;Row&gt;"]
        Preview["preview: String (ANSI)"]
        Actions["actions / repo_actions: Vec&lt;Effect&gt;"]
    end

    FzfFE --> Rows
    FzfFE --> Preview
    FzfFE --> Actions
    NativeFE --> Rows
    NativeFE --> Preview
    NativeFE --> Actions
    Actions -.wizards.-> Sel[InteractiveSelector trait]
    FzfSel -.implements.-> Sel
    NatSel -.implements.-> Sel
    FzfEff --> Files[(cd-file / post-cmd-file / #EZ_RELAUNCH)]
    NatEff --> Files
```

Handlers return `Effect`s (`Cd`, `PostCmds`, `Exec`, `OpenNote`, `Refresh`, `Notify`, `Exit`) instead of writing files, and each front end applies them. The shell-wrapper contract (cd-file, post-cmd-file, relaunch marker) is identical in both.

### Native event loop

```mermaid
flowchart LR
    Keys[terminal events] --> Loop
    subgraph "main thread"
        Loop["event loop (poll + mpsc)"] --> Update[App update: keys, nav stack]
        Update --> Draw[draw if dirty]
    end
    Update -->|jobs + generation id| Pool[worker pool]
    Pool -->|"rows, branches, previews, plugin views"| Loop
    Snap[(browser-snapshot.toml)] -->|first frame| Draw
    Loop -->|on exit, atomic write| Snap
    Update -->|action needs a prompt| Modal[NativeSelector nested loop on shared terminal]
    Modal --> Update
```

Stale worker results (older generation id) are dropped. Snapshot rows are replaced by fresh rows while the cursor stays on the same value.

### Stdio capture and suspend

The alternate screen must never receive stray output. `tui/stdio.rs` redirects fds 1 and 2 to a sink while the TUI is active and replays any stray output after teardown; `tui/capture.rs` collects handler output so it can be shown as a notification. Plugin binds that need the terminal (`Effect::Exec`) suspend the TUI (leave alternate screen, restore stdio), run the command, then resume; cd and post-command effects end the process so the shell wrapper can act on the files.

## Session Tree Model

```mermaid
graph TD
    subgraph "Session Tree (flat list with parent_id pointers)"
        main["main * (default)"]
        feat[feature-auth]
        api[api-tests]
        ui[frontend-ui]
        bug[bugfix-crash]

        feat --> api
        feat --> ui
    end
```

## Plugin Protocol

```mermaid
sequenceDiagram
    participant EZ as ez-workspaces
    participant P as Plugin Process

    EZ->>P: spawn executable
    EZ->>P: write JSON request to stdin
    EZ->>P: close stdin
    P->>P: process request
    P-->>EZ: JSON response on stdout
    P-->>EZ: diagnostics on stderr

    Note over EZ: Apply session_mutations
    Note over EZ: Apply repo_mutations
    Note over EZ: Execute shell_commands
    Note over EZ: Write post_shell_commands to file
    Note over EZ: Write cd_target to cd-file

    Note over EZ,P: OnNameResolve hook (e.g. GitHub PR → branch name)
    EZ->>P: JSON request with name_resolve_context (raw_url, candidate_name)
    P-->>EZ: JSON response with resolved_name
    Note over EZ: Use resolved_name as session name (fallback to candidate_name)
```

## Plugin View Flow

```mermaid
sequenceDiagram
    participant U as User
    participant B as Browser
    participant P as Plugin Process

    U->>B: Press Ctrl-a (plugin view key)
    B->>P: on_view hook
    P-->>B: view_items + view_prompt
    B->>U: fzf with plugin items
    U->>B: Select an item
    B->>P: on_view_select hook (selected_value)
    P-->>B: post_shell_commands
    B->>B: Write post-cmd-file
    B-->>U: ez exits
    Note over U: Shell wrapper sources post-cmd-file
    Note over U: e.g., tmux switch-client
```

## Module Dependencies

```mermaid
graph LR
    main --> cli
    main --> browser
    main --> config
    main --> repo
    main --> session
    main --> plugin

    browser --> selector[browser::selector]
    browser --> repo
    browser --> session

    session --> repo
    session --> plugin
    repo --> plugin

    config --> paths
    repo --> paths
    session --> paths
    plugin --> paths

    style selector fill:#f9f,stroke:#333
    style paths fill:#bbf,stroke:#333
```

## Data Storage Layout

```mermaid
graph TD
    subgraph "~/.config/ez/"
        config_toml[config.toml]
        subgraph repos/
            index[index.toml]
            subgraph "repos/&lt;id&gt;/"
                repo_toml[repo.toml]
                sessions_toml[sessions.toml]
            end
        end
        subgraph plugins/
            subgraph "git-worktree/"
                gw_manifest[manifest.toml]
                gw_exec[git-worktree-plugin]
            end
            subgraph "tmux/"
                tm_manifest[manifest.toml]
                tm_exec[tmux-plugin]
            end
        end
    end
```
