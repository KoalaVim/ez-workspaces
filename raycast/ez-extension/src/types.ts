export interface Repo {
  id: string;
  name: string;
  path: string;
  is_git: boolean;
  default_branch: string | null;
  remote_url: string | null;
  labels: string[];
}

export interface Session {
  id: string;
  name: string;
  parent_id: string | null;
  path: string | null;
  bare: boolean;
  labels: string[];
  last_accessed: string | null;
  env: Record<string, string>;
  is_default: boolean;
}

