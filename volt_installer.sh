#!/bin/bash
set -e

echo "🚀 Volt Miner Installer for Ubuntu"
echo "=================================="

# 1. Install Dependencies
echo "📦 Installing system dependencies..."
sudo apt update -qq
sudo apt install -y build-essential pkg-config libssl-dev git curl screen

# 2. Install Rust (Non-interactive)
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "✅ Rust already installed."
fi

# 3. Clone & Build
echo "⬇️  Fetching source code..."
cd /tmp
rm -rf volt-core
git clone https://github.com/eslamsheref5000/volt-core.git
cd volt-core

echo "🏗️  Building Miner (Release Mode)..."
# Force update to ensure latest version
git pull origin main
cargo build --release --bin volt_core

# 4. Install Binary
echo "💿 Installing binary to $HOME/volt_miner..."
mkdir -p $HOME/volt_miner
cp target/release/volt_core $HOME/volt_miner/volt_core
chmod +x $HOME/volt_miner/volt_core

# 5. Cleanup
echo "🧹 Cleaning up source files..."
cd /tmp
rm -rf volt-core

echo "=================================="
echo "✅ Installation Complete!"
echo ""
echo "To create a wallet:"
echo "  cd ~/volt_miner && ./volt_core"
echo ""
echo "To start mining:"
echo "  screen -S miner ~/volt_miner/volt_core --mine --address YOUR_ADDRESS"
echo ""
