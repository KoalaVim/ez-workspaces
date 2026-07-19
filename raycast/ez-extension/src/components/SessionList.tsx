import { Color, Icon, List } from "@raycast/api";
import { useCachedPromise } from "@raycast/utils";
import { listSessions } from "../lib/ez";
import { buildTree, formatRelativeTime } from "../lib/session-tree";
import SessionActions from "./SessionActions";

export default function SessionList({ repoName }: { repoName: string }) {
  const {
    data: sessions,
    isLoading,
    revalidate,
  } = useCachedPromise(listSessions, [repoName]);

  const treeItems = sessions ? buildTree(sessions) : [];

  return (
    <List
      isLoading={isLoading}
      navigationTitle={`Sessions — ${repoName}`}
      searchBarPlaceholder="Search sessions..."
    >
      {treeItems.length === 0 && !isLoading ? (
        <List.EmptyView
          title="No Sessions"
          description={`No sessions for "${repoName}". Use 'ez session new' to create one.`}
          icon={Icon.Leaf}
        />
      ) : (
        treeItems.map((item) => {
          const { session, prefix } = item;
          const accessories: List.Item.Accessory[] = [];

          if (session.last_accessed) {
            accessories.push({
              text: formatRelativeTime(session.last_accessed),
              tooltip: `Last accessed: ${session.last_accessed}`,
            });
          }

          if (session.is_default) {
            accessories.push({
              tag: { value: "default", color: Color.Green },
            });
          }
          if (session.bare) {
            accessories.push({
              tag: { value: "bare", color: Color.Orange },
            });
          }

          const prNumber = session.env?.ez_pr_number;
          const prStatus = session.env?.ez_pr_status;
          if (prNumber) {
            accessories.push({
              tag: {
                value: `PR #${prNumber}${prStatus ? ` ${prStatus}` : ""}`,
                color: prStatus === "MERGED" ? Color.Purple : Color.Blue,
              },
            });
          }

          return (
            <List.Item
              key={session.id}
              title={`${prefix}${session.name}`}
              subtitle={session.path ?? undefined}
              icon={session.bare ? Icon.Circle : Icon.Leaf}
              accessories={accessories}
              actions={
                <SessionActions
                  session={session}
                  repoName={repoName}
                  onDelete={revalidate}
                />
              }
            />
          );
        })
      )}
    </List>
  );
}
