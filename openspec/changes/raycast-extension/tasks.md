## 1. Project Scaffolding

- [x] 1.1 Create `raycast/ez-extension/` directory with `package.json` manifest (extension metadata, commands, dependencies: `@raycast/api`, `@raycast/utils`)
- [x] 1.2 Create `tsconfig.json` and eslint config
- [x] 1.3 Create extension icon in `assets/extension-icon.png`

## 2. Types and Data Layer

- [x] 2.1 Create `src/types.ts` with TypeScript interfaces for `Repo` and `Session` matching the `ez` CLI JSON output
- [x] 2.2 Create `src/lib/ez.ts` with helper functions: `listRepos()`, `listSessions(repoName)`, `deleteSession(repoName, sessionName)`, `removeRepo(repoName)` — all wrapping `child_process.execSync` calls to the `ez` binary
- [x] 2.3 Create `src/lib/terminal.ts` with `runInTerminal(command)` and `openPathInTerminal(path)` functions using AppleScript via `@raycast/utils` `runAppleScript`

## 3. Browse Repos Command

- [x] 3.1 Create `src/browse-repos.tsx` with a `List` component that fetches repos via `listRepos()` and renders `List.Item` with name, path subtitle, and label accessories
- [x] 3.2 Add `List.Dropdown` searchBarAccessory with view modes: Repo (flat), Owner (grouped), Label (grouped), Workspace (grouped)
- [x] 3.3 Implement grouping logic: Owner groups by parent directory, Label groups by label tags, Workspace groups by config workspace roots
- [x] 3.4 Add repo action panel: push to SessionList (Enter), Show in Finder, Open in Terminal, Open in Cursor, Copy Path, Remove (with confirmation Alert)

## 4. Session List Component

- [x] 4.1 Create `src/components/SessionList.tsx` that accepts a repo name prop, fetches sessions via `listSessions(repoName)`, and renders the list
- [x] 4.2 Implement tree hierarchy rendering: build parent-child tree from `parent_id`, render with indentation glyphs (├──, └──) in item titles
- [x] 4.3 Add accessory metadata: LRU relative timestamp, bare/default tags, PR status tag (from `env.ez_pr_number` and `env.ez_pr_status`)
- [x] 4.4 Add session action panel: Enter Session (Terminal AppleScript), Open in Cursor, Show in Finder, Copy Path, Open PR (if `ez_pr_url` in env), Delete (with confirmation Alert)
- [x] 4.5 Hide path-dependent actions (Cursor, Finder, Copy Path) for bare sessions

## 5. Search Sessions Command

- [x] 5.1 Create `src/search-sessions.tsx` that fetches all repos then all sessions for each repo
- [x] 5.2 Render flat session list grouped by repo name using `List.Section`
- [x] 5.3 Reuse session action panel from `SessionList` component

## 6. Documentation

- [x] 6.1 Update `raycast/README.md` to document the extension as primary integration, keep script commands as fallback
- [x] 6.2 Run `npm run build` to verify the extension compiles
