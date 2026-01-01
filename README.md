# Volt Blockchain ⚡

**Volt** is a high-performance Layer 1 Proof-of-Work blockchain built entirely in Rust. It is designed for speed, security, and decentralized mining.

## 🚀 Features

- **Built in Rust**: Safe, concurrent, and blazing fast.
- **Proof-of-Work (PoW)**: Uses valid SHA-256 for consensus.
- **Asset Issuance**: Native support for creating custom tokens on-chain.
- **Decentralized**: No pre-mine, fair launch, and community-driven.
- **Cross-Platform**: Runs on Windows, Linux, and macOS.

## 🛠️ Build from Source

### Prerequisites
You need to have **Rust** installed.
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Building
Clone the repository and build the release binary:

```bash
git clone https://github.com/eslamsheref5000/volt-core.git
cd volt_core
cargo build --release
```

The binary will be located at `target/release/volt_core`.

## 🏃 Running a Node

To start a full node and sync with the network:

```bash
./target/release/volt_core
```

### Configuration
On first run, a `config.toml` and `wallet.key` will be generated.
- **Port**: 6000 (P2P), 6001 (RPC)
- **Mining**: Enabled by default (set `mining = false` in config to disable)

## 🌐 Ecosystem
- **Explorer**: [https://volt-core.vercel.app](https://volt-core.vercel.app)
- **GitHub**: [https://github.com/eslamsheref5000/volt-core](https://github.com/eslamsheref5000/volt-core)

## 🤝 Contributing
Pull requests are welcome! For major changes, please open an issue first to discuss what you would like to change.

## 📄 License
This project is licensed under the MIT License.
