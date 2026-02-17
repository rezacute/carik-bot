#!/bin/bash
cd /home/ubuntu/.openclaw/workspace/carik-bot
source .env
exec ./target/release/carik-bot run
