#!/bin/bash
set -euo pipefail

PLIST_NAME="com.meepo.meepo"
PLIST_PATH="$HOME/Library/LaunchAgents/$PLIST_NAME.plist"

echo "Uninstalling Meepo launch agent..."

if [ -f "$PLIST_PATH" ]; then
    launchctl unload "$PLIST_PATH" 2>/dev/null || true
    rm "$PLIST_PATH"
    echo "Removed $PLIST_PATH"
else
    echo "No launchd plist found at $PLIST_PATH"
fi

echo "Meepo meepo uninstalled."
echo "Config and data remain at ~/.meepo/ — delete manually if desired."
