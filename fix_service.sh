#!/bin/bash
ADDRESS="0332bf350542c7f5fca0ae0ad97e379e4ea03ce80598e29ba8e14cc666ffca40ac"
echo "Updating Service to mine to $ADDRESS..."
sed -i "s/--miner_address auto/--miner_address $ADDRESS/" /etc/systemd/system/volt.service
systemctl daemon-reload
systemctl restart volt
echo "Service Updated and Restarted."
