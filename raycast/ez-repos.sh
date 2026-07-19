#!/bin/bash

# Required parameters:
# @raycast.schemaVersion 1
# @raycast.title EZ Repos
# @raycast.mode fullOutput
# @raycast.packageName EZ Workspaces

# Optional parameters:
# @raycast.icon 📂
# @raycast.description Browse registered ez-workspaces repos

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

json=$(ez repo list --json 2>&1)
if [ $? -ne 0 ]; then
  echo "Error running 'ez repo list --json':"
  echo "$json"
  exit 1
fi

count=$(echo "$json" | jq 'length')
if [ "$count" -eq 0 ]; then
  echo "No repos registered. Use 'ez repo add' or 'ez repo clone' to add repos."
  exit 0
fi

echo "📂 EZ Workspaces — $count repo(s)"
echo "─────────────────────────────────"
echo ""

echo "$json" | jq -r '.[] | "  \(.name)\n    Path: \(.path)\n"'
