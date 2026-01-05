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

# 1.5 Start Public Tunnel (Serveo -> Localhost.run Loop)
echo "Starting Public Tunnel Strategy..."
rm -f /tmp/tunnel.log

(
    # Generates a random ID for the session
    RAND_ID=$((1000 + RANDOM % 9999))
    
    while true; do
        echo "Trying Serveo (Alias: voltpool-$RAND_ID)..."
        # Try Serveo with a specific Alias
        ssh -R "voltpool-$RAND_ID:80:localhost:9861" serveo.net \
            -o ServerAliveInterval=60 \
            -o StrictHostKeyChecking=no \
            -o UserKnownHostsFile=/dev/null \
            -o ExitOnForwardFailure=yes \
            > /tmp/tunnel.log 2>&1
        
        echo "Serveo Failed/Exited. Switching to Localhost.run..."
        # Backup: Localhost.run
        ssh -R 80:localhost:9861 localhost.run \
            -o ServerAliveInterval=60 \
            -o StrictHostKeyChecking=no \
            -o UserKnownHostsFile=/dev/null \
            -o ExitOnForwardFailure=yes \
            >> /tmp/tunnel.log 2>&1
            
        echo "Both Tunnels Failed. Sleeping 10s before retry..."
        sleep 10
    done
) &

# Monitor logs
(
    sleep 5
    echo "--- Public Tunnel Log ---"
    tail -f /tmp/tunnel.log &
) &

# 3. Monitor Loops)
wait $VOLT_PID
EXIT_CODE=$?
echo "❌ Volt Core Crashed with exit code $EXIT_CODE"
echo "Sleeping intentionally to keep logs visible..."
sleep 3600
