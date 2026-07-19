import {
  Action,
  ActionPanel,
  Alert,
  Color,
  Icon,
  List,
  confirmAlert,
  showToast,
  Toast,
} from "@raycast/api";
import { useCachedPromise } from "@raycast/utils";
import { useState } from "react";
import { listRepos, removeRepo } from "./lib/ez";
import { openPathInTerminal } from "./lib/terminal";
import { groupRepos, ViewMode } from "./lib/grouping";
import { Repo } from "./types";
import SessionList from "./components/SessionList";

export default function BrowseRepos() {
  const [viewMode, setViewMode] = useState<ViewMode>("repo");
  const { data: repos, isLoading, revalidate } = useCachedPromise(listRepos);

  const groups = repos ? groupRepos(repos, viewMode) : [];

  return (
    <List
      isLoading={isLoading}
      searchBarPlaceholder="Search repos..."
      searchBarAccessory={
        <List.Dropdown
          tooltip="View Mode"
          storeValue
          onChange={(v) => setViewMode(v as ViewMode)}
        >
          <List.Dropdown.Item title="Repo" value="repo" />
          <List.Dropdown.Item title="Owner" value="owner" />
          <List.Dropdown.Item title="Label" value="label" />
          <List.Dropdown.Item title="Workspace" value="workspace" />
        </List.Dropdown>
      }
    >
      {groups.map((group) => {
        const items = group.repos.map((repo) => (
          <RepoItem
            key={repo.id}
            repo={repo}
            onRemove={revalidate}
          />
        ));

        if (viewMode === "repo") {
          return items;
        }

        return (
          <List.Section key={group.title} title={group.title}>
            {items}
          </List.Section>
        );
      })}
    </List>
  );
}

function RepoItem({ repo, onRemove }: { repo: Repo; onRemove: () => void }) {
  const accessories: List.Item.Accessory[] = [];
  if (!repo.is_git) {
    accessories.push({ tag: { value: "non-git", color: Color.Orange } });
  }
  for (const label of repo.labels) {
    accessories.push({ tag: { value: label, color: Color.Purple } });
  }

  return (
    <List.Item
      title={repo.name}
      subtitle={repo.path}
      icon={repo.is_git ? Icon.Folder : Icon.Document}
      accessories={accessories}
      actions={
        <ActionPanel title={repo.name}>
          <Action.Push
            title="Browse Sessions"
            icon={Icon.List}
            target={<SessionList repoName={repo.name} />}
          />
          <Action.ShowInFinder path={repo.path} />
          <Action
            title="Open in Terminal"
            icon={Icon.Terminal}
            shortcut={{ modifiers: ["cmd"], key: "t" }}
            onAction={async () => {
              await openPathInTerminal(repo.path);
            }}
          />
          <Action.Open
            title="Open in Cursor"
            target={repo.path}
            application="Cursor"
            icon={Icon.Code}
            shortcut={{ modifiers: ["cmd", "shift"], key: "c" }}
          />
          <Action.CopyToClipboard
            title="Copy Path"
            content={repo.path}
            shortcut={{ modifiers: ["cmd", "shift"], key: "." }}
          />
          <Action
            title="Remove Repo"
            icon={Icon.Trash}
            style={Action.Style.Destructive}
            shortcut={{ modifiers: ["ctrl"], key: "x" }}
            onAction={async () => {
              if (
                await confirmAlert({
                  title: `Remove "${repo.name}"?`,
                  message:
                    "This will unregister the repo from ez-workspaces. The directory will not be deleted.",
                  primaryAction: {
                    title: "Remove",
                    style: Alert.ActionStyle.Destructive,
                  },
                })
              ) {
                try {
                  removeRepo(repo.name);
                  await showToast(Toast.Style.Success, `Removed ${repo.name}`);
                  onRemove();
                } catch (e) {
                  await showToast(
                    Toast.Style.Failure,
                    "Failed to remove",
                    String(e),
                  );
                }
              }
            }}
          />
        </ActionPanel>
      }
    />
  );
}
