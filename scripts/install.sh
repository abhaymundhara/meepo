#!/bin/bash
set -euo pipefail

PLIST_NAME="com.meepo.meepo"
PLIST_PATH="$HOME/Library/LaunchAgents/$PLIST_NAME.plist"
BINARY_PATH="$HOME/.cargo/bin/meepo"
LOG_DIR="$HOME/.meepo/logs"

echo "Installing Meepo as a macOS launch agent..."

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "Meepo binary not found at $BINARY_PATH"
    echo "Run: cargo install --path crates/meepo-cli"
    exit 1
fi

# Create log directory
mkdir -p "$LOG_DIR"

# Create launchd plist
cat > "$PLIST_PATH" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>$PLIST_NAME</string>
    <key>ProgramArguments</key>
    <array>
        <string>$BINARY_PATH</string>
        <string>start</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$LOG_DIR/meepo.out.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/meepo.err.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin:$HOME/.cargo/bin</string>
    </dict>
</dict>
</plist>
EOF

echo "Created launchd plist at $PLIST_PATH"

# Load the agent
launchctl load "$PLIST_PATH"
echo "Meepo meepo started and will run on login."
echo ""
echo "Commands:"
echo "  launchctl stop $PLIST_NAME     # Stop"
echo "  launchctl start $PLIST_NAME    # Start"
echo "  launchctl unload $PLIST_PATH   # Uninstall"
echo "  tail -f $LOG_DIR/meepo.out.log # View logs"
