import { Session } from "../types";

export interface FlatTreeItem {
  session: Session;
  prefix: string;
  depth: number;
}

export function buildTree(sessions: Session[]): FlatTreeItem[] {
  const byId = new Map(sessions.map((s) => [s.id, s]));
  const childrenOf = new Map<string | null, Session[]>();

  for (const s of sessions) {
    const parentKey = s.parent_id;
    const list = childrenOf.get(parentKey) ?? [];
    list.push(s);
    childrenOf.set(parentKey, list);
  }

  const result: FlatTreeItem[] = [];

  function walk(parentId: string | null, depth: number, linePrefix: string) {
    const children = childrenOf.get(parentId) ?? [];
    children.forEach((child, i) => {
      const isLast = i === children.length - 1;
      const connector = depth === 0 ? "" : isLast ? "└── " : "├── ";
      const prefix = linePrefix + connector;

      result.push({ session: child, prefix, depth });

      const nextPrefix =
        depth === 0 ? "" : linePrefix + (isLast ? "    " : "│   ");
      walk(child.id, depth + 1, nextPrefix);
    });
  }

  walk(null, 0, "");
  return result;
}

export function formatRelativeTime(isoDate: string | null): string {
  if (!isoDate) return "";
  const then = new Date(isoDate).getTime();
  const now = Date.now();
  const diffMs = now - then;
  if (diffMs < 0) return "";

  const seconds = Math.floor(diffMs / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}
