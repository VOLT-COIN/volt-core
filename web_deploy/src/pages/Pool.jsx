import React, { useState, useEffect } from 'react';
import axios from 'axios';
import { signTransaction } from '../utils/wallet';

const API_URL = "http://localhost:6001/api";

function Pool() {
    const [pools, setPools] = useState([]);
    const [tokenA, setTokenA] = useState('VLT');
    const [tokenB, setTokenB] = useState('');
    const [amountA, setAmountA] = useState('');
    const [amountB, setAmountB] = useState('');
    const [wallet, setWallet] = useState(null);
    const [msg, setMsg] = useState({ type: '', text: '' });
    const [processing, setProcessing] = useState(false);

    useEffect(() => {
        loadWallet();
        fetchPools();
    }, []);

    const loadWallet = () => {
        const key = localStorage.getItem('volt_priv_key');
        const addr = localStorage.getItem('volt_address');
        if (key && addr) setWallet({ address: addr, privateKey: key });
    };

    const fetchPools = async () => {
        try {
            const res = await axios.post(API_URL, { command: "get_pools" });
            if (res.data.status === 'success') setPools(res.data.data.pools);
        } catch (e) { }
    };

    const handleAddLiquidity = async () => {
        if (!wallet) return setMsg({ type: 'error', text: "Connect Wallet First" });
        if (!amountA || !amountB || !tokenB) return setMsg({ type: 'error', text: "Invalid Inputs" });

        setProcessing(true);
        try {
            const nonceRes = await axios.post(API_URL, { command: "get_balance", address: wallet.address });
            const nonce = nonceRes.data.data.nonce || 0;

            const poolId = `${tokenA}/${tokenB}`;

            const tx = {
                sender: wallet.address,
                receiver: "AMM_SYSTEM",
                amount: Math.floor(parseFloat(amountA) * 100000000),
                token: poolId,
                tx_type: "AddLiquidity",
                nonce: nonce + 1,
                fee: 100000,
                timestamp: Math.floor(Date.now() / 1000),
                price: Math.floor(parseFloat(amountB) * 100000000), // Hack: Using price field for Amount B
                script_pub_key: { code: [], type: "P2PKH" },
                script_sig: { code: [], type: "P2PKH" }
            };

            const signature = signTransaction(tx, wallet.privateKey);
            tx.signature = signature;

            const res = await axios.post(API_URL, { command: "broadcast_transaction", data: tx });
            if (res.data.status === 'success') {
                setMsg({ type: 'success', text: 'Liquidity Added!' });
                fetchPools();
            } else {
                setMsg({ type: 'error', text: res.data.message });
            }
        } catch (e) {
            console.error(e);
            setMsg({ type: 'error', text: "Failed" });
        }
        setProcessing(false);
    };

    return (
        <div className="main-content">
            <div className="glass-card">
                <h1>💧 Liquidity Pools</h1>

                <div className="pool-list">
                    <h3>Active Pools</h3>
                    {pools.length === 0 ? <p>No Active Pools</p> : (
                        <table>
                            <thead><tr><th>Pair</th><th>Reserve A</th><th>Reserve B</th><th>Shares</th></tr></thead>
                            <tbody>
                                {pools.map((p, i) => (
                                    <tr key={i}>
                                        <td>{p.token_a}/{p.token_b}</td>
                                        <td>{(p.reserve_a / 1e8).toFixed(2)}</td>
                                        <td>{(p.reserve_b / 1e8).toFixed(2)}</td>
                                        <td>{p.total_shares}</td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    )}
                </div>

                <div className="add-pool-form">
                    <h3>➕ Add Liquidity</h3>
                    <div className="row">
                        <input value={tokenA} onChange={e => setTokenA(e.target.value)} placeholder="Token A (VLT)" />
                        <input type="number" value={amountA} onChange={e => setAmountA(e.target.value)} placeholder="Amount A" />
                    </div>
                    <div className="row">
                        <input value={tokenB} onChange={e => setTokenB(e.target.value)} placeholder="Token B (Symbol)" />
                        <input type="number" value={amountB} onChange={e => setAmountB(e.target.value)} placeholder="Amount B" />
                    </div>
                    <button className="btn-primary" onClick={handleAddLiquidity} disabled={processing}>
                        {processing ? "Adding..." : "Add Liquidity"}
                    </button>
                    {msg.text && <div className={`msg ${msg.type}`}>{msg.text}</div>}
                </div>
            </div>

            <style>{`
                .pool-list table { width: 100%; border-collapse: collapse; margin-bottom: 30px; }
                .pool-list th, .pool-list td { padding: 10px; border-bottom: 1px solid rgba(255,255,255,0.1); text-align: left; }
                .add-pool-form { background: rgba(0,0,0,0.2); pading: 20px; border-radius: 10px; }
                .row { display: flex; gap: 10px; margin-bottom: 10px; }
                .row input { flex: 1; padding: 10px; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); color: white; border-radius: 5px; }
            `}</style>
        </div>
    );
}

export default Pool;
