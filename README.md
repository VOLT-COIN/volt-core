# ⚡ Volt Core & Wallet

**Volt is a high-performance Layer 1 blockchain built in Rust.** It combines a custom Proof-of-Work (PoW) engine with advanced networking and a modern React credentials-free web wallet.

### 🌐 Network Status: **ONLINE**
- **RPC Endpoint**: `https://voltcore-node.hf.space/api/rpc`
- **Mining Stratum**: `stratum+tcp://82.201.143.174:3333`
- **Block Explorer**: [https://volt-core.vercel.app/](https://volt-core.vercel.app/)
- **Web Wallet**: [https://volt-core.vercel.app/wallet](https://volt-core.vercel.app/wallet)

---

## 🚀 Key Features

### 🛡️ Security & Privacy
*   **Mandatory Wallet Encryption**: All wallets are encrypted by default using **AES-256 GCM** and **PBKDF2**.
*   **MTP-11 Consensus**: Implements **Median Time Past (MTP)** rules to prevent Time Warp attacks.
*   **Active Defense**: Automatic **Peer Banning** system detects and blocks malicious nodes.

### ⚙️ Protocol
*   **Hybrid Consensus**: PoW mining (CPU/Argon2-like) with PoS validation (Staking).
*   **Dynamic Difficulty**: Custom retargeting algorithm adjusts seamlessly.
*   **Native Assets**: Built-in support for Token Issuance, NFTs, and DEX.

### 💼 Volt Wallet (Web & Desktop)
*   **Modern Dashboard**: Responsive React-based UI.
*   **Secure Unlocking**: Requires password authentication for sensitive actions.
*   **Visual Explorer**: Built-in block explorer and transaction history.

## 🗺️ Roadmap

1.  **Smart Contract Support (Wasm/EVM)**: Integration of WebAssembly for DApps.
2.  **Advanced P2P Network (Kademlia DHT)**: Faster peer discovery and lower latency.
3.  **GUI Wallet V2**: Enhanced DEX & Asset Management UI.
4.  **Mobile Wallet**: Lightweight SPV client for iOS/Android.

## 🛠️ Getting Started

### Prerequisites
*   **Rust**: Stable toolchain (`cargo`).
*   **Node.js**: For the Web Dashboard.

### Build & Run
1.  **Clone the Repository**:
    ```bash
    git clone https://github.com/VOLT-COIN/volt-core.git
    cd volt-core
    ```

2.  **Run the Node**:
    ```bash
    cargo run --release --bin volt_core
    ```

3.  **Run the Web Dashboard**:
    ```bash
    cd web_deploy
    npm install
    npm run dev
    ```

## 🤝 Contributing
Contributions are welcome! Please check the `implementation_plan.md` for active tasks.

## 📄 License
MIT License
