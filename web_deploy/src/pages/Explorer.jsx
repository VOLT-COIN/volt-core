import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import axios from 'axios';
import { API_URL } from '../config';
import { getApiConfig } from '../utils/apiConfig';

const timeAgo = (date) => {
    const seconds = Math.floor((new Date() - date) / 1000);
    let interval = seconds / 31536000;
    if (interval > 1) return Math.floor(interval) + " years ago";
    interval = seconds / 2592000;
    if (interval > 1) return Math.floor(interval) + " months ago";
    interval = seconds / 86400;
    if (interval > 1) return Math.floor(interval) + " days ago";
    interval = seconds / 3600;
    if (interval > 1) return Math.floor(interval) + " hours ago";
    interval = seconds / 60;
    if (interval > 1) return Math.floor(interval) + " minutes ago";
    return Math.floor(seconds) + " seconds ago";
};

function Explorer() {
    const navigate = useNavigate();
    const [search, setSearch] = useState('');
    const [blocks, setBlocks] = useState([]);
    const [txs, setTxs] = useState([]);
    const [stats, setStats] = useState({ height: 0, difficulty: 0, peers: 0 });

    useEffect(() => {
        fetchChainData();
        const intv = setInterval(fetchChainData, 10000);
        return () => clearInterval(intv);
    }, []);

    const fetchChainData = async () => {
        try {
            const res = await axios.post(API_URL, { command: "get_chain_info" }, getApiConfig());
            if (res.data.status === 'success') {
                const data = res.data.data;
                setStats(data);

                const currentHeight = data.height;
                const bRes = await axios.post(API_URL, { command: "get_blocks", start_index: Math.max(0, data.height - 10) }, getApiConfig());
                if (bRes.data.status === 'success') {
                    setBlocks(bRes.data.data.blocks.map(b => ({
                        ...b,
                        height: b.index,
                        time: timeAgo(new Date(b.timestamp * 1000)),
                        confirmations: currentHeight - b.index
                    })));
                }

                const txRes = await axios.post(API_URL, { command: "get_recent_txs" }, getApiConfig());
                if (txRes.data.status === 'success') {
                    setTxs(txRes.data.data.transactions);
                }
            }
        } catch (e) { console.error("Fetch error", e); }
    };

    const handleSearch = () => {
        if (!search) return;
        const s = search.trim();
        if (/^\d+$/.test(s)) {
            navigate("/block/" + s);
        } else if (s.length >= 64) {
            navigate("/tx/" + s);
        } else if (s.startsWith("V") && s.length > 20) {
            navigate("/address/" + s);
        } else {
            navigate("/tx/" + s);
        }
    };

    const formatMetric = (num) => {
        if (!num) return '0';
        if (num >= 1e12) return (num / 1e12).toFixed(2) + ' T';
        if (num >= 1e9) return (num / 1e9).toFixed(2) + ' B';
        if (num >= 1e6) return (num / 1e6).toFixed(2) + ' M';
        if (num >= 1e3) return (num / 1e3).toFixed(2) + ' k';
        return num.toLocaleString();
    };

    return (
        <div className="container" style={{ padding: '40px 20px', maxWidth: '1200px', margin: '0 auto' }}>
            <div style={{ textAlign: 'center', marginBottom: '60px' }}>
                <h1 style={{ fontSize: '3.5rem', marginBottom: '20px', letterSpacing: '-1px' }}>
                    <span className="gradient-text">VOLTSCAN</span> <span style={{ textShadow: '0 0 30px rgba(0, 242, 234, 0.4)' }}>⚡</span>
                </h1>
                <p style={{ color: '#888', marginBottom: '30px' }}>The decentralized ledger for the Volt network.</p>
                <div style={{ maxWidth: '700px', margin: '0 auto', display: 'flex', gap: '10px', background: 'rgba(255,255,255,0.03)', padding: '5px', borderRadius: '12px', border: '1px solid rgba(255,255,255,0.1)' }}>
                    <input
                        className="glass-input"
                        placeholder="Search Blocks, Transactions or Addresses..."
                        value={search}
                        onChange={e => setSearch(e.target.value)}
                        onKeyDown={e => e.key === 'Enter' && handleSearch()}
                        style={{ width: '100%', padding: '15px 20px', border: 'none', background: 'transparent' }}
                    />
                    <button className="btn btn-primary" onClick={handleSearch} style={{ padding: '0 30px', borderRadius: '8px' }}>SEARCH</button>
                </div>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', gap: '20px', marginBottom: '60px' }}>
                <div className="glass-card" style={{ padding: '25px', borderLeft: '4px solid #00f2ea' }}>
                    <p style={{ color: '#888', fontSize: '0.8rem', marginBottom: '10px' }}>CHAIN HEIGHT</p>
                    <h2 style={{ margin: 0, fontSize: '1.8rem' }}>#{stats.height.toLocaleString()}</h2>
                </div>
                <div className="glass-card" style={{ padding: '25px', borderLeft: '4px solid #ff0055' }}>
                    <p style={{ color: '#888', fontSize: '0.8rem', marginBottom: '10px' }}>NETWORK DIFFICULTY</p>
                    <h2 style={{ margin: 0, fontSize: '1.8rem' }}>{formatMetric(stats.difficulty)}</h2>
                </div>
                <div className="glass-card" style={{ padding: '25px', borderLeft: '4px solid #fbbf24' }}>
                    <p style={{ color: '#888', fontSize: '0.8rem', marginBottom: '10px' }}>ACTIVE PEERS</p>
                    <h2 style={{ margin: 0, fontSize: '1.8rem' }}>{stats.peers} Connected</h2>
                </div>
                <div className="glass-card" style={{ padding: '25px', borderLeft: '4px solid #34d399' }}>
                    <p style={{ color: '#888', fontSize: '0.8rem', marginBottom: '10px' }}>TOTAL SUPPLY</p>
                    <h2 style={{ margin: 0, fontSize: '1.8rem' }}>{(stats.height * 50).toLocaleString()} VLT</h2>
                </div>
            </div>

            <div className="mobile-stack" style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '30px' }}>
                <div className="glass-card" style={{ padding: '0' }}>
                    <div style={{ padding: '20px', borderBottom: '1px solid rgba(255,255,255,0.1)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <h3 style={{ margin: 0 }}>Latest Blocks</h3>
                        <span style={{ fontSize: '0.8rem', color: '#888' }}>Last 10</span>
                    </div>
                    {blocks.map(b => (
                        <div key={b.height} className="hover-row" onClick={() => navigate('/block/' + b.height)} style={{ display: 'flex', justifyContent: 'space-between', padding: '15px 20px', borderBottom: '1px solid rgba(255,255,255,0.05)', cursor: 'pointer' }}>
                            <div>
                                <div style={{ color: '#00f2ea', fontWeight: 'bold' }}>#{b.height}</div>
                                <div style={{ fontSize: '0.75rem', color: '#666' }}>{b.time}</div>
                            </div>
                            <div style={{ textAlign: 'right' }}>
                                <div style={{ color: '#fff' }}>{b.transactions} Transactions</div>
                                <div style={{ fontSize: '0.75rem', color: '#4ade80' }}>{b.confirmations} Confirms</div>
                            </div>
                        </div>
                    ))}
                </div>

                <div className="glass-card" style={{ padding: '0' }}>
                    <div style={{ padding: '20px', borderBottom: '1px solid rgba(255,255,255,0.1)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <h3 style={{ margin: 0 }}>Recent Transactions</h3>
                        <span style={{ fontSize: '0.8rem', color: '#888' }}>Mempool</span>
                    </div>
                    {txs
                        .filter(t => t.receiver !== 'LOCKED' && !t.receiver?.includes('LOCKED'))
                        .map(t => (
                            <div key={t.hash} onClick={() => navigate('/tx/' + t.hash)} className="hover-row" style={{ padding: '15px 20px', borderBottom: '1px solid rgba(255,255,255,0.05)', cursor: 'pointer' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '5px' }}>
                                    <span style={{ color: '#ff0055', fontFamily: 'monospace', fontSize: '0.85rem' }}>{t.hash ? t.hash.substr(0, 16) : '???'}...</span>
                                    {t.tx_type === 'DeployContract' || (typeof t.tx_type === 'object' && t.tx_type.DeployContract) ? (
                                        <span style={{ color: '#ec4899', fontWeight: 'bold' }}>CONTRACT DEPLOY</span>
                                    ) : t.tx_type === 'CallContract' || (typeof t.tx_type === 'object' && t.tx_type.CallContract) ? (
                                        <span style={{ color: '#d946ef', fontWeight: 'bold' }}>CONTRACT CALL</span>
                                    ) : (
                                        <span style={{ color: '#fff', fontWeight: 'bold' }}>{(t.amount / 100000000).toLocaleString()} VLT</span>
                                    )}
                                </div>
                                <div style={{ color: '#888', fontSize: '0.75rem', display: 'flex', gap: '10px' }}>
                                    <span>From: {t.sender ? t.sender.substr(0, 8) : 'System'}...</span>
                                    <span>To: {t.receiver ? t.receiver.substr(0, 8) : 'Unknown'}...</span>
                                </div>
                            </div>
                        ))}
                </div>
            </div>
        </div>
    );
}

export default Explorer;
