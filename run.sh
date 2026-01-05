#!/bin/bash
set -x # Print every command

# Global Trap: If any command fails, keep container alive to show logs
trap 'echo "❌ SCRIPT CRASHED ON LINE $LINENO"; sleep 3600' ERR

# Create temp directories manually to prevent Permission Denied
echo "Creating Nginx Temp Paths..."
mkdir -p /tmp/client_body /tmp/proxy_temp /tmp/fastcgi_temp /tmp/uwsgi_temp /tmp/scgi_temp
touch /tmp/nginx_error.log

# 1. Start Nginx
echo "Starting Nginx Proxy..."
# Use strict configuration found in file
nginx -c /home/appuser/app/nginx.conf
if [ $? -ne 0 ]; then
    echo "❌ Nginx Failed to Start! Checking Config..."
    nginx -c /home/appuser/app/nginx.conf -t
    exit 1
fi

# 2. Start Volt Core FIRST (Priority)
# Args: [API Port] [P2P Port] [Stratum Port] [Websocket Port]
echo "Starting Volt Core..."
/home/appuser/app/volt_core 7861 7862 9861 7863 &
VOLT_PID=$!
echo "Volt Core started with PID $VOLT_PID"

# 1.5 Start Public Tunnel (Serveo SSH)
# No installation required using standard SSH client
echo "Starting Serveo Tunnel..."
# Auto-restart loop for the tunnel
while true; do
  ssh -R 0:localhost:9861 serveo.net -o ServerAliveInterval=60 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null 2>&1 | grep "Forwarding TCP" &
  TUNNEL_PID=$!
  sleep 10
  if ps -p $TUNNEL_PID > /dev/null; then
     echo "✅ Serveo Tunnel Active."
     break
  else
     echo "⚠️ Serveo disconnected, retrying..."
  fi
done &

# 3. Monitor Loops)
wait $VOLT_PID
EXIT_CODE=$?
echo "❌ Volt Core Crashed with exit code $EXIT_CODE"
echo "Sleeping intentionally to keep logs visible..."
sleep 3600
