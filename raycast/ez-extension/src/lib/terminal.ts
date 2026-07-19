import { runAppleScript } from "@raycast/utils";

function escapeForAppleScript(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

export async function runInTerminal(command: string): Promise<void> {
  const escaped = escapeForAppleScript(command);
  await runAppleScript(`
    tell application "Terminal"
      do script "${escaped}"
      activate
    end tell
  `);
}

export async function openPathInTerminal(path: string): Promise<void> {
  const escaped = escapeForAppleScript(path);
  await runAppleScript(`
    tell application "Terminal"
      do script "cd \\"${escaped}\\""
      activate
    end tell
  `);
}
