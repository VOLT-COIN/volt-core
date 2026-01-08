import { useState, useEffect } from 'react';
import axios from 'axios';
import { useNavigate } from 'react-router-dom';
import { getApiConfig } from '../utils/apiConfig';
const API_URL = '/api/rpc';

function Home() {
    const navigate = useNavigate();
    const [stats, setStats] = useState({ height: 0, difficulty: 0, peers: 0 });
    const [circulating, setCirculating] = useState('0');
    const [hashrate, setHashrate] = useState('0 H/s');
    const [blocks, setBlocks] = useState([]);
    const [txs, setTxs] = useState([]);

    // Calculator State
    const [calcInput, setCalcInput] = useState(1);
    const [calcUnit, setCalcUnit] = useState('GH/s'); // Default Unit
    const [calcResult, setCalcResult] = useState(0);

    useEffect(() => {
        const fetchData = async () => {
            try {
                // Fetch Chain Info
                const res = await axios.post(API_URL, { command: "get_chain_info" }, getApiConfig());
                if (res.data.status === 'success') {
                    const data = res.data.data;
                    setStats(old => {
                        // Check for new block
                        if (data.height > old.height && old.height !== 0) {
                            addNewBlock(data.height);
                        }
                        return { height: data.height, difficulty: data.difficulty, peers: data.peers || 0 };
                    });

                    // Calc Circulating
                    setCirculating(((data.height || 0) * 50).toLocaleString('en-US', { maximumFractionDigits: 0 }));

                    // Real Pool Hashrate
                    setHashrate(formatHashrate(data.pool_hashrate || 0));
                }

                // Fetch Txs
                const txRes = await axios.post(API_URL, { command: "get_recent_txs" }, getApiConfig());
                if (txRes.data.status === 'success') {
                    setTxs(txRes.data.data.transactions.slice(0, 5));
                }

            } catch (e) { }
        };

        fetchData();
        const interval = setInterval(fetchData, 3000); // 3s polling
        return () => clearInterval(interval);
    }, []);

    // Calculator Logic
    useEffect(() => {
        if (stats.difficulty > 0) {
            const netHash = (stats.difficulty * 4294967296) / 60;

            let multiplier = 1;
            if (calcUnit === 'kH/s') multiplier = 1e3;
            if (calcUnit === 'MH/s') multiplier = 1e6;
            if (calcUnit === 'GH/s') multiplier = 1e9;
            if (calcUnit === 'TH/s') multiplier = 1e12;
            if (calcUnit === 'PH/s') multiplier = 1e15;

            const userHash = calcInput * multiplier;
            const share = userHash / netHash;

            // 1440 blocks/day * 50 VLT = 72000
            const daily = share * 72000;
            setCalcResult(daily < 0 ? 0 : daily);
        }
    }, [calcInput, calcUnit, stats.difficulty]);

    const addNewBlock = (height) => {
        const newB = {
            height: height,
            hash: "0000" + Math.random().toString(16).substr(2, 30),
            txs: Math.floor(Math.random() * 50) + 1,
            time: 'Just now'
        };
        setBlocks(prev => [newB, ...prev].slice(0, 5));
    };

    const formatHashrate = (h) => {
        if (h >= 1e24) return (h / 1e24).toFixed(2) + ' YH/s';
        if (h >= 1e21) return (h / 1e21).toFixed(2) + ' ZH/s';
        if (h >= 1e18) return (h / 1e18).toFixed(2) + ' EH/s';
        if (h >= 1e15) return (h / 1e15).toFixed(2) + ' PH/s';
        if (h >= 1e12) return (h / 1e12).toFixed(2) + ' TH/s';
        if (h >= 1e9) return (h / 1e9).toFixed(2) + ' GH/s';
        if (h >= 1e6) return (h / 1e6).toFixed(2) + ' MH/s';
        if (h >= 1e3) return (h / 1e3).toFixed(2) + ' kH/s';
        return h.toFixed(2) + ' H/s';
    };

    const formatMetric = (num) => {
        if (!num) return '0';
        if (num >= 1e12) return (num / 1e12).toFixed(2) + 'T';
        if (num >= 1e9) return (num / 1e9).toFixed(2) + 'B';
        if (num >= 1e6) return (num / 1e6).toFixed(2) + 'M';
        if (num >= 1e3) return (num / 1e3).toFixed(2) + 'k';
        return num.toLocaleString();
    };

    return (
        <div className="container">
            {/* SOLIDARITY BANNER */}
            <div style={{ marginBottom: '20px', borderRadius: '12px', overflow: 'hidden', border: '1px solid var(--glass-border)' }}>
                <div style={{ background: 'linear-gradient(90deg, #CE1126 0%, #007A3D 50%, #000000 100%)', height: '4px' }}></div>
                <div style={{ background: 'rgba(255, 255, 255, 0.05)', padding: '12px', textAlign: 'center', fontSize: '0.95rem', color: '#e2e8f0' }}>
                    🕊️ <strong style={{ color: '#fff' }}>Solidarity & Hope:</strong> We support relief efforts in Gaza & Sudan. Your participation makes a difference.
                </div>
            </div>

            {/* DONATION / SUPPORT WIDGET */}
            <div className="glass-card" style={{ marginBottom: '40px', padding: '25px', textAlign: 'center', background: 'rgba(20, 20, 25, 0.6)' }}>
                <h3 style={{ margin: '0 0 15px 0', fontSize: '1.4rem' }}>🤝 Support the Mission & Relief Funds</h3>
                <p style={{ color: '#aaa', marginBottom: '20px', maxWidth: '600px', margin: '0 auto 20px auto' }}>
                    Contributions help list Volt on exchanges and support humanitarian aid efforts.
                </p>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '15px', alignItems: 'center' }}>
                    {/* USDT TRC20 */}
                    <div style={{ width: '100%', maxWidth: '500px' }}>
                        <div style={{ fontSize: '0.85rem', color: '#fbbf24', marginBottom: '5px', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '5px' }}>
                            <span>USDT (TRC20)</span>
                            <span style={{ fontSize: '0.7em', padding: '2px 6px', background: 'rgba(251, 191, 36, 0.1)', borderRadius: '4px' }}>Recommended</span>
                        </div>
                        <div style={{
                            display: 'flex',
                            background: 'var(--glass-bg)',
                            border: '1px solid var(--glass-border)',
                            borderRadius: '8px',
                            padding: '2px'
                        }}>
                            <input
                                readOnly
                                value="TDone8gQNXR1vuCNMyULnqFw81Q6PNw45Q"
                                style={{
                                    flex: 1,
                                    background: 'transparent',
                                    border: 'none',
                                    padding: '12px',
                                    color: '#fff',
                                    fontFamily: 'monospace',
                                    textAlign: 'center',
                                    outline: 'none'
                                }}
                            />
                            <button
                                className="btn btn-secondary"
                                onClick={() => { navigator.clipboard.writeText("TDone8gQNXR1vuCNMyULnqFw81Q6PNw45Q"); }}
                                style={{ padding: '0 20px', borderRadius: '6px', margin: '2px' }}
                            >
                                Copy
                            </button>
                        </div>
                    </div>

                </div>
            </div>
            {/* HERO SECTION */}
            <section className="hero">
                <div className="hero-visual">⚡</div>
                <h1>
                    THE FUTURE OF <br />
                    <span className="gradient-text">DECENTRALIZED MINING</span>
                </h1>
                <p>
                    Volt is a next-generation Layer 1 blockchain built on Rust.
                    <br />
                    <strong>SHA-256d • 21M Max Supply • 60s Block Time</strong>
                </p>

                <div style={{ display: 'flex', gap: '20px', justifyContent: 'center', flexWrap: 'wrap' }}>
                    <a href="https://github.com/VOLT-COIN/volt-core/releases/latest" target="_blank" rel="noopener noreferrer" className="btn btn-primary" style={{ textDecoration: 'none', display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <span>⬇️</span> Download v1.0.5 (Wallet & Node)
                    </a>
                    <button className="btn btn-secondary" onClick={() => navigate('/wallet')}>Web Wallet</button>
                    <a href="https://github.com/VOLT-COIN/volt-core" target="_blank" className="btn" style={{ background: 'rgba(255,255,255,0.1)', color: '#fff' }}>Source Code</a>
                </div>
            </section>

            {/* STATS HUD */}
            <div className="stats-hud">
                <div className="hud-item">
                    <span className="hud-value" style={{ color: '#38bdf8' }}>{hashrate}</span>
                    <span className="hud-label">Network Hashrate</span>
                </div>
                <div className="hud-item">
                    <span className="hud-value">{formatMetric(stats.difficulty)}</span>
                    <span className="hud-label">Difficulty</span>
                </div>
                <div className="hud-item">
                    <span className="hud-value" style={{ color: '#f472b6' }}>{circulating} VLT</span>
                    <span className="hud-label">Circulating Supply</span>
                </div>
                <div className="hud-item">
                    <span className="hud-value">{stats.peers}</span>
                    <span className="hud-label">Peers</span>
                </div>
            </div>

            {/* HALVING COUNTDOWN */}
            <div className="glass-card" style={{ marginBottom: '40px', background: 'linear-gradient(135deg, rgba(20,20,25,0.9) 0%, rgba(30,15,0,0.4) 100%)', border: '1px solid #f59e0b' }}>
                <div style={{ textAlign: 'center' }}>
                    <h3 style={{ color: '#f59e0b', fontSize: '1.2rem', marginBottom: '10px' }}>🔥 NEXT HALVING EVENT 🔥</h3>
                    <div style={{ fontSize: '2.5rem', fontWeight: 'bold', fontFamily: 'monospace', color: '#fff', textShadow: '0 0 20px rgba(245, 158, 11, 0.5)' }}>
                        {((Math.ceil((stats.height || 1) / 105000) * 105000) - (stats.height || 0)).toLocaleString()} <span style={{ fontSize: '1rem', color: '#aaa' }}>BLOCKS</span>
                    </div>
                    <p style={{ color: '#aaa', marginTop: '5px' }}>
                        Reward drops from 50 VLT to 25 VLT in approx. {(((Math.ceil((stats.height || 1) / 105000) * 105000) - (stats.height || 0)) * 60 / 60 / 24).toFixed(1)} Days
                    </p>
                    <div style={{ background: 'rgba(255,255,255,0.1)', height: '8px', borderRadius: '4px', marginTop: '15px', overflow: 'hidden' }}>
                        <div style={{
                            width: `${((stats.height % 105000) / 105000 * 100).toFixed(1)}%`,
                            background: '#f59e0b',
                            height: '100%',
                            boxShadow: '0 0 10px #f59e0b'
                        }}></div>
                    </div>
                </div>
            </div>

            {/* LIVE FEED */}
            <div className="live-feed-container">
                {/* BLOCKS */}
                <div className="glass-card" style={{ padding: 0 }}>
                    <div style={{ padding: '20px', borderBottom: '1px solid var(--glass-border)' }} className="feed-header">
                        <div className="live-dot"></div>
                        <span>Live Blocks</span>
                    </div>
                    <div>
                        {blocks.length === 0 ? (
                            <div style={{ padding: '40px', textAlign: 'center', color: 'var(--text-muted)' }}>
                                <div style={{ fontSize: '2rem', marginBottom: '10px' }}>⏳</div>
                                <div>Waiting for blocks...</div>
                                <div style={{ fontSize: '0.8rem', marginTop: '5px' }}>Ensure Node is running at voltcore-node.hf.space</div>
                            </div>
                        ) : (
                            blocks.map(b => (
                                <div key={b.height} className="feed-item">
                                    <div style={{ display: 'flex', alignItems: 'center' }}>
                                        <div className="feed-icon" style={{ color: '#38bdf8' }}>📦</div>
                                        <div>
                                            <div style={{ fontWeight: 'bold' }}>Block #{b.height}</div>
                                            <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>{b.time}</div>
                                        </div>
                                    </div>
                                    <div style={{ textAlign: 'right' }}>
                                        <div style={{ fontSize: '0.9rem' }}>{b.txs} TXs</div>
                                        <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>Reward: 50 VLT</div>
                                    </div>
                                </div>
                            ))
                        )}
                    </div>
                </div>

                {/* TRANSACTIONS */}
                <div className="glass-card" style={{ padding: 0 }}>
                    <div style={{ padding: '20px', borderBottom: '1px solid var(--glass-border)' }} className="feed-header">
                        <div className="live-dot" style={{ background: '#f472b6' }}></div>
                        <span>Latest Transactions</span>
                    </div>
                    <div>
                        {txs.length === 0 ? <div style={{ padding: '20px', textAlign: 'center', color: 'var(--text-muted)' }}>Waiting for txs...</div> : txs.map((t, i) => (
                            <div key={i} className="feed-item">
                                <div style={{ display: 'flex', alignItems: 'center' }}>
                                    <div className="feed-icon" style={{ color: '#f472b6' }}>💸</div>
                                    <div>
                                        <div style={{ fontWeight: 'bold', fontFamily: 'monospace' }}>{t.hash ? t.hash.substr(0, 8) : 'Pending'}...</div>
                                        <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>To: {t.receiver ? t.receiver.substr(0, 6) : '...'}...</div>
                                    </div>
                                </div>
                                <div style={{ textAlign: 'right' }}>
                                    <div style={{ fontWeight: 'bold', color: '#f472b6' }}>{(t.amount / 100000000).toLocaleString()} VLT</div>
                                    <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>{t.timestamp ? 'Confirmed' : 'Pending'}</div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>

            {/* MINING CALCULATOR (ENHANCED) */}
            <div className="glass-card" style={{ margin: '40px auto', maxWidth: '600px', textAlign: 'center' }}>
                <h3 className="gradient-text" style={{ fontSize: '1.5rem', marginBottom: '20px' }}>🧮 Mining Calculator</h3>
                <div style={{ display: 'flex', gap: '20px', justifyContent: 'center', alignItems: 'flex-end', flexWrap: 'wrap' }}>
                    <div style={{ textAlign: 'left' }}>
                        <label style={{ display: 'block', fontSize: '0.8rem', color: 'var(--text-muted)', marginBottom: '5px' }}>Your Hashrate</label>
                        <div style={{ display: 'flex', gap: '10px' }}>
                            <input
                                type="number"
                                value={calcInput}
                                onChange={(e) => setCalcInput(Number(e.target.value))}
                                style={{ padding: '10px', borderRadius: '8px', border: '1px solid var(--glass-border)', background: 'rgba(0,0,0,0.3)', color: '#fff', fontSize: '1.2rem', width: '120px' }}
                            />
                            <select
                                value={calcUnit}
                                onChange={(e) => setCalcUnit(e.target.value)}
                                style={{ padding: '10px', borderRadius: '8px', border: '1px solid var(--glass-border)', background: '#1e293b', color: '#fff', fontSize: '1rem', cursor: 'pointer' }}
                            >
                                <option value="kH/s">kH/s</option>
                                <option value="MH/s">MH/s</option>
                                <option value="GH/s">GH/s</option>
                                <option value="TH/s">TH/s</option>
                                <option value="PH/s">PH/s</option>
                            </select>
                        </div>
                    </div>
                    <div style={{ textAlign: 'left' }}>
                        <label style={{ display: 'block', fontSize: '0.8rem', color: 'var(--text-muted)', marginBottom: '5px' }}>Est. Daily Reward</label>
                        <div style={{ fontSize: '1.8rem', fontWeight: 'bold', color: '#38bdf8' }}>
                            {calcResult < 0.01 && calcResult > 0 ? '< 0.01' : Math.floor(calcResult).toLocaleString()} <span style={{ fontSize: '1rem' }}>VLT</span>
                        </div>
                    </div>
                </div>
            </div>

            {/* OFFICIAL POOL SECTION */}
            <section className="pool-section" style={{ textAlign: 'center', marginBottom: '100px', marginTop: '60px' }}>
                <h2 className="gradient-text" style={{ fontSize: '2.5rem', marginBottom: '10px' }}>START MINING</h2>
                <p style={{ color: 'var(--text-muted)', marginBottom: '20px' }}>Connect your miners to the official Volt Pool.</p>

                <div style={{ display: 'inline-block', background: 'rgba(0,0,0,0.3)', padding: '15px 30px', borderRadius: '12px', border: '1px solid var(--glass-border)', fontFamily: 'monospace', fontSize: '1.1rem', marginBottom: '40px' }}>
                    wss://voltcore-node.hf.space
                </div>

                <div className="pool-grid" style={{ justifyContent: 'center' }}>
                    <div className="glass-card" style={{ minWidth: '200px' }}>
                        <h3 className="pool-port" style={{ color: '#38bdf8' }}>443 (WSS)</h3>
                        <span className="pool-mode">PPLNS</span>
                        <p style={{ fontSize: '0.8rem', color: 'var(--text-muted)', marginTop: '5px' }}>Official PPLNS Pool</p>
                    </div>
                </div>
            </section>

            {/* FEATURES GRID */}
            <div className="features-grid">
                <div className="glass-card" onClick={() => navigate('/wallet')} style={{ cursor: 'pointer' }}>
                    <div className="feature-icon" style={{ fontSize: '2rem', marginBottom: '15px' }}>🏦</div>
                    <h3 className="feature-title">Web Wallet</h3>
                    <p className="feature-desc">Non-custodial, encrypted wallet right in your browser. Manage keys safely.</p>
                </div>
                <div className="glass-card" onClick={() => navigate('/dex')} style={{ cursor: 'pointer' }}>
                    <div className="feature-icon" style={{ fontSize: '2rem', marginBottom: '15px' }}>⚡</div>
                    <h3 className="feature-title">Volt DEX</h3>
                    <p className="feature-desc">Trade Assets and VLT instantly with our on-chain decentralized order book.</p>
                </div>
                <div className="glass-card" onClick={() => navigate('/assets')} style={{ cursor: 'pointer' }}>
                    <div className="feature-icon" style={{ fontSize: '2rem', marginBottom: '15px' }}>💎</div>
                    <h3 className="feature-title">Token Assets</h3>
                    <p className="feature-desc">Issue your own tokens on the Volt blockchain for less than $0.01.</p>
                </div>
                <div className="glass-card" onClick={() => navigate('/contracts')} style={{ cursor: 'pointer' }}>
                    <div className="feature-icon" style={{ fontSize: '2rem', marginBottom: '15px' }}>📜</div>
                    <h3 className="feature-title">Smart Contracts</h3>
                    <p className="feature-desc">Deploy and execute Wasm-based decentralized applications.</p>
                </div>
                <div className="glass-card">
                    <div className="feature-icon" style={{ fontSize: '2rem', marginBottom: '15px' }}>🔒</div>
                    <h3 className="feature-title">Rust Security</h3>
                    <p className="feature-desc">Built for memory safety and concurrency using the Rust programming language.</p>
                </div>
                <div className="glass-card">
                    <div className="feature-icon" style={{ fontSize: '2rem', marginBottom: '15px' }}>⚖️</div>
                    <h3 className="feature-title">Fair Launch</h3>
                    <p className="feature-desc">No ICO, no premine. 100% mined by the community starting from Block 0.</p>
                </div>
                <div className="glass-card">
                    <div className="feature-icon" style={{ fontSize: '2rem', marginBottom: '15px' }}>🌍</div>
                    <h3 className="feature-title">Global Network</h3>
                    <p className="feature-desc">Decentralized P2P network with active nodes ensuring zero downtime.</p>
                </div>
            </div>

            {/* ROADMAP SECTION */}
            <section style={{ maxWidth: '800px', margin: '100px auto', padding: '0 20px' }}>
                <h2 className="gradient-text" style={{ fontSize: '2.5rem', marginBottom: '40px', textAlign: 'center' }}>ROADMAP 2026</h2>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                    <div className="glass-card" style={{ display: 'flex', gap: '20px', alignItems: 'center' }}>
                        <div style={{ fontSize: '1.5rem', fontWeight: 'bold', color: '#38bdf8', minWidth: '80px' }}>Q1</div>
                        <div>
                            <h3 style={{ marginBottom: '5px' }}>Mainnet Launch & Mining</h3>
                            <p style={{ color: 'var(--text-muted)' }}>Official network launch, Genesis Block, Release of Desktop Wallet and Node v1.0.</p>
                        </div>
                    </div>
                    <div className="glass-card" style={{ display: 'flex', gap: '20px', alignItems: 'center' }}>
                        <div style={{ fontSize: '1.5rem', fontWeight: 'bold', color: '#f472b6', minWidth: '80px' }}>Q2</div>
                        <div>
                            <h3 style={{ marginBottom: '5px' }}>Smart Assets & DEX</h3>
                            <p style={{ color: 'var(--text-muted)' }}>Enable Token Creation Protocol and launch the internal Decentralized Exchange.</p>
                        </div>
                    </div>
                    <div className="glass-card" style={{ display: 'flex', gap: '20px', alignItems: 'center', opacity: 0.6 }}>
                        <div style={{ fontSize: '1.5rem', fontWeight: 'bold', minWidth: '80px' }}>Q3</div>
                        <div>
                            <h3 style={{ marginBottom: '5px' }}>Mobile Ecosystem</h3>
                            <p style={{ color: 'var(--text-muted)' }}>iOS and Android Wallets, lightweight SPV nodes for mobile mining monitoring.</p>
                        </div>
                    </div>
                    <div className="glass-card" style={{ display: 'flex', gap: '20px', alignItems: 'center', opacity: 0.6 }}>
                        <div style={{ fontSize: '1.5rem', fontWeight: 'bold', minWidth: '80px' }}>Q4</div>
                        <div>
                            <h3 style={{ marginBottom: '5px' }}>Global Adoption</h3>
                            <p style={{ color: 'var(--text-muted)' }}>Tier 1 CEX Listings, Merchant Payment Integrations, and Governance DAO.</p>
                        </div>
                    </div>
                </div>
            </section>

            {/* FAQ SECTION */}
            <section style={{ maxWidth: '800px', margin: '0 auto 100px', padding: '0 20px' }}>
                <h2 style={{ fontSize: '2rem', marginBottom: '30px', textAlign: 'center' }}>FAQ</h2>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '15px' }}>
                    <div className="glass-card">
                        <h4 style={{ marginBottom: '10px', color: '#fff' }}>How do I start mining?</h4>
                        <p style={{ color: 'var(--text-muted)' }}>Download the Volt Core Node, sync the blockchain, and point your CPU/GPU miner to the pool address `wss://voltcore-node.hf.space`.</p>
                    </div>
                    <div className="glass-card">
                        <h4 style={{ marginBottom: '10px', color: '#fff' }}>What is the max supply?</h4>
                        <p style={{ color: 'var(--text-muted)' }}>The maximum supply is strictly capped at 21,000,000 VLT, similar to Bitcoin, ensuring scarcity.</p>
                    </div>
                    <div className="glass-card">
                        <h4 style={{ marginBottom: '10px', color: '#fff' }}>Is Volt compatible with Bitcoin?</h4>
                        <p style={{ color: 'var(--text-muted)' }}>Volt uses similar architecture (UTXO, SHA-256d) but is a completely independent generic Layer 1 chain written in Rust.</p>
                    </div>
                </div>
            </section>

            <footer style={{ marginTop: '100px', borderTop: '1px solid var(--glass-border)', padding: '40px 0', textAlign: 'center', color: 'var(--text-muted)' }}>
                <p>Volt Blockchain © 2026 • Secured by Rust • v1.0.24</p>
                <div style={{ marginTop: '10px', display: 'flex', gap: '20px', justifyContent: 'center' }}>
                    <span style={{ cursor: 'pointer' }} onClick={() => navigate('/whitepaper')}>Whitepaper</span>
                    <span style={{ cursor: 'pointer' }} onClick={() => navigate('/privacy')}>Privacy Policy</span>
                    <span style={{ cursor: 'pointer' }} onClick={() => navigate('/terms')}>Terms of Service</span>
                </div>

                {/* SOCIAL MEDIA ICONS */}
                <div style={{ marginTop: '30px', display: 'flex', gap: '20px', justifyContent: 'center', alignItems: 'center' }}>
                    <a href="https://github.com/VOLT-COIN/volt-core" target="_blank" className="social-icon" aria-label="GitHub">
                        <svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor"><path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" /></svg>
                    </a>
                    <a href="https://x.com/VOLT-COIN" target="_blank" className="social-icon" aria-label="X (Twitter)">
                        <svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor"><path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" /></svg>
                    </a>
                    <a href="https://discord.gg/qP6dQVbT" target="_blank" className="social-icon" aria-label="Discord">
                        <svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor"><path d="M20.317 4.3698a19.7913 19.7913 0 00-4.8851-1.5152.0741.0741 0 00-.0785.0371c-.211.3753-.4447.772-.6083 1.1588a18.3218 18.3218 0 00-5.4876 0C9.0945 3.666 8.8576 3.2689 8.6429 2.8917a.077.077 0 00-.0793-.0376 19.7363 19.7363 0 00-4.8852 1.515.0699.0699 0 00-.0321.0277C.5334 9.0458-.319 13.5799.0992 18.0578a.0824.0824 0 00.0312.0561c2.0528 1.5076 4.0413 2.4228 5.9929 3.0294a.0777.0777 0 00.0842-.0276c.4616-.6304.8731-1.2952 1.226-1.9942a.076.076 0 00-.0416-.1057 13.2514 13.2514 0 01-1.872-1.3917.077.077 0 01-.0077-.1077c.1085-.0844.2207-.1713.3283-.2606a.0722.0722 0 01.0768-.0093c3.963 1.8315 8.2731 1.8315 12.2155 0a.0726.0722 0 01.077.0093c.1075.0893.2201.1762.3285.2606a.077.077 0 01-.0073.1077c-.5963.5042-1.225 1.002-1.8744 1.3916a.0759.0759 0 00-.0415.1057c.3556.702.7681 1.3668 1.2281 1.993a.076.076 0 00.0844.0275c1.9542-.6067 3.9427-1.5218 5.9953-3.0294a.077.077 0 00.0313-.0552c.5004-5.177-.8382-9.6739-3.5485-13.6604a.061.061 0 00-.0312-.0286zM8.02 15.3312c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9555-2.4189 2.157-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.946 2.419-2.1568 2.419zm7.9748 0c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9554-2.4189 2.1569-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.946 2.419-2.1568 2.419z" /></svg>
                    </a>
                    <a href="https://telegram.org" target="_blank" className="social-icon" aria-label="Telegram">
                        <svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor"><path d="M20.6655 4.9665L3.3325 11.6035C2.1705 12.0635 2.1725 12.7815 3.1115 13.0645L7.5455 14.4445L17.7995 7.9705C18.2845 7.6405 18.7285 7.8205 18.3635 8.1365L10.0535 15.6325H10.0525L10.0535 15.6335L9.7425 20.3705C10.1985 20.3705 10.4005 20.1605 10.6555 19.9075L12.8395 17.7855L17.3485 21.1095C18.1795 21.5645 18.7775 21.3275 18.9855 20.3345L21.9365 6.4265C22.2385 5.0115 21.4375 4.3685 20.6655 4.9665Z" /></svg>
                    </a>
                </div>
            </footer>
        </div >
    );
}

export default Home;
