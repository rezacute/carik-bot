#!/bin/bash
# Carik Bot Installer for Linux

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
mkdir -p ~/.config/carik-bot

# Copy config if not exists
if [ ! -f ~/.carik-bot/config.yaml ]; then
    cp "$APP_DIR/config.yaml.example" ~/.carik-bot/config.yaml 2>/dev/null || true
fi

# Build the bot
echo "📦 Building Carik Bot..."
cd "$APP_DIR"
cargo build --release

# Create systemd service (if systemd)
if command -v systemctl &> /dev/null; then
    echo "📋 Installing systemd service..."
    sudo cp carik-bot.service /etc/systemd/system/
    sudo systemctl daemon-reload
    echo "✅ Run 'sudo systemctl enable carik-bot' to start on boot"
fi

# Create launcher
echo "� Creating launcher..."
sudo cp carik-bot.desktop /usr/share/applications/ 2>/dev/null || true
sudo cp carik-bot.desktop ~/.local/share/applications/ 2>/dev/null || true

echo "✅ Installation complete!"
echo ""
echo "To run:"
echo "  ./target/release/carik-bot run"
echo ""
echo "Or as service:"
echo "  sudo systemctl start carik-bot"
