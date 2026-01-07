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

### 💼 Volt Wallet (GUI)
*   **Modern Dashboard**: Built with `egui` for a responsive, high-performance interface.
*   **Setup Wizard**: User-friendly flow for creating new HD wallets or importing existing seed phrases.
*   **Secure Unlocking**: requires password authentication for all sensitive actions (Transfer, Stake, etc.).
*   **Advanced Fee Market**: Select from **Eco**, **Standard**, or **Fast** transaction fees.
*   **Visual Explorer**: Built-in block explorer and transaction history.

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
    git clone https://github.com/eslamsheref5000/volt-core.git
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
