#!/bin/bash
# Carik Bot Installer for macOS

set -e

echo "🚀 Installing Carik Bot..."

# Check for required tools
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Install from https://rustup.rs/"
    exit 1
fi

# Get the directory where script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$SCRIPT_DIR"

# Create app data directory
mkdir -p ~/.carik-bot
mkdir -p ~/Library/Application\ Support/carik-bot

# Copy config if not exists
if [ ! -f ~/.carik-bot/config.yaml ]; then
    cp "$APP_DIR/config.yaml.example" ~/.carik-bot/config.yaml 2>/dev/null || true
fi

# Build the bot
echo "📦 Building Carik Bot..."
cd "$APP_DIR"
cargo build --release

# Create LaunchAgent (for starting at login)
mkdir -p ~/Library/LaunchAgents
cat > ~/Library/LaunchAgents/com.carik.bot.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.carik.bot</string>
    <key>ProgramArguments</key>
    <array>
        <string>$APP_DIR/target/release/carik-bot</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
EOF

echo "✅ Installation complete!"
echo ""
echo "To run:"
echo "  ./target/release/carik-bot run"
echo ""
echo "To start at login:"
echo "  launchctl load ~/Library/LaunchAgents/com.carik.bot.plist"
