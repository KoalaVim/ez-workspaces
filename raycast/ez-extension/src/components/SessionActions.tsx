import {
  Action,
  ActionPanel,
  Alert,
  Icon,
  confirmAlert,
  showToast,
  Toast,
} from "@raycast/api";
import { deleteSession } from "../lib/ez";
import { runInTerminal } from "../lib/terminal";
import { Session } from "../types";

export default function SessionActions({
  session,
  repoName,
  onDelete,
}: {
  session: Session;
  repoName: string;
  onDelete: () => void;
}) {
  const hasPath = session.path && !session.bare;
  const prUrl = session.env?.ez_pr_url;

  return (
    <ActionPanel title={session.name}>
      <Action
        title="Enter Session"
        icon={Icon.Terminal}
        onAction={async () => {
          await runInTerminal(
            `ez session enter "${session.name}" --repo "${repoName}"`,
          );
        }}
      />
      {hasPath && (
        <Action.Open
          title="Open in Cursor"
          target={session.path!}
          application="Cursor"
          icon={Icon.Code}
          shortcut={{ modifiers: ["cmd", "shift"], key: "c" }}
        />
      )}
      {hasPath && (
        <Action.ShowInFinder
          path={session.path!}
          shortcut={{ modifiers: ["cmd", "shift"], key: "f" }}
        />
      )}
      {hasPath && (
        <Action.CopyToClipboard
          title="Copy Path"
          content={session.path!}
          shortcut={{ modifiers: ["cmd", "shift"], key: "." }}
        />
      )}
      {prUrl && (
        <Action.OpenInBrowser
          title="Open PR"
          url={prUrl}
          shortcut={{ modifiers: ["cmd"], key: "p" }}
        />
      )}
      <Action
        title="Delete Session"
        icon={Icon.Trash}
        style={Action.Style.Destructive}
        shortcut={{ modifiers: ["ctrl"], key: "x" }}
        onAction={async () => {
          if (
            await confirmAlert({
              title: `Delete session "${session.name}"?`,
              message: `This will delete the session and its worktree from repo "${repoName}".`,
              primaryAction: {
                title: "Delete",
                style: Alert.ActionStyle.Destructive,
              },
            })
          ) {
            try {
              deleteSession(repoName, session.name);
              await showToast(
                Toast.Style.Success,
                `Deleted ${session.name}`,
              );
              onDelete();
            } catch (e) {
              await showToast(
                Toast.Style.Failure,
                "Failed to delete",
                String(e),
              );
            }
          }
        }}
      />
    </ActionPanel>
  );
}
