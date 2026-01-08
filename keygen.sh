#!/bin/bash
# Generate Mnemonic
echo "Generating Mnemonic..."
curl -s -X POST -d '{"command":"generate_mnemonic"}' http://127.0.0.1:7862 > gen.json
cat gen.json
echo ""

# Extract Mnemonic (Basic grep/sed since jq might not be there)
# JSON: {"data":"word word word ..."}
MNEMONIC=$(cat gen.json | grep -o '"data":"[^"]*' | cut -d'"' -f4)

if [ -z "$MNEMONIC" ]; then
    echo "Failed to generate mnemonic"
    exit 1
fi

echo "Mnemonic: $MNEMONIC"

# Import to retrieve address
echo "Importing to get address..."
# We need to construct JSON with the variable.
# Bash quoting is safe here.
curl -s -X POST -d "{\"command\":\"import_mnemonic\", \"mnemonic\":\"$MNEMONIC\"}" http://127.0.0.1:7862 > import.json
cat import.json
echo ""

# Get Address
echo "Getting Address..."
curl -s -X POST -d '{"command":"get_address"}' http://127.0.0.1:7862 > addr.json
cat addr.json
echo ""
ADDRESS=$(cat addr.json | grep -o '"data":"[^"]*' | cut -d'"' -f4)

echo "MINER_ADDRESS=$ADDRESS"

# Save to persistent file
echo "$MNEMONIC" > /home/eslam/volt-core/miner_mnemonic.txt
echo "$ADDRESS" > /home/eslam/volt-core/miner_address.txt

# Update Service with new address
# We use sed to replace the ExecStart line or append the arg if missing, or replace existing one.
# Existing: ExecStart=... --mine --miner_address auto
# New: ExecStart=... --mine --miner_address $ADDRESS

SERVICE_FILE="/etc/systemd/system/volt.service"
# We need sudo for this.
echo "Updating Service to mine to $ADDRESS..."
# Use a temporary file for sed to avoid permission issues with direct edit if not running as root info
# We will just print the command for the user to run or echo it to a file and sudo mv.

# Generate update script
cat <<EOF > update_service.sh
sed -i 's/--miner_address auto/--miner_address $ADDRESS/' $SERVICE_FILE
systemctl daemon-reload
systemctl restart volt
EOF

echo "Ready to update service. Run: sudo bash update_service.sh"
