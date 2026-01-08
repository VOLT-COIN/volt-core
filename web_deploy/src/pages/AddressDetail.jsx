import { useParams } from 'react-router-dom';
import { useState, useEffect } from 'react';
import axios from 'axios';
import { getApiConfig } from '../utils/apiConfig';
import { API_URL } from '../config';

function AddressDetail() {
    const { id } = useParams();
    const [data, setData] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);

    useEffect(() => {
        fetchData();
    }, [id]);

    const fetchData = async () => {
        try {
            let balance = 0;
            let staked = 0;
            let txs = [];

            try {
                const resBal = await axios.post(API_URL, { command: "get_balance", address: id }, getApiConfig());
                if (resBal.data.status === 'success') {
                    balance = resBal.data.data.balance;
                    staked = resBal.data.data.staked;
                }
            } catch (e) { setError(e.message); }

            try {
                const resTxs = await axios.post(API_URL, { command: "get_address_history", address: id }, getApiConfig());
                if (resTxs.data.status === 'success') {
                    txs = resTxs.data.data.history;
                }
            } catch (e) { console.error("History fetch error:", e); }

            setData({
                address: id,
                balance: balance / 100000000,
                staked: staked / 100000000,
                txCount: txs.length,
                history: txs.map(tx => ({
                    hash: tx.hash,
                    type: tx.receiver === id ? 'IN' : 'OUT',
                    amount: tx.amount / 100000000,
                    time: new Date(tx.timestamp * 1000).toLocaleString(),
                    block: tx.block_index,
                    status: tx.confirmations >= 6 ? 'Confirmed' : 'Pending',
                    token: tx.token
                }))
            });
        } catch (e) { console.error(e); }
        setLoading(false);
    };

    if (error) return <div className="container" style={{ textAlign: 'center', padding: '50px' }}><h2 className="gradient-text">Error: {error}</h2><p style={{ color: '#888' }}>Ensure your node is synchronized.</p></div>;
    if (loading) return <div className="container" style={{ textAlign: 'center', padding: '50px' }}>Loading...</div>;
    if (!data) return null;

    return (
        <div className="container" style={{ padding: '40px 20px', maxWidth: '1000px' }}>
            <h1 className="gradient-text">Address Insights</h1>
            <div style={{ fontFamily: 'monospace', wordBreak: 'break-all', fontSize: '1.1rem', marginBottom: '40px' }}>{data.address}</div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '20px', marginBottom: '40px' }}>
                <div className="glass-card" style={{ padding: '30px', textAlign: 'center' }}>
                    <div style={{ color: '#888', marginBottom: '10px', fontSize: '0.8rem' }}>AVAILABLE BALANCE</div>
                    <div style={{ fontSize: '2rem', fontWeight: 'bold', color: '#00f2ea' }}>{data.balance.toLocaleString()} VLT</div>
                </div>
                <div className="glass-card" style={{ padding: '30px', textAlign: 'center' }}>
                    <div style={{ color: '#888', marginBottom: '10px', fontSize: '0.8rem' }}>STAKED BALANCE</div>
                    <div style={{ fontSize: '2rem', fontWeight: 'bold', color: '#f472b6' }}>{data.staked.toLocaleString()} VLT</div>
                </div>
            </div>

            <h3 style={{ marginBottom: '20px' }}>Transaction History ({data.txCount})</h3>
            <div className="glass-card" style={{ padding: '0', overflow: 'hidden' }}>
                {data.history.length === 0 ? (
                    <div style={{ padding: '40px', textAlign: 'center', color: '#555' }}>No transactions found for this address.</div>
                ) : (
                    data.history.map((tx, i) => (
                        <div key={i} className="hover-row" style={{ padding: '20px', borderBottom: '1px solid rgba(255,255,255,0.05)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '5px' }}>
                                <div style={{ fontFamily: 'monospace', color: '#fff', fontSize: '0.9rem' }}>{(tx.hash || "").substr(0, 32)}...</div>
                                <div style={{ fontSize: '0.75rem', color: '#888' }}>
                                    {tx.time} • Block #{tx.block} •
                                    <span style={{ color: tx.status === 'Confirmed' ? '#4ade80' : '#fbbf24', marginLeft: '5px' }}>{tx.status}</span>
                                </div>
                            </div>
                            <div style={{ textAlign: 'right' }}>
                                <div style={{ fontWeight: 'bold', color: tx.type === 'IN' ? '#4ade80' : '#f43f5e', fontSize: '1.1rem' }}>
                                    {tx.type === 'IN' ? '+' : '-'}{tx.amount} {tx.token}
                                </div>
                                <div style={{ fontSize: '0.7rem', color: tx.type === 'IN' ? 'rgba(74, 222, 128, 0.4)' : 'rgba(244, 63, 94, 0.4)' }}>
                                    {tx.type === 'IN' ? 'RECEIVED' : 'SENT'}
                                </div>
                            </div>
                        </div>
                    ))
                )}
            </div>
        </div>
    );
}

export default AddressDetail;
