import { Color, Icon, List } from "@raycast/api";
import { useCachedPromise } from "@raycast/utils";
import { listRepos, listSessions } from "./lib/ez";
import { formatRelativeTime } from "./lib/session-tree";
import { Repo, Session } from "./types";
import SessionActions from "./components/SessionActions";

interface RepoSessions {
  repo: Repo;
  sessions: Session[];
}

async function fetchAllSessions(): Promise<RepoSessions[]> {
  const repos = await listRepos();
  const result: RepoSessions[] = [];
  for (const repo of repos) {
    try {
      const sessions = await listSessions(repo.name);
      if (sessions.length > 0) {
        result.push({ repo, sessions });
      }
    } catch {
      continue;
    }
  }
  return result;
}

export default function SearchSessions() {
  const {
    data: allRepoSessions,
    isLoading,
    revalidate,
  } = useCachedPromise(fetchAllSessions);

  return (
    <List
      isLoading={isLoading}
      searchBarPlaceholder="Search all sessions..."
    >
      {allRepoSessions?.map(({ repo, sessions }) => (
        <List.Section key={repo.id} title={repo.name} subtitle={repo.path}>
          {sessions.map((session) => {
            const accessories: List.Item.Accessory[] = [];

            if (session.last_accessed) {
              accessories.push({
                text: formatRelativeTime(session.last_accessed),
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
                key={`${repo.id}-${session.id}`}
                title={session.name}
                subtitle={session.path ?? undefined}
                icon={session.bare ? Icon.Circle : Icon.Leaf}
                accessories={accessories}
                actions={
                  <SessionActions
                    session={session}
                    repoName={repo.name}
                    onDelete={revalidate}
                  />
                }
              />
            );
          })}
        </List.Section>
      ))}
    </List>
  );
}
