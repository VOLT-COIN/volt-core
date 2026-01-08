import { useState, useEffect } from 'react';
import axios from 'axios';
import { useNavigate } from 'react-router-dom';
// import { generateColor } from '../utils/colors'; // Removed non-existent import
import { keysFromMnemonic, signTransaction } from '../utils/wallet';

const API_URL = '/api/rpc';

// Inline helper if not imported
const getColor = (str) => {
    const colors = ['#ef4444', '#f97316', '#f59e0b', '#84cc16', '#10b981', '#06b6d4', '#3b82f6', '#6366f1', '#a855f7', '#ec4899'];
    const hash = str.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
    return colors[hash % colors.length];
};

function Assets() {
    const navigate = useNavigate();
    const [view, setView] = useState('list');
    const [assets, setAssets] = useState({});
    const [wallet, setWallet] = useState(null);
    const [nativeBalance, setNativeBalance] = useState(0);

    // UI
    const [searchTerm, setSearchTerm] = useState('');
    const [layout, setLayout] = useState('grid');
    const [sortBy, setSortBy] = useState('balance');

    // Create
    const [tokenName, setTokenName] = useState('');
    const [supply, setSupply] = useState('');
    const [error, setError] = useState(null);
    const [success, setSuccess] = useState(null);
    const [processing, setProcessing] = useState(false);

    useEffect(() => { loadWallet(); }, []);

    const loadWallet = () => {
        const storedKey = localStorage.getItem('volt_priv_key');
        const storedAddr = localStorage.getItem('volt_address');
        if (storedKey && storedAddr) {
            setWallet({ address: storedAddr, privateKey: storedKey });
            fetchData(storedAddr);
        }
    };

    const fetchData = async (address) => {
        try {
            const balRes = await axios.post(API_URL, { command: "get_balance", address });
            if (balRes.data.status === 'success') {
                setNativeBalance(balRes.data.data.balance);
            }
            const assetRes = await axios.post(API_URL, { command: "get_assets", address });
            if (assetRes.data.status === 'success') {
                setAssets(assetRes.data.data.assets);
            }
        } catch (e) { console.error(e); }
    };

    const handleIssue = async () => {
        if (!tokenName || !supply) return setError("Please fill all fields");
        setProcessing(true);
        try {
            // Get Nonce
            const nonceRes = await axios.post(API_URL, { command: "get_balance", address: wallet.address });
            const nonce = nonceRes.data.data.nonce || 0;

            const tx = {
                sender: wallet.address,
                receiver: wallet.address, // Issue to self
                amount: parseInt(supply),
                token: tokenName.toUpperCase(),
                tx_type: "IssueToken",
                nonce: nonce + 1,
                fee: 100000000, // 1.0 VLT to prevent spam
                timestamp: Math.floor(Date.now() / 1000),
                script_pub_key: { code: [], type: "P2PKH" },
                script_sig: { code: [], type: "P2PKH" },
                price: 0
            };

            const signature = signTransaction(tx, wallet.privateKey);
            tx.signature = signature;

            const res = await axios.post(API_URL, { command: "broadcast_transaction", data: tx });
            if (res.data.status === 'success') {
                setSuccess(`Token ${tokenName} Issue Broadcasted!`);
                setView('list');
                setTimeout(() => fetchData(wallet.address), 2000);
            } else { setError(res.data.message); }
        } catch (e) { setError("Failed to issue"); }
        setProcessing(false);
    };

    const handleBurn = async (token, amount) => {
        if (!window.confirm(`Burn ${amount} ${token}? UNDOABLE.`)) return;
        setProcessing(true);
        try {
            // Get Nonce
            const nonceRes = await axios.post(API_URL, { command: "get_balance", address: wallet.address });
            const nonce = nonceRes.data.data.nonce || 0;

            const tx = {
                sender: wallet.address,
                receiver: "0000000000000000000000000000000000000000", // Burn Address
                amount: parseInt(amount),
                token: token,
                tx_type: "Burn",
                nonce: nonce + 1,
                fee: 100000,
                timestamp: Math.floor(Date.now() / 1000),
                script_pub_key: { code: [], type: "P2PKH" },
                script_sig: { code: [], type: "P2PKH" },
                price: 0
            };

            const signature = signTransaction(tx, wallet.privateKey);
            tx.signature = signature;

            const res = await axios.post(API_URL, { command: "broadcast_transaction", data: tx });
            if (res.data.status === 'success') {
                setSuccess(`Burned ${amount} ${token} 🔥`);
                setTimeout(() => fetchData(wallet.address), 2000);
            } else { setError(res.data.message); }
        } catch (e) { setError("Burn Failed"); }
        setProcessing(false);
    };

    if (!wallet) return (
        <div style={{ textAlign: 'center', padding: '50px' }}>
            <h2>Wallet Not Connected</h2>
            <button className="btn btn-primary" onClick={() => navigate('/wallet')}>Go to Wallet</button>
        </div>
    );

    // Filter AND Sort
    const processedAssets = Object.entries(assets)
        .filter(([symbol]) => symbol !== 'VLT')
        .filter(([symbol]) => symbol.toLowerCase().includes(searchTerm.toLowerCase()))
        .sort(([symA, amtA], [symB, amtB]) => {
            if (sortBy === 'name') return symA.localeCompare(symB);
            if (sortBy === 'balance') return amtB - amtA; // Descending
            return 0;
        });

    return (
        <div className="container" style={{ maxWidth: '900px' }}>
            <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '30px' }}>
                <div>
                    <h2>Your Assets 💎</h2>
                    <p style={{ color: '#94a3b8' }}>Manage and trade your diverse portfolio</p>
                </div>
                <button className="btn btn-primary" onClick={() => setView('create')} style={{ width: 'auto' }}>+ Issue Token</button>
            </header>

            {success && <div style={{ padding: '15px', background: '#064e3b', color: '#34d399', borderRadius: '8px', marginBottom: '20px' }}>{success}</div>}
            {error && <div style={{ padding: '15px', background: '#7f1d1d', color: '#f87171', borderRadius: '8px', marginBottom: '20px' }}>{error}</div>}

            {view === 'create' && (
                <div className="wallet-card" style={{ marginBottom: '40px', border: '1px solid #fbbf24' }}>
                    <h3 style={{ color: '#fbbf24' }}>Issue New Token</h3>
                    <p style={{ color: '#94a3b8', fontSize: '0.9rem' }}>Create your own cryptocurrency on the Volt Blockchain.</p>

                    <div style={{ textAlign: 'left' }}>
                        <label style={{ display: 'block', marginBottom: '5px' }}>Token Symbol (Max 5 chars)</label>
                        <input className="input-field" placeholder="e.g. GOLD" value={tokenName} onChange={e => setTokenName(e.target.value)} maxLength={5} style={{ width: '100%', padding: '10px', marginBottom: '15px', background: '#0f172a', border: '1px solid #334155', color: 'white', borderRadius: '5px' }} />

                        <label style={{ display: 'block', marginBottom: '5px' }}>Total Supply</label>
                        <input className="input-field" type="number" placeholder="e.g. 1000000" value={supply} onChange={e => setSupply(e.target.value)} style={{ width: '100%', padding: '10px', marginBottom: '15px', background: '#0f172a', border: '1px solid #334155', color: 'white', borderRadius: '5px' }} />

                        <div style={{ display: 'flex', gap: '10px' }}>
                            <button className="btn btn-secondary" onClick={() => setView('list')}>Cancel</button>
                            <button className="btn btn-primary" onClick={handleIssue} disabled={processing}>{processing ? 'Signing...' : 'Issue Token (Fee: 1.00 VLT)'}</button>
                        </div>
                    </div>
                </div>
            )}

            {/* Toolbar: Search | Sort | Layout */}
            <div style={{ display: 'flex', gap: '10px', marginBottom: '20px', flexWrap: 'wrap' }}>
                <input
                    type="text"
                    placeholder="🔍 Search..."
                    value={searchTerm}
                    onChange={(e) => setSearchTerm(e.target.value)}
                    style={{
                        flex: 1, padding: '12px', borderRadius: '8px',
                        background: '#1e293b', border: '1px solid #334155', color: 'white'
                    }}
                />

                <select
                    value={sortBy}
                    onChange={e => setSortBy(e.target.value)}
                    style={{ padding: '0 15px', borderRadius: '8px', background: '#1e293b', border: '1px solid #334155', color: 'white', cursor: 'pointer' }}
                >
                    <option value="balance">Sort: Balance (High)</option>
                    <option value="name">Sort: Name (A-Z)</option>
                </select>

                <div style={{ display: 'flex', background: '#1e293b', borderRadius: '8px', border: '1px solid #334155', overflow: 'hidden' }}>
                    <button
                        onClick={() => setLayout('grid')}
                        style={{ padding: '10px 15px', background: layout === 'grid' ? '#3b82f6' : 'transparent', border: 'none', color: 'white', cursor: 'pointer' }}
                    >
                        Grid ▣
                    </button>
                    <button
                        onClick={() => setLayout('table')}
                        style={{ padding: '10px 15px', background: layout === 'table' ? '#3b82f6' : 'transparent', border: 'none', color: 'white', cursor: 'pointer' }}
                    >
                        List ☰
                    </button>
                </div>
            </div>

            {/* Grid View */}
            {layout === 'grid' && (
                <div className="stats-grid">
                    {/* Native VLT Card */}
                    <div className="card" style={{ background: 'linear-gradient(135deg, #1e293b 0%, #0f172a 100%)', border: '1px solid #3b82f6' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '15px', marginBottom: '15px' }}>
                            <div style={{ width: '45px', height: '45px', borderRadius: '50%', background: '#3b82f6', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: '1.2rem' }}>⚡</div>
                            <div><h3 style={{ margin: 0 }}>VLT</h3><small style={{ color: '#94a3b8' }}>Native</small></div>
                        </div>
                        <p style={{ fontSize: '1.5rem', fontWeight: 'bold' }}>{(nativeBalance / 100000000).toLocaleString()}</p>
                        <div style={{ display: 'flex', gap: '10px', marginTop: '15px' }}>
                            <button className="btn btn-primary" onClick={() => navigate('/wallet')} style={{ flex: 1 }}>Send</button>
                            <button className="btn btn-secondary" onClick={() => navigate('/dex')} style={{ flex: 1 }}>Trade</button>
                        </div>
                    </div>

                    {processedAssets.map(([symbol, amount]) => (
                        <div className="card" key={symbol}>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '15px', marginBottom: '15px' }}>
                                <div style={{
                                    width: '45px', height: '45px', borderRadius: '50%',
                                    background: getColor(symbol), color: 'white',
                                    display: 'flex', alignItems: 'center', justifyContent: 'center', fontWeight: 'bold', fontSize: '1.2rem'
                                }}>{symbol[0]}</div>
                                <div><h3 style={{ margin: 0 }}>{symbol}</h3><small style={{ color: '#94a3b8' }}>Asset</small></div>
                            </div>
                            <p style={{ fontSize: '1.5rem', fontWeight: 'bold' }}>{amount}</p>
                            <div style={{ display: 'flex', gap: '5px', marginTop: '15px' }}>
                                <button className="btn btn-secondary" onClick={() => navigate('/dex')} style={{ flex: 1, fontSize: '0.9rem' }}>Trade</button>
                                <button className="btn btn-secondary" onClick={() => handleBurn(symbol, amount)} style={{ flex: 1, fontSize: '0.9rem', background: '#7f1d1d', color: '#fca5a5', border: 'none' }}>Burn</button>
                            </div>
                        </div>
                    ))}
                </div>
            )}

            {/* List/Table View */}
            {layout === 'table' && (
                <div style={{ background: '#1e293b', borderRadius: '12px', overflow: 'hidden', border: '1px solid #334155' }}>
                    <div className="responsive-table-header" style={{ gridTemplateColumns: '1.5fr 1fr 1fr 1fr' }}>
                        <div>Asset</div>
                        <div>Type</div>
                        <div>Balance</div>
                        <div style={{ textAlign: 'right' }}>Actions</div>
                    </div>

                    {/* VLT Row */}
                    <div className="responsive-table-row" style={{ gridTemplateColumns: '1.5fr 1fr 1fr 1fr' }}>
                        <div className="responsive-table-cell" data-label="Asset">
                            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                                <div style={{ width: '30px', height: '30px', borderRadius: '50%', background: '#3b82f6', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>⚡</div>
                                VLT
                            </div>
                        </div>
                        <div className="responsive-table-cell" data-label="Type">
                            <span style={{ color: '#3b82f6' }}>Native</span>
                        </div>
                        <div className="responsive-table-cell" data-label="Balance">
                            <span style={{ fontWeight: 'bold', fontSize: '1.1rem' }}>{(nativeBalance / 100000000).toLocaleString()}</span>
                        </div>
                        <div className="responsive-table-cell" data-label="Actions">
                            <div style={{ textAlign: 'right' }}>
                                <button onClick={() => navigate('/wallet')} style={{ background: 'transparent', border: '1px solid #3b82f6', color: '#3b82f6', borderRadius: '5px', padding: '5px 10px', cursor: 'pointer' }}>Send</button>
                            </div>
                        </div>
                    </div>

                    {processedAssets.map(([symbol, amount]) => (
                        <div key={symbol} className="responsive-table-row" style={{ gridTemplateColumns: '1.5fr 1fr 1fr 1fr' }}>
                            <div className="responsive-table-cell" data-label="Asset">
                                <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                                    <div style={{
                                        width: '30px', height: '30px', borderRadius: '50%',
                                        background: getColor(symbol), color: 'white',
                                        display: 'flex', alignItems: 'center', justifyContent: 'center', fontWeight: 'bold'
                                    }}>{symbol[0]}</div>
                                    {symbol}
                                </div>
                            </div>
                            <div className="responsive-table-cell" data-label="Type">
                                <span style={{ color: '#94a3b8' }}>Asset</span>
                            </div>
                            <div className="responsive-table-cell" data-label="Balance">
                                <span style={{ fontWeight: 'bold' }}>{amount}</span>
                            </div>
                            <div className="responsive-table-cell" data-label="Actions">
                                <div style={{ textAlign: 'right', display: 'flex', gap: '5px', justifyContent: 'flex-end', width: '100%' }}>
                                    <button onClick={() => navigate('/dex')} style={{ background: 'transparent', border: '1px solid #94a3b8', color: '#94a3b8', borderRadius: '5px', padding: '5px 10px', cursor: 'pointer' }}>Trade</button>
                                    <button onClick={() => handleBurn(symbol, amount)} style={{ background: 'transparent', border: '1px solid #7f1d1d', color: '#fca5a5', borderRadius: '5px', padding: '5px 10px', cursor: 'pointer' }}>Burn</button>
                                </div>
                            </div>
                        </div>
                    ))}
                </div>
            )}

            {processedAssets.length === 0 && Object.keys(assets).length > 1 && (
                <div style={{ textAlign: 'center', padding: '40px', color: '#64748b' }}>
                    <p>No assets found matching "{searchTerm}"</p>
                </div>
            )}
        </div>
    );
}

export default Assets;
