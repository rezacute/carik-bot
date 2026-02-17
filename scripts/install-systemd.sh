#!/bin/bash
# carik-bot systemd installation script

set -e

# Configuration
SERVICE_NAME="carik-bot"
USER="ubuntu"
WORKDIR="/home/ubuntu/.openclaw/workspace/carik-bot"
BINARY_PATH="${WORKDIR}/target/release/carik-bot"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
ENV_FILE="${WORKDIR}/.env"

echo "🚀 Installing ${SERVICE_NAME} to systemd..."

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    echo "❌ This script must be run as root (use sudo)"
    exit 1
fi

# Create .env file if it doesn't exist
if [[ ! -f "${ENV_FILE}" ]]; then
    echo "📝 Creating ${ENV_FILE}..."
    cat > "${ENV_FILE}" << 'EOF'
# carik-bot environment variables
# BOT_TOKEN=your_telegram_bot_token_here
# GROQ_API_KEY=your_groq_api_key_here
EOF
    echo "⚠️  Edit ${ENV_FILE} and add your BOT_TOKEN before starting!"
else
    echo "✅ .env file already exists"
fi

# Build if binary doesn't exist
if [[ ! -f "${BINARY_PATH}" ]]; then
    echo "🔨 Building ${SERVICE_NAME}..."
    cd "${WORKDIR}"
    if command -v cargo &> /dev/null; then
        cargo build --release
    else
        echo "❌ cargo not found. Please build manually with: cargo build --release"
        exit 1
    fi
else
    echo "✅ Binary already exists at ${BINARY_PATH}"
fi

# Create systemd service file
echo "📄 Creating systemd service file..."
cat > "${SERVICE_FILE}" << EOF
[Unit]
Description=Carik Bot - AI Telegram Assistant
After=network.target

[Service]
Type=simple
User=${USER}
WorkingDirectory=${WORKDIR}
ExecStart=${BINARY_PATH} run
Restart=always
RestartSec=10
EnvironmentFile=${ENV_FILE}

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=${WORKDIR}/logs

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=${SERVICE_NAME}

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd and enable service
echo "🔄 Reloading systemd daemon..."
systemctl daemon-reload

echo "✅ Enabling ${SERVICE_NAME} on boot..."
systemctl enable "${SERVICE_NAME}"

echo "▶️  Starting ${SERVICE_NAME}..."
systemctl start "${SERVICE_NAME}"

echo ""
echo "✅ Installation complete!"
echo ""
echo "Useful commands:"
echo "  systemctl status ${SERVICE_NAME}   # Check status"
echo "  journalctl -u ${SERVICE_NAME} -f   # View logs"
echo "  systemctl restart ${SERVICE_NAME}   # Restart"
echo "  systemctl stop ${SERVICE_NAME}      # Stop"
