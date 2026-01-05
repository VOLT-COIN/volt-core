#!/bin/bash
set -x # Print every command

# Global Trap: If any command fails, keep container alive to show logs
trap 'echo "❌ SCRIPT CRASHED ON LINE $LINENO"; sleep 3600' ERR

# 1. Start Nginx (Reverse Proxy)
# Create temp directories manually to prevent Permission Denied
echo "Creating Nginx Temp Paths..."
mkdir -p /tmp/client_body /tmp/proxy_temp /tmp/fastcgi_temp /tmp/uwsgi_temp /tmp/scgi_temp
touch /tmp/nginx_error.log

echo "Starting Nginx Proxy..."
nginx -c /home/appuser/app/nginx.conf -t
if [ $? -eq 0 ]; then
    nginx -c /home/appuser/app/nginx.conf -g "error_log /tmp/nginx_error.log; pid /tmp/nginx.pid;" &
else
    echo "❌ Nginx Config Invalid! Listing content:"
    cat /home/appuser/app/nginx.conf
fi

# 1.5 Start Public Tunnel
# Strategy: Playit (Permanent) > Pinggy (Temporary Backup)

# Playit Removed per user request
echo "Playit Support Disabled."

# 2. Start Volt Core
# Start on 7861 to avoid conflict with Nginx (on 7860)
# P2P will be at 7862, Stratum at 9861
# 2. Start Volt Core
# Start on 7861 to avoid conflict with Nginx (on 7860)
# P2P will be at 7862, Stratum at 9861
echo "Starting Volt Core..."
# Run in background so we can monitor it
/home/appuser/app/volt_core 7861 &
VOLT_PID=$!

# 3. Monitor Loop (Prevents container exit if Volt crashes)
wait $VOLT_PID
EXIT_CODE=$?
echo "❌ Volt Core Crashed with exit code $EXIT_CODE"
echo "Sleeping intentionally to keep logs visible..."
sleep 3600
