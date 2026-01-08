
import { useState, useEffect } from 'react';
import axios from 'axios';
import { signTransaction } from '../utils/wallet';
import Chart from '../components/Chart';

const API_URL = '/api/rpc';

function Dex() {
    const [wallet, setWallet] = useState(null);
    const [orders, setOrders] = useState([]);
    const [pair, setPair] = useState('USDT');
    const [side, setSide] = useState('BUY');
    const [amount, setAmount] = useState('');
    const [price, setPrice] = useState('');
    const [msg, setMsg] = useState({ type: '', text: '' });
    const [assets, setAssets] = useState({});
    const [processing, setProcessing] = useState(false);

    useEffect(() => { loadWallet(); }, []);
    useEffect(() => { fetchData(); setInterval(fetchData, 5000); }, [wallet]);

    const loadWallet = () => {
        const storedKey = localStorage.getItem('volt_priv_key');
        const storedAddr = localStorage.getItem('volt_address');
        if (storedKey && storedAddr) {
            setWallet({ address: storedAddr, privateKey: storedKey });
        }
    };

    const fetchData = async () => {
        try {
            if (wallet) {
                const assetRes = await axios.post(API_URL, { command: "get_assets", address: wallet.address });
                if (assetRes.data.status === 'success') setAssets(assetRes.data.data.assets);
            }
            const ords = await axios.post(API_URL, { command: "get_orders" });
            if (ords.data.status === 'success') setOrders(ords.data.data.orders);
        } catch (e) { }
    };

    const handleTrade = async () => {
        if (!wallet) return setMsg({ type: 'error', text: "Connect Wallet First" });
        if (!amount || !price) return setMsg({ type: 'error', text: "Enter Amount & Price" });

        setProcessing(true);
        try {
            // Get Nonce
            const nonceRes = await axios.post(API_URL, { command: "get_balance", address: wallet.address });
            const nonce = nonceRes.data.data.nonce || 0;

            const satsAmount = Math.floor(parseFloat(amount) * 100000000);
            const satsPrice = Math.floor(parseFloat(price) * 100000000);

            const tx = {
                sender: wallet.address,
                receiver: side === 'BUY' ? "DEX_BUY" : "DEX_SELL",
                amount: satsAmount,
                token: pair,
                tx_type: "PlaceOrder",
                nonce: nonce + 1,
                fee: 100000,
                timestamp: Math.floor(Date.now() / 1000),
                price: satsPrice, // Included in hash now
                script_pub_key: { code: [], type: "P2PKH" },
                script_sig: { code: [], type: "P2PKH" }
            };

            const signature = signTransaction(tx, wallet.privateKey);
            tx.signature = signature;

            const res = await axios.post(API_URL, { command: "broadcast_transaction", data: tx });
            if (res.data.status === 'success') {
                setMsg({ type: 'success', text: 'Order Placed!' });
                setAmount(''); setPrice(''); fetchData();
            } else { setMsg({ type: 'error', text: res.data.message }); }
        } catch (e) { console.error(e); setMsg({ type: 'error', text: "Failed" }); }
        setProcessing(false);
    };

    const handleCancel = async (id) => {
        if (!window.confirm("Cancel this order?")) return;
        setProcessing(true);
        try {
            // Get Nonce
            const nonceRes = await axios.post(API_URL, { command: "get_balance", address: wallet.address });
            const nonce = nonceRes.data.data.nonce || 0;

            const tx = {
                sender: wallet.address,
                receiver: "DEX_CANCEL",
                amount: 0,
                token: id, // Order ID passed in token field
                tx_type: "CancelOrder",
                nonce: nonce + 1,
                fee: 10000,
                timestamp: Math.floor(Date.now() / 1000),
                price: 0,
                script_pub_key: { code: [], type: "P2PKH" },
                script_sig: { code: [], type: "P2PKH" }
            };

            const signature = signTransaction(tx, wallet.privateKey);
            tx.signature = signature;

            const res = await axios.post(API_URL, { command: "broadcast_transaction", data: tx });
            if (res.data.status === 'success') {
                setMsg({ type: 'success', text: 'Cancel Sent!' });
                fetchData();
            } else { setMsg({ type: 'error', text: res.data.message }); }

        } catch (e) { setMsg({ type: 'error', text: "Cancel Failed" }); }
        setProcessing(false);
    };

    const pairOrders = orders.filter(o => o.token === pair);
    const bids = pairOrders.filter(o => o.side === 'BUY').sort((a, b) => b.price - a.price)
        .map(o => ({ ...o, price: o.price / 100000000, amount: o.amount / 100000000 }));
    const asks = pairOrders.filter(o => o.side === 'SELL').sort((a, b) => a.price - b.price)
        .map(o => ({ ...o, price: o.price / 100000000, amount: o.amount / 100000000 }));

    // Calc Max Vol for Depth Bars
    const maxVol = Math.min(Math.max(...bids.map(o => o.amount), ...asks.map(o => o.amount), 1), 10000);

    if (!wallet) return <div className="loading" onClick={() => window.location.href = '/wallet'} style={{ cursor: 'pointer' }}>Connect Wallet First</div>;

    return (
        <div className="pro-container">
            <div className="pro-layout">
                {/* --- CENTER: CHART --- */}
                <div className="glass-panel pro-main-chart">
                    <div className="panel-header">
                        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                            <span style={{ color: '#fff', fontSize: '1.1rem' }}>{pair} / VLT</span>
                            <span style={{ background: '#1ea1f1', padding: '2px 6px', borderRadius: '4px', color: '#fff', fontSize: '0.7rem' }}>15m</span>
                        </div>
                        <select
                            value={pair}
                            onChange={e => setPair(e.target.value)}
                            style={{ background: 'rgba(0,0,0,0.3)', border: '1px solid #334155', color: '#fff', borderRadius: '4px', padding: '2px 8px' }}
                        >
                            <option value="GOLD">GOLD</option>
                            <option value="USDT">USDT</option>
                            {Object.keys(assets).filter(a => a !== 'GOLD' && a !== 'USDT').map(a => <option key={a} value={a}>{a}</option>)}
                        </select>
                    </div>
                    <div className="panel-content" style={{ padding: 0, overflow: 'hidden' }}>
                        <Chart pair={pair} />
                    </div>
                </div>

                {/* --- RIGHT: SIDE PANEL --- */}
                <div className="pro-side-panel">
                    {/* ORDER BOOK */}
                    <div className="glass-panel" style={{ flex: 1, minHeight: '300px' }}>
                        <div className="panel-header">ORDER BOOK</div>
                        <div className="panel-content" style={{ display: 'flex', flexDirection: 'column', gap: '0' }}>
                            <div style={{ display: 'flex', justifyContent: 'space-between', padding: '5px 8px', color: '#64748b', fontSize: '0.7rem' }}>
                                <span>PRICE</span><span>AMT</span>
                            </div>
                            <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column-reverse', justifyContent: 'flex-end' }}>
                                {asks.map(o => (
                                    <div key={o.id} className="order-row-pro">
                                        <div className="depth-bar depth-ask" style={{ width: `${(o.amount / maxVol) * 100}%` }}></div>
                                        <span style={{ color: '#ef4444', zIndex: 1 }}>{o.price.toFixed(4)}</span>
                                        <span style={{ color: '#cbd5e1', zIndex: 1 }}>{o.amount.toFixed(2)}</span>
                                    </div>
                                ))}
                            </div>
                            <div style={{ textAlign: 'center', padding: '8px', color: '#94a3b8', borderTop: '1px solid rgba(255,255,255,0.05)', borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                                {orders.length > 0 ? (orders[0].price / 100000000).toFixed(4) : '---'}
                            </div>
                            <div style={{ flex: 1, overflowY: 'auto' }}>
                                {bids.map(o => (
                                    <div key={o.id} className="order-row-pro">
                                        <div className="depth-bar depth-bid" style={{ width: `${(o.amount / maxVol) * 100}%` }}></div>
                                        <span style={{ color: '#10b981', zIndex: 1 }}>{o.price.toFixed(4)}</span>
                                        <span style={{ color: '#cbd5e1', zIndex: 1 }}>{o.amount.toFixed(2)}</span>
                                    </div>
                                ))}
                            </div>
                        </div>
                    </div>

                    {/* TRADE FORM */}
                    <div className="glass-panel">
                        <div className="panel-header">PLACE ORDER</div>
                        <div className="panel-content">
                            <div style={{ display: 'flex', background: 'rgba(0,0,0,0.3)', borderRadius: '6px', padding: '2px', marginBottom: '15px' }}>
                                <button onClick={() => setSide('BUY')} style={{ flex: 1, padding: '6px', borderRadius: '4px', border: 'none', background: side === 'BUY' ? '#10b981' : 'transparent', color: side === 'BUY' ? '#fff' : '#64748b', fontWeight: 'bold', cursor: 'pointer', transition: '0.2s' }}>BUY</button>
                                <button onClick={() => setSide('SELL')} style={{ flex: 1, padding: '6px', borderRadius: '4px', border: 'none', background: side === 'SELL' ? '#ef4444' : 'transparent', color: side === 'SELL' ? '#fff' : '#64748b', fontWeight: 'bold', cursor: 'pointer', transition: '0.2s' }}>SELL</button>
                            </div>
                            <div style={{ marginBottom: '10px' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', color: '#94a3b8', fontSize: '0.7rem', marginBottom: '4px' }}>
                                    <span>Avail:</span>
                                    <span>{side === 'BUY' ? `${(assets['VLT'] || 0).toLocaleString()} VLT` : `${(assets[pair] || 0).toLocaleString()} ${pair}`}</span>
                                </div>
                                <input type="number" placeholder="Price" value={price} onChange={e => setPrice(e.target.value)} style={{ width: '100%', background: 'rgba(0,0,0,0.2)', border: '1px solid #334155', borderRadius: '6px', padding: '8px', color: '#fff' }} />
                            </div>
                            <div style={{ marginBottom: '15px' }}>
                                <input type="number" placeholder="Amount" value={amount} onChange={e => setAmount(e.target.value)} style={{ width: '100%', background: 'rgba(0,0,0,0.2)', border: '1px solid #334155', borderRadius: '6px', padding: '8px', color: '#fff' }} />
                            </div>

                            <button onClick={handleTrade} disabled={processing} style={{ width: '100%', padding: '10px', background: side === 'BUY' ? '#10b981' : '#ef4444', border: 'none', borderRadius: '6px', color: '#fff', fontWeight: 'bold', cursor: 'pointer', opacity: processing ? 0.7 : 1 }}>
                                {processing ? '...' : `${side} ${pair}`}
                            </button>
                            {msg.text && <div style={{ marginTop: '10px', color: msg.type === 'error' ? '#ef4444' : '#10b981', fontSize: '0.8rem', textAlign: 'center' }}>{msg.text}</div>}
                        </div>
                    </div>
                </div>

                {/* --- BOTTOM: ACTIVE ORDERS --- */}
                <div className="glass-panel pro-orders-panel">
                    <div className="panel-header">
                        <span>OPEN ORDERS</span>
                        <span style={{ fontSize: '0.7rem', color: '#64748b' }}>History</span>
                    </div>
                    <div className="panel-content">
                        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.8rem', color: '#cbd5e1' }}>
                            <thead>
                                <tr style={{ textAlign: 'left', color: '#64748b' }}>
                                    <th style={{ padding: '8px', fontWeight: 600 }}>Time</th>
                                    <th style={{ padding: '8px', fontWeight: 600 }}>Pair</th>
                                    <th style={{ padding: '8px', fontWeight: 600 }}>Side</th>
                                    <th style={{ padding: '8px', fontWeight: 600 }}>Price</th>
                                    <th style={{ padding: '8px', fontWeight: 600 }}>Amount</th>
                                    <th style={{ padding: '8px', fontWeight: 600, textAlign: 'right' }}>Action</th>
                                </tr>
                            </thead>
                            <tbody>
                                {orders.filter(o => o.creator === wallet.address).length === 0 ? (
                                    <tr><td colSpan="6" style={{ textAlign: 'center', padding: '20px', color: '#64748b' }}>No open orders</td></tr>
                                ) : (
                                    orders.filter(o => o.creator === wallet.address).map(o => (
                                        <tr key={o.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                                            <td style={{ padding: '8px' }}>{new Date().toLocaleTimeString()}</td>
                                            <td style={{ padding: '8px' }}>{o.token}/VLT</td>
                                            <td style={{ padding: '8px', color: o.side === 'BUY' ? '#10b981' : '#ef4444' }}>{o.side}</td>
                                            <td style={{ padding: '8px' }}>{(o.price / 100000000).toFixed(4)}</td>
                                            <td style={{ padding: '8px' }}>{(o.amount / 100000000).toFixed(2)}</td>
                                            <td style={{ padding: '8px', textAlign: 'right' }}>
                                                <button onClick={() => handleCancel(o.id)} style={{ background: 'transparent', border: '1px solid #334155', color: '#ef4444', borderRadius: '4px', padding: '2px 8px', cursor: 'pointer' }}>Cancel</button>
                                            </td>
                                        </tr>
                                    ))
                                )}
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </div>
    );
}

export default Dex;
