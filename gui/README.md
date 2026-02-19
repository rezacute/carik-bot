# Carik Bot Desktop GUI

A minimal desktop application for Carik Bot.

## Features

- System tray icon
- Quick controls (Start/Stop/Status)
- View logs
- Open configuration

## Build

```bash
# Install Tauri CLI
cargo install tauri-cli

# Build GUI
cd gui
npm install
npm run tauri build
```

## Tech Stack

- **Tauri 2.x** - Lightweight desktop framework
- **React** - Frontend UI
- **Rust** - Backend

## Quick Start

1. Build the bot: `cargo build --release`
2. Build the GUI: `cd gui && npm run tauri build`
3. Run: `./gui/target/release/carik-gui`
