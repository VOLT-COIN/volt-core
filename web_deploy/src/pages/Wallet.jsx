import { useState, useEffect } from 'react';
import axios from 'axios';
import { createWallet, restoreWallet, keysFromMnemonic, signTransaction } from '../utils/wallet';
import { getApiConfig, getPayload } from '../utils/apiConfig';
import QRCode from "react-qr-code";
import AddressBook from '../components/AddressBook';

const API_URL = '/api/rpc';

// Moved Wrapper outside to prevent re-renders losing focus
const Wrapper = ({ children, title, error, success }) => (
    <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '60vh' }}>
        <div className="glass-card" style={{ width: '100%', maxWidth: '500px', textAlign: 'center' }}>
            <h2 className="gradient-text" style={{ fontSize: '2rem', marginBottom: '30px' }}>{title}</h2>
            {error && <div style={{ color: '#ff0055', background: 'rgba(255,0,85,0.1)', padding: '10px', borderRadius: '8px', marginBottom: '20px' }}>{error}</div>}
            {success && <div style={{ color: '#00f2ea', background: 'rgba(0,242,234,0.1)', padding: '10px', borderRadius: '8px', marginBottom: '20px' }}>{success}</div>}
            {children}
        </div>
    </div>
);

function Wallet() {
    const [view, setView] = useState('loading');
    const [wallet, setWallet] = useState(null);
    const [mnemonic, setMnemonic] = useState('');
    const [password, setPassword] = useState('');
    const [error, setError] = useState(null);
    const [success, setSuccess] = useState(null);
    const [balance, setBalance] = useState(0);
    const [assets, setAssets] = useState({});
    const [selectedToken, setSelectedToken] = useState('VLT');
    const [recipient, setRecipient] = useState('');
    const [amount, setAmount] = useState('');
    const [sending, setSending] = useState(false);
    const [transactions, setTransactions] = useState([]);
    const [feeRate, setFeeRate] = useState(25); // Default 25 sat/vB
    const VIRTUAL_SIZE = 250; // Approximated vBytes for standard tx

    // Auto-Logout Config (5 Minutes)
    const AUTO_LOGOUT_MS = 5 * 60 * 1000;

    useEffect(() => {
        loadWallet();

        let timer;
        const resetTimer = () => {
            if (timer) clearTimeout(timer);
            timer = setTimeout(() => {
                console.log("Auto-logout due to inactivity");
                handleLogout();
            }, AUTO_LOGOUT_MS);
        };

        // Listen to events
        window.addEventListener('mousemove', resetTimer);
        window.addEventListener('keypress', resetTimer);
        window.addEventListener('click', resetTimer);

        // Init
        resetTimer();

        return () => {
            if (timer) clearTimeout(timer);
            window.removeEventListener('mousemove', resetTimer);
            window.removeEventListener('keypress', resetTimer);
            window.removeEventListener('click', resetTimer);
        };
    }, []);

    const loadWallet = () => {
        const storedKey = localStorage.getItem('volt_priv_key');
        const storedAddr = localStorage.getItem('volt_address');
        if (storedKey && storedAddr) {
            setWallet({ address: storedAddr, privateKey: storedKey });
            fetchBalance(storedAddr);
            setView('dashboard');
        } else {
            setView('entry');
        }
    };



    const fetchBalance = async (address) => {
        try {
            // Pass getApiConfig() which now includes params
            const res = await axios.post(API_URL, getPayload("get_balance", { address }), getApiConfig());

            if (res.data.status === 'success') {
                setBalance(res.data.data.balance);
            }
            const assetRes = await axios.post(API_URL, getPayload("get_assets", { address }), getApiConfig());
            if (assetRes.data.status === 'success') setAssets(assetRes.data.data.assets);

            // Fetch Transactions
            const txRes = await axios.post(API_URL, getPayload("get_recent_txs"), getApiConfig());
            if (txRes.data.status === 'success') {
                // Filter my txs
                const myTxs = txRes.data.data.transactions.filter(tx => tx.sender === address || tx.receiver === address);
                setTransactions(myTxs.slice(0, 10)); // Show last 10
            }
        } catch (e) {
            console.error("Balance fetch error", e);
            // Show error in UI only if it's explicit user action, but here it's auto.
            // Maybe set a small indicator?
        }
    };

    const handleCreate = () => {
        const w = createWallet();
        setMnemonic(w.mnemonic);
        // We temporarily store the key to auto-login IF they confirm
        setWallet(w);
        setView('create');
    };

    // ... handleSend (Broadcast) ...
    const handleBroadcast = async (tx) => {
        const res = await axios.post(API_URL, getPayload("broadcast_transaction", { data: tx }), getApiConfig());
        return res;
    };

    const handleImport = () => {
        const input = mnemonic.trim();
        let derivedWallet = null;
        if (input.split(' ').length > 1) {
            derivedWallet = restoreWallet(input);
        } else if (input.length === 64) {
            // Private Key Direct
            try {
                // We assume input is valid key if length 64 hex
                // But we don't have easy restore from key function imported
                setError("Please enter a Seed Phrase (12 words).");
                return;
            } catch (e) { }
        }

        if (derivedWallet) {
            localStorage.setItem('volt_priv_key', derivedWallet.privateKey);
            localStorage.setItem('volt_address', derivedWallet.address);
            setWallet(derivedWallet);
            fetchBalance(derivedWallet.address);
            setView('dashboard');
            setMnemonic('');
        } else {
            setError("Invalid Seed Phrase");
        }
    };

    const handleLogout = () => {
        localStorage.removeItem('volt_priv_key');
        localStorage.removeItem('volt_address');
        setWallet(null);
        setView('entry');
    };

    const handleSend = async () => {
        setSending(true);
        try {
            // Get Nonce
            const nonceRes = await axios.post(API_URL, { command: "get_balance", address: wallet.address }, getApiConfig());
            const nonce = nonceRes.data.data.nonce || 0;

            const sendAmount = selectedToken === 'VLT' ? Math.floor(parseFloat(amount) * 100000000) : parseFloat(amount);

            const tx = {
                sender: wallet.address,
                receiver: recipient,
                amount: sendAmount,
                token: selectedToken,
                tx_type: "Transfer",
                nonce: nonce + 1,
                fee: VIRTUAL_SIZE * feeRate,
                timestamp: Math.floor(Date.now() / 1000),
                script_pub_key: { code: [], type: "P2PKH" },
                script_sig: { code: [], type: "P2PKH" },
                price: 0
            };

            const signature = signTransaction(tx, wallet.privateKey);
            tx.signature = signature;

            const res = await axios.post(API_URL, getPayload("broadcast_transaction", { data: tx }), getApiConfig());

            if (res.data.status === 'success') {
                setSuccess(`Broadcasted ${amount} ${selectedToken}!`);
                setRecipient(''); setAmount('');
                setTimeout(() => setSuccess(null), 3000);
                setTimeout(() => fetchBalance(wallet.address), 2000);
                setView('dashboard');
            } else { setError(res.data.message); }

        } catch (e) { console.error(e); setError("Failed to send"); }
        setSending(false);
    };

    if (view === 'entry') return (
        <Wrapper title="Volt Web Wallet" error={error} success={success}>

            <div style={{ display: 'grid', gap: '20px' }}>
                <button className="glass-card" onClick={handleCreate} style={{ cursor: 'pointer', textAlign: 'left', padding: '20px' }}>
                    <div style={{ fontSize: '2rem', marginBottom: '10px' }}>🆕</div>
                    <h3>Create New Wallet</h3>
                    <p style={{ color: '#888', margin: 0 }}>Generate a unique private key locally.</p>
                </button>
                <button className="glass-card" onClick={() => setView('import')} style={{ cursor: 'pointer', textAlign: 'left', padding: '20px' }}>
                    <div style={{ fontSize: '2rem', marginBottom: '10px' }}>🔑</div>
                    <h3>Access Wallet</h3>
                    <p style={{ color: '#888', margin: 0 }}>Enter Private Key or Mnemonic Hash.</p>
                </button>
            </div>
        </Wrapper>
    );



    if (view === 'create') return (
        <Wrapper title="Secret Recovery Phrase" error={error} success={success}>
            <p style={{ color: '#ff0055', fontWeight: 'bold' }}>SAVE THESE 12 WORDS. THEY ARE THE ONLY WAY TO RESTORE YOUR WALLET.</p>
            <div style={{ background: 'rgba(0,0,0,0.3)', border: '1px dashed #00f2ea', padding: '20px', borderRadius: '12px', margin: '20px 0', fontFamily: 'monospace', fontSize: '1.2rem', wordBreak: 'break-word', lineHeight: '1.6' }}>
                {mnemonic}
            </div>
            <button className="btn btn-primary" style={{ width: '100%' }} onClick={() => {
                localStorage.setItem('volt_priv_key', wallet.privateKey);
                localStorage.setItem('volt_address', wallet.address);
                setView('dashboard');
            }}>I Saved It &rarr; Access Wallet</button>
        </Wrapper>
    );



    if (view === 'import') return (
        <Wrapper title="Restore Wallet" error={error} success={success}>
            <textarea
                placeholder="Enter 12-word Seed Phrase..."
                value={mnemonic}
                onChange={e => setMnemonic(e.target.value)}
                style={{ width: '100%', height: '100px', background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: '12px', color: 'white', padding: '15px', marginBottom: '15px' }}
            />
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
                <button className="btn btn-secondary" onClick={() => setView('entry')}>Cancel</button>
                <button className="btn btn-primary" onClick={handleImport}>Unlock</button>
            </div>
        </Wrapper>
    );

    if (view === 'send') return (
        <Wrapper title="Send Funds" error={error} success={success}>
            <div style={{ textAlign: 'left', marginBottom: '20px' }}>
                <label style={{ color: '#888', marginLeft: '10px' }}>Recipient</label>
                <input placeholder="Volt Address (Hex)" value={recipient} onChange={e => setRecipient(e.target.value)} />
                <AddressBook onSelect={(addr) => setRecipient(addr)} />
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: '10px', marginBottom: '30px' }}>
                <div>
                    <label style={{ color: '#888', marginLeft: '10px' }}>Amount</label>
                    <input type="number" placeholder="0.00" value={amount} onChange={e => setAmount(e.target.value)} />
                </div>
                <div>
                    <label style={{ color: '#888', marginLeft: '10px' }}>Asset</label>
                    <select value={selectedToken} onChange={(e) => setSelectedToken(e.target.value)} style={{ width: '100%', padding: '15px', borderRadius: '50px', background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.1)', color: 'white', marginTop: '3px' }}>
                        <option value="VLT">VLT</option>
                    </select>
                </div>
            </div>

            {/* Bitcoin-like Fee Selector */}
            <div style={{ textAlign: 'left', marginBottom: '30px' }}>
                <label style={{ color: '#888', marginLeft: '10px' }}>Network Fee ({feeRate} sat/vB)</label>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '10px', marginTop: '5px' }}>
                    <div onClick={() => setFeeRate(10)} className={`fee-card ${feeRate === 10 ? 'active' : ''}`} style={{ cursor: 'pointer', padding: '10px', borderRadius: '12px', background: feeRate === 10 ? 'rgba(16, 185, 129, 0.2)' : 'rgba(255,255,255,0.05)', border: feeRate === 10 ? '1px solid #10b981' : '1px solid transparent', textAlign: 'center' }}>
                        <div style={{ fontSize: '0.8rem', color: '#fff' }}>Eco</div>
                        <div style={{ fontSize: '0.7rem', color: '#888' }}>10 sat/vB</div>
                    </div>
                    <div onClick={() => setFeeRate(25)} className={`fee-card ${feeRate === 25 ? 'active' : ''}`} style={{ cursor: 'pointer', padding: '10px', borderRadius: '12px', background: feeRate === 25 ? 'rgba(56, 189, 248, 0.2)' : 'rgba(255,255,255,0.05)', border: feeRate === 25 ? '1px solid #38bdf8' : '1px solid transparent', textAlign: 'center' }}>
                        <div style={{ fontSize: '0.8rem', color: '#fff' }}>Standard</div>
                        <div style={{ fontSize: '0.7rem', color: '#888' }}>25 sat/vB</div>
                    </div>
                    <div onClick={() => setFeeRate(100)} className={`fee-card ${feeRate === 100 ? 'active' : ''}`} style={{ cursor: 'pointer', padding: '10px', borderRadius: '12px', background: feeRate === 100 ? 'rgba(244, 114, 182, 0.2)' : 'rgba(255,255,255,0.05)', border: feeRate === 100 ? '1px solid #f472b6' : '1px solid transparent', textAlign: 'center' }}>
                        <div style={{ fontSize: '0.8rem', color: '#fff' }}>Fast</div>
                        <div style={{ fontSize: '0.7rem', color: '#888' }}>100 sat/vB</div>
                    </div>
                </div>
                <div style={{ fontSize: '0.8rem', color: '#666', marginTop: '5px', marginLeft: '10px' }}>
                    Total Fee: {(VIRTUAL_SIZE * feeRate).toLocaleString()} sats ({(VIRTUAL_SIZE * feeRate / 100000000).toFixed(8)} VLT)
                </div>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
                <button className="btn btn-secondary" onClick={() => setView('dashboard')}>Cancel</button>
                <button className="btn btn-primary" onClick={handleSend} disabled={sending}>{sending ? '...' : `Sign & Broadcast`}</button>
            </div>
        </Wrapper>
    );

    return (
        <Wrapper title="Web Wallet" error={error} success={success}>
            <div style={{ background: 'rgba(0,0,0,0.3)', borderRadius: '16px', padding: '20px', marginBottom: '30px', border: '1px solid rgba(255,255,255,0.05)' }}>
                <p style={{ color: '#888', fontSize: '0.9rem', marginBottom: '5px' }}>AVAILABLE BALANCE</p>
                <h1 style={{ fontSize: '3.5rem', margin: '0', color: '#fff' }}>
                    {(balance / 100000000).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 8 })} <span style={{ fontSize: '1.5rem', color: '#00f2ea' }}>VLT</span>
                </h1>
                <p style={{ fontFamily: 'monospace', color: '#666', marginTop: '10px', wordBreak: 'break-all', fontSize: '0.8rem' }} onClick={() => { navigator.clipboard.writeText(wallet?.address); alert("Copied!"); }} title="Click to Copy">
                    {wallet?.address}
                </p>
                <div style={{ background: 'white', padding: '10px', display: 'inline-block', borderRadius: '10px', marginTop: '15px' }}>
                    <QRCode value={wallet?.address || ""} size={128} />
                </div>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '15px', marginBottom: '40px' }}>
                <button className="btn btn-primary" onClick={() => setView('send')}>Send VLT</button>
                <div style={{ display: 'grid', gap: '10px' }}>
                    <button className="btn btn-secondary" onClick={() => window.location.href = '/contracts'}>Smart Contracts</button>
                    <button className="btn btn-secondary" onClick={handleLogout}>Logout</button>
                </div>
            </div>

            {/* TRANSACTION HISTORY */}
            <div style={{ textAlign: 'left' }}>
                <h3 style={{ color: '#94a3b8', marginBottom: '15px' }}>Recent Transactions</h3>
                <div style={{ background: '#1e293b', borderRadius: '12px', overflow: 'hidden', border: '1px solid #334155' }}>
                    {transactions.length === 0 ? (
                        <div style={{ padding: '20px', textAlign: 'center', color: '#666' }}>No recent transactions</div>
                    ) : (
                        <>
                            <div className="responsive-table-header" style={{ gridTemplateColumns: '1fr 1fr 1fr 1fr' }}>
                                <div>Type</div>
                                <div>TxHash</div>
                                <div>Amount</div>
                                <div style={{ textAlign: 'right' }}>Status</div>
                            </div>
                            {transactions.map(tx => (
                                <div key={tx.hash} className="responsive-table-row" style={{ gridTemplateColumns: '1fr 1fr 1fr 1fr' }}>
                                    <div className="responsive-table-cell" data-label="Type">
                                        <span style={{ color: tx.sender === wallet.address ? '#f59e0b' : '#10b981', fontWeight: 'bold' }}>
                                            {tx.sender === wallet.address ? 'OUT' : 'IN'}
                                        </span>
                                    </div>
                                    <div className="responsive-table-cell" data-label="TxHash">
                                        <span style={{ fontFamily: 'monospace', fontSize: '0.9rem', color: '#94a3b8' }}>
                                            {tx.hash ? tx.hash.substr(0, 8) + '...' : 'Pending'}
                                        </span>
                                    </div>
                                    <div className="responsive-table-cell" data-label="Amount">
                                        <span style={{ fontWeight: 'bold', color: '#fff' }}>
                                            {(tx.amount / 100000000).toLocaleString()} VLT
                                        </span>
                                    </div>
                                    <div className="responsive-table-cell" data-label="Status">
                                        <div style={{ textAlign: 'right', width: '100%' }}>
                                            <span style={{ background: 'rgba(16, 185, 129, 0.2)', color: '#10b981', padding: '2px 8px', borderRadius: '4px', fontSize: '0.8rem' }}>Confirmed</span>
                                        </div>
                                    </div>
                                </div>
                            ))}
                        </>
                    )}
                </div>
            </div>
        </Wrapper>
    );
}

export default Wallet;
