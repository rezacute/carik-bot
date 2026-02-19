# Carik Bot Installer for Windows

Write-Host "🚀 Installing Carik Bot..." -ForegroundColor Green

# Check for Rust
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Rust/Cargo not found. Install from https://rustup.rs/" -ForegroundColor Red
    exit 1
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$AppDir = (Resolve-Path $ScriptDir).Path

# Create app data directory
$AppData = "$env:APPDATA\carik-bot"
New-Item -ItemType Directory -Force -Path $AppData | Out-Null

# Copy config if not exists
if (-not (Test-Path "$AppData\config.yaml")) {
    Copy-Item "$AppDir\config.yaml.example" "$AppData\config.yaml" -ErrorAction SilentlyContinue
}

# Build the bot
Write-Host "📦 Building Carik Bot..." -ForegroundColor Yellow
Set-Location $AppDir
cargo build --release

# Create Start Menu shortcut
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Carik Bot.lnk")
$Shortcut.TargetPath = "$AppDir\target\release\carik-bot.exe"
$Shortcut.Arguments = "run"
$Shortcut.Description = "Carik Bot - AI Telegram Assistant"
$Shortcut.Save()

# Create service (optional)
Write-Host "✅ Installation complete!"
Write-Host ""
Write-Host "To run:" -ForegroundColor Cyan
Write-Host "  .\target\release\carik-bot.exe run"
