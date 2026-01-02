# Volt Blockchain (VLT)

Volt is a next-generation, Rust-based Layer 1 blockchain designed for decentralized GPU/CPU mining. It features a unique SHA-256d implementation optimized for high throughput and security.

## 🚀 Features

*   **Algorithm:** SHA-256d (Proof of Work)
*   **Max Supply:** 21,000,000 VLT
*   **Block Time:** 60 Seconds
*   **Language:** Rust (Performance & Safety)
*   **Network:** Peer-to-Peer (P2P)

## 📦 Components

This repository contains the full Volt ecosystem:
1.  **`volt_core`**: The main blockchain node and miner.
2.  **`volt_wallet`**: The graphical wallet for Windows/Linux.
3.  **`web_deploy`**: The React-based block explorer and website.

## 📥 Download

Latest Release: [v1.0.6](https://github.com/eslamsheref5000/volt-core/releases/tag/v1.0.6)


## 🛠️ Build from Source

### Prerequisites
*   Rust (latest stable)
*   GCC Toolchain (for Windows resource links)

```bash
# Clone the repository
git clone https://github.com/eslamsheref5000/volt-core.git
cd volt-core

# Build Node
cd src
cargo build --release

# Build Wallet
cd ../volt_wallet
cargo build --release
```

## ⛏️ Mining

Connect to the official **PPLNS** pool:
`stratum+tcp://volt-core.zapto.org:3333`

**Network Status:** [https://volt-core.vercel.app/status](https://volt-core.vercel.app/status)


## 📄 License

MIT License
