import path from "path";
import { Repo } from "../types";

export type ViewMode = "repo" | "owner" | "label" | "workspace";

export interface RepoGroup {
  title: string;
  repos: Repo[];
}

export function groupRepos(repos: Repo[], mode: ViewMode): RepoGroup[] {
  switch (mode) {
    case "repo":
      return [{ title: "", repos }];

    case "owner":
      return groupBy(repos, (r) => path.basename(path.dirname(r.path)));

    case "label": {
      const labeled = new Map<string, Repo[]>();
      const unlabeled: Repo[] = [];
      for (const repo of repos) {
        if (repo.labels.length === 0) {
          unlabeled.push(repo);
        } else {
          for (const label of repo.labels) {
            const list = labeled.get(label) ?? [];
            list.push(repo);
            labeled.set(label, list);
          }
        }
      }
      const groups: RepoGroup[] = [];
      for (const [label, list] of labeled.entries()) {
        groups.push({ title: label, repos: list });
      }
      groups.sort((a, b) => a.title.localeCompare(b.title));
      if (unlabeled.length > 0) {
        groups.push({ title: "Unlabeled", repos: unlabeled });
      }
      return groups;
    }

    case "workspace":
      return groupBy(repos, (r) => {
        const parts = r.path.split("/");
        const wsIdx = parts.indexOf("workspace");
        if (wsIdx >= 0 && wsIdx + 1 < parts.length) {
          return parts.slice(0, wsIdx + 2).join("/");
        }
        return path.dirname(r.path);
      });
  }
}

function groupBy(repos: Repo[], keyFn: (r: Repo) => string): RepoGroup[] {
  const map = new Map<string, Repo[]>();
  for (const repo of repos) {
    const key = keyFn(repo);
    const list = map.get(key) ?? [];
    list.push(repo);
    map.set(key, list);
  }
  const groups: RepoGroup[] = [];
  for (const [title, list] of map.entries()) {
    groups.push({ title, repos: list });
  }
  return groups.sort((a, b) => a.title.localeCompare(b.title));
}
