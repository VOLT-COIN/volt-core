import { useNavigate } from 'react-router-dom';

function Whitepaper() {
    return (
        <div className="container" style={{ paddingTop: '100px', color: '#ccc' }}>
            <h1 className="gradient-text">Volt Whitepaper</h1>
            <p><strong>Version 1.0</strong> | January 2026</p>
            <hr style={{ borderColor: '#333', margin: '20px 0' }} />

            <h2>1. Abstract</h2>
            <p>
                Volt (VLT) is a peer-to-peer cryptocurrency built from scratch using the Rust programming language.
                It aims to provide a secure, decentralized, and efficient medium of exchange without the legacy overhead of older blockchains.
                By leveraging Rust's memory safety and performance, Volt offers a modern alternative for miners and users alike.
            </p>

            <h2>2. Introduction</h2>
            <p>
                The cryptocurrency landscape is dominated by forks of Bitcoin and Ethereum. While robust, these codebases carry over a decade of technical debt.
                Volt was conceived as a clean-slate implementation, optimizing for the modern multi-core era while retaining the proven "Nakamoto Consensus" model.
            </p>

            <h2>3. Technical Architecture</h2>
            <h3>3.1 Core Language</h3>
            <p>
                Volt is written entirely in **Rust**, ensuring memory safety without garbage collection.
                This results in a lighter daemon footprint and higher resistance to buffer overflow attacks compared to C++ implementations.
            </p>
            <h3>3.2 Smart Contracts Integration</h3>
            <p>
                Volt features a built-in <strong>WebAssembly (Wasm)</strong> virtual machine.
                This engine allows developers to write smart contracts in any language that compiles to Wasm (e.g., Rust, AssemblyScript)
                and deploy them directly to the blockchain. The execution environment is sandboxed and metered using a "Gas" system to prevent resource abuse.
            </p>
            <h3>3.3 Consensus Mechanism</h3>
            <p>
                Volt utilizes **Proof-of-Work (PoW)** with the **SHA-256d** algorithm (Double SHA-256).
                This ensures compatibility with existing mining hardware (ASICs) and software infrastructures, promoting immediate network security.
            </p>

            <h2>4. Economic Model</h2>
            <ul>
                <li><strong>Ticker:</strong> VLT</li>
                <li><strong>Max Supply:</strong> 21,000,000 VLT</li>
                <li><strong>Block Time:</strong> 60 Seconds</li>
                <li><strong>Block Reward:</strong> 50 VLT</li>
                <li><strong>Halving Interval:</strong> 105,000 Blocks (Approx. 2 Years)</li>
            </ul>
            <p>
                The emission curve mimics Bitcoin's disinflationary model, ensuring VLT becomes scarcer over time.
            </p>

            <h2>5. Network Security</h2>
            <p>
                The difficulty adjustment algorithm retargets every 10 blocks to maintain a stable 60-second block generation time.
                Transactions are secured using Secp256k1 elliptic curve cryptography.
            </p>

            <h2>6. Conclusion</h2>
            <p>
                Volt represents a return to fundamentals—simple, secure, and decentralized money—built with the tools of the future.
            </p>

            <div style={{ marginTop: '50px' }}>
                <a href="/" className="btn btn-secondary">← Back to Home</a>
            </div>
        </div>
    );
}

export default Whitepaper;
