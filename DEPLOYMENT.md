# carik-bot Deployment Notes

## systemd Service Deployment

### Known Issues

**Issue: Exit Code 203 with ProtectHome=true**

When deploying carik-bot as a systemd service, using `ProtectHome=true` causes the bot to fail with exit code 203 (EXEC error).

**Root Cause:** The bot runs from `/home/ubuntu/.openclaw/workspace/carik-bot` which is under `/home`. The `ProtectHome=true` directive blocks read/write access to `/home/` entirely, preventing the bot from accessing:
- Working directory: `/home/ubuntu/.openclaw/workspace/carik-bot`
- Config file: `/home/ubuntu/.openclaw/workspace/carik-bot/.env`
- Binary: `/home/ubuntu/.openclaw/workspace/carik-bot/target/release/carik-bot`

**Solution:** Use `ProtectSystem=strict` instead of `ProtectHome=true`. This protects system directories (`/usr`, `/etc`, `/boot`, etc.) while allowing access to `/home`.

### Working Service File

```ini
[Unit]
Description=Carik Bot - AI Telegram Assistant
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/.openclaw/workspace/carik-bot
ExecStart=/home/ubuntu/.openclaw/workspace/carik-bot/target/release/carik-bot run
Restart=always
RestartSec=10
EnvironmentFile=/home/ubuntu/.openclaw/workspace/carik-bot/.env

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/home/ubuntu/.openclaw/workspace/carik-bot/logs

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=carik-bot

[Install]
WantedBy=multi-user.target
```

### Environment Variables

The `.env` file must contain:
- `BOT_TOKEN` - Telegram bot token
- `GROQ_API_KEY` - Groq API key for AI responses

```bash
BOT_TOKEN=your_telegram_token_here
GROQ_API_KEY=your_groq_api_key_here
```

### Deployment Commands

```bash
# Install service
sudo cp carik-bot.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable carik-bot
sudo systemctl start carik-bot

# Check status
systemctl status carik-bot

# View logs
journalctl -u carik-bot -f

# Restart
sudo systemctl restart carik-bot
```

### Troubleshooting

**Exit Code 203 (EXEC error):**
- Check if binary exists and is executable
- Verify `ProtectHome=true` is not used
- Check environment file path
- Run manually: `cd /home/ubuntu/.openclaw/workspace/carik-bot && source .env && ./target/release/carik-bot run`

**GROQ_API_KEY warnings:**
- Bot will work in "echo mode" without AI responses
- Add valid API key to `.env`

### Security Notes

- `NoNewPrivileges=true` prevents the service from gaining additional privileges
- `ProtectSystem=strict` mounts `/usr`, `/boot`, `/etc` as read-only
- `ReadWritePaths` explicitly allows write access to logs directory
- Running as non-root `ubuntu` user limits blast radius

## Docker for Kiro

Kiro CLI runs in a Docker container (`kiro-persistent`) for isolation:

```bash
# Start container
docker run -d --name kiro-persistent \
  -v /home/ubuntu/.kiro:/root/.kiro \
  -v /home/ubuntu/.local/share/kiro-cli:/root/.local/share/kiro-cli \
  -v /home/ubuntu/.carik-bot:/workspace \
  -v /home/ubuntu/.local/bin/kiro-cli:/usr/local/bin/kiro-cli \
  -v /home/ubuntu/.aws:/root/.aws \
  -e PATH="/root/.local/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
  -e AWS_ACCESS_KEY_ID=... \
  -e AWS_SECRET_ACCESS_KEY=... \
  -e AWS_REGION=ap-southeast-1 \
  --workdir /workspace \
  ubuntu:latest sleep infinity

# Check container
docker ps | grep kiro-persistent

# View logs
docker logs kiro-persistent

# Restart container
docker restart kiro-persistent
```
