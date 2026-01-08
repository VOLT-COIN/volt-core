# ⚡ Volt Core & Wallet

Volt is a next-generation Hybrid Proof-of-Work (PoW) / Proof-of-Stake (PoS) cryptocurrency built from scratch in Rust. It prioritizes performance, security, and user experience, featuring a custom blockchain engine, decentralized consensus, and a modern GUI wallet.

## 🚀 Key Features

### 🛡️ Security & Privacy
*   **Mandatory Wallet Encryption**: All wallets are encrypted by default using **AES-256 GCM** and **PBKDF2**. Plaintext keys are never stored on disk.
*   **MTP-11 Consensus**: Implements **Median Time Past (MTP)** rules to prevent Time Warp attacks and stabilize block times against network clock skew.
*   **Active Defense**: Automatic **Peer Banning** system detects and blocks malicious nodes attempting DoS, spam, or invalid block propagation.

### ⚙️ Protocol
*   **Hybrid Consensus**: Combines PoW mining (CPU/Argon2-like) with PoS validation (Staking) for 51% attack resistance.
*   **Dynamic Difficulty**: Custom retargeting algorithm adjusts seamlessly to network hashrate fluctuations.
*   **Deflationary Supply**: Verified capped supply (~105M VLT) with scheduled halvings.
*   **Native Assets**: Built-in support for Token Issuance, NFTs, and a Decentralized Exchange (DEX) at the protocol level.

### 💼 Volt Wallet (GUI)
*   **Modern Dashboard**: Built with `egui` for a responsive, high-performance interface.
*   **Setup Wizard**: User-friendly flow for creating new HD wallets or importing existing seed phrases.
*   **Secure Unlocking**: Requires password authentication for all sensitive actions (Transfer, Stake, etc.).
*   **Advanced Fee Market**: Select from **Eco**, **Standard**, or **Fast** transaction fees.
*   **Visual Explorer**: Built-in block explorer and transaction history.

## 🗺️ Roadmap

The following updates are planned for the next development phase:

1.  **Smart Contract Support (Wasm/EVM)**: 
    *   Integration of WebAssembly or EVM to enable complex Decentralized Applications (DApps).
    *   This is the foundation for building true **Layer 2** scaling solutions.

2.  **Advanced P2P Network (Kademlia DHT)**:
    *   Transitioning from the current flooding mechanism to a Kademlia Distributed Hash Table (DHT).
    *   Ensures faster peer discovery, lower latency, and high resistance to censorship.

3.  **GUI Wallet V2 (DEX & Asset UI)**:
    *   A major interface overhaul to expose the core's native capabilities.
    *   **Features**: Visual Token Manager, NFT Gallery, and a one-click Interface for the built-in Decentralized Exchange.

4.  **Mobile Wallet (SPV)**:
    *   Development of a lightweight mobile client using Simplified Payment Verification (SPV).
    *   Allows secure usage on mobile devices without downloading the full blockchain.

5.  **Explorer V2**:
    *   Enhanced block explorer with Rich Lists, Token Analytics, and Network Health metrics.

## 📦 Components

1.  **`volt_core`**: The backbone. Handles P2P networking, blockchain database (`sled`), consensus logic, and mining.
2.  **`volt_wallet`**: The frontend. A cross-platform desktop application that connects to the Core node.
3.  **`miner`**: Integrated CPU miner optimized for the custom PoW algorithm.

## 🛠️ Getting Started

### Prerequisites
*   **Rust**: Stable toolchain (`cargo`).
*   **GCC**: Required for building cryptographic dependencies (`ring`).

### Build & Run
1.  **Clone the Repository**:
    ```bash
    git clone https://github.com/VOLT-COIN/volt-core.git
    cd volt-core
    ```

2.  **Run the Wallet (includes embedded Node)**:
    ```bash
    cd volt_wallet
    cargo run --release
    ```

### Usage
1.  **First Run**: You will be prompted to **Create** or **Import** a wallet.
    *   **Create**: Generates a new 12-word seed phrase. **SAVE THIS SAFELY!** Set a strong password.
    *   **Import**: Restore from an existing mnemonic.
2.  **Dashboard**: Once unlocked, you can view your balance, history, and network status.
3.  **Mining**: The internal node will automatically sync and can be configured to mine.

## 🤝 Contributing
Contributions are welcome! Please check the `implementation_plan.md` for active tasks.

## 📄 License
MIT License
