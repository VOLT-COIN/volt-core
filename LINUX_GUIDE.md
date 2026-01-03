# 🐧 Linux Server Guide: Volt Node & Mining

This guide explains how to set up a Volt Node and CPU Miner on a fresh Ubuntu server.

## 1. Prepare Server & Install Rust
Run this block to install dependencies and the Rust toolchain.

```bash
# Update and install tools
sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev git curl screen automake libcurl4-openssl-dev libjansson-dev libgmp-dev

# Install Rust (Select '1' if prompted)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## 2. Install & Run Node (Volt Core)
This builds the blockchain node.

```bash
# Clone Project
cd ~
git clone https://github.com/eslamsheref5000/volt-core.git
cd volt-core

# Build (Release Mode)
cargo build --release --bin volt_core

# Run Node in Background Screen
screen -S node

# Make sure you are in the folder
cd ~/volt-core

# COMMAND TO START NODE:
./target/release/volt_core

# (Wait for it to sync, then Press Ctrl+A then D to detach)
```

## 3. Install & Run Miner (CPU Miner)
This builds the miner software for SHA-256d.

```bash
cd ~
git clone https://github.com/tpruvot/cpuminer-multi.git
cd cpuminer-multi
./build.sh

# Run Miner in Background Screen
screen -S miner
# REPLACE 'YOUR_WALLET_ADDRESS' WITH YOUR ACTUAL ADDRESS!
./cpuminer -a sha256d -o stratum+tcp://localhost:3333 -u YOUR_WALLET_ADDRESS -p x
# Press Ctrl+A then D to detach
```

## 4. Useful Commands
- **Check Node:** `screen -r node`
- **Check Miner:** `screen -r miner`
- **Detach Screen:** `Ctrl+A` then `D`
- **Stop Process:** `Ctrl+C` (inside the screen)
