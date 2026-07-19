import { execSync } from "child_process";
import { Repo, Session } from "../types";

function getEzPath(): string {
  try {
    return execSync("which ez", { encoding: "utf-8" }).trim();
  } catch {
    const common = [
      "/usr/local/bin/ez",
      "/opt/homebrew/bin/ez",
      `${process.env.HOME}/.cargo/bin/ez`,
    ];
    for (const p of common) {
      try {
        execSync(`test -x "${p}"`);
        return p;
      } catch {
        continue;
      }
    }
    throw new Error(
      "ez binary not found. Install ez-workspaces and ensure it's in your PATH.",
    );
  }
}

function runEz(args: string[]): string {
  const ez = getEzPath();
  const cmd = `"${ez}" ${args.map((a) => `"${a}"`).join(" ")}`;
  return execSync(cmd, { encoding: "utf-8", timeout: 10000 });
}

export async function listRepos(): Promise<Repo[]> {
  const output = runEz(["repo", "list", "--json"]);
  return JSON.parse(output) as Repo[];
}

export async function listSessions(repoName: string): Promise<Session[]> {
  const output = runEz(["session", "list", "--json", "--repo", repoName]);
  return JSON.parse(output) as Session[];
}

export function deleteSession(repoName: string, sessionName: string): void {
  runEz(["session", "delete", sessionName, "--repo", repoName, "--force"]);
}

export function removeRepo(repoName: string): void {
  runEz(["repo", "remove", repoName]);
}
