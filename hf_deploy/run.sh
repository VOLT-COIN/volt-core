#!/bin/bash

echo "🚀 Starting Volt Cloud Node on Hugging Face..."
echo "=============================================="

# 1. Start Playit.gg Tunnel in Background
echo "🔵 Launching Playit Tunnel..."
playit &
PLAYIT_PID=$!

echo "⏳ Waiting for Playit to initialize..."
sleep 5

echo ""
echo "⚠️  ACTION REQUIRED: CLAIM YOUR TUNNEL!"
echo "⚠️  Look at the logs below. You will see a link like: https://playit.gg/claim/..."
echo "⚠️  Click it to map your random IP (e.g. lovely-cat.playit.gg) to this Node."
echo ""

# 2. Start Dummy Web Server for HF Health Check (Port 7860)
echo "🟢 Starting Dummy Web Server on 7860..."
mkdir -p public
echo "<h1>Volt Node is RUNNING ⚡</h1><p>Check Logs for Playit Claim URL.</p>" > public/index.html
python3 -m http.server 7860 --directory public &

# 2. Start Volt Node
echo "⚡ Starting Volt Blockchain Node (Port 6000)..."

# Pipe empty input that hangs to prevent infinite loop on EOF
# And point to the new binary location (/app/volt_core)
tail -f /dev/null | ./volt_core
