#!/bin/bash

# Required parameters:
# @raycast.schemaVersion 1
# @raycast.title EZ Sessions
# @raycast.mode fullOutput
# @raycast.packageName EZ Workspaces
# @raycast.argument1 { "type": "text", "placeholder": "repo name" }

# Optional parameters:
# @raycast.icon 🌿
# @raycast.description Browse sessions for an ez-workspaces repo

# Documentation:
# @raycast.author ez-workspaces

if ! command -v ez &> /dev/null; then
  echo "Error: 'ez' is not installed or not in PATH"
  exit 1
fi

if ! command -v jq &> /dev/null; then
  echo "Error: 'jq' is not installed (brew install jq)"
  exit 1
fi

repo="$1"
if [ -z "$repo" ]; then
  echo "Error: repo name is required"
  exit 1
fi

json=$(ez session list --json --repo "$repo" 2>&1)
if [ $? -ne 0 ]; then
  echo "Error running 'ez session list --json --repo $repo':"
  echo "$json"
  exit 1
fi

count=$(echo "$json" | jq 'length')
if [ "$count" -eq 0 ]; then
  echo "No sessions for repo '$repo'. Use 'ez new' to create one."
  exit 0
fi

echo "🌿 Sessions for $repo — $count session(s)"
echo "───────────────────────────────────────────"
echo ""

echo "$json" | jq -r '.[] | "  \(.name)\(.bare // false | if . then " [bare]" else "" end)\n    Path: \(.path)\n"'
