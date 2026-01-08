import React, { useState, useEffect } from 'react';
import axios from 'axios';
import { signTransaction } from '../utils/wallet';
import Chart from '../components/Chart';

const API_URL = "http://localhost:6001/api";

function Swap() {
    const [pools, setPools] = useState([]);
    const [tokenIn, setTokenIn] = useState('VLT');
    const [tokenOut, setTokenOut] = useState('');
    const [amountIn, setAmountIn] = useState('');
    const [estimatedOut, setEstimatedOut] = useState(0);
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
            if (res.data.status === 'success') {
                setPools(res.data.data.pools);
            }
        } catch (e) {
            console.error("Fetch pools error", e);
        }
    };

    const calculateOutput = (amt) => {
        if (!amt || !tokenOut) return 0;
        const pool = pools.find(p =>
            (p.token_a === tokenIn && p.token_b === tokenOut) ||
            (p.token_a === tokenOut && p.token_b === tokenIn)
        );

        if (!pool) return 0;

        const isAtoB = pool.token_a === tokenIn;
        const rIn = isAtoB ? pool.reserve_a : pool.reserve_b;
        const rOut = isAtoB ? pool.reserve_b : pool.reserve_a;

        if (rIn === 0 || rOut === 0) return 0;

        const amtInSats = Math.floor(parseFloat(amt) * 100000000); // Atomic Units if applicable, but usually simple tokens
        // Check if tokens are standard 8 decimals. VLT is. Assuming others are too for now.

        const inputWithFee = amtInSats * 997;
        const numerator = inputWithFee * rOut;
        const denominator = (rIn * 1000) + inputWithFee;
        const output = numerator / denominator;

        return output / 100000000;
    };

    const handleAmountChange = (e) => {
        setAmountIn(e.target.value);
        setEstimatedOut(calculateOutput(e.target.value));
    };

    const handleSwap = async () => {
        if (!wallet) return setMsg({ type: 'error', text: "Connect Wallet First" });
        if (!amountIn || !tokenOut) return setMsg({ type: 'error', text: "Invalid Parameters" });

        setProcessing(true);
        try {
            const pool = pools.find(p =>
                (p.token_a === tokenIn && p.token_b === tokenOut) ||
                (p.token_a === tokenOut && p.token_b === tokenIn)
            );

            if (!pool) throw new Error("Pool not found");

            const isAtoB = pool.token_a === tokenIn;
            // Pool ID is traditionally "TokenA/TokenB" key in map
            const poolId = `${pool.token_a}/${pool.token_b}`; // We need exact key from map. 
            // NOTE: The backend stores keys as provided in AddLiquidity. Ideally we pass the pool ID or token pair.
            // Let's assume unique pair and we can reconstruction key if we force alphabetic order or just search.
            // Actually, `get_pools` returns values. We need the KEY to reference it in tx.token?
            // Wait, backend `ApplyTransaction` for Swap uses `transaction.token` as `pool_id`.
            // So we need to ensure we send the correct map key.
            // For now, let's assume we can derive it or `get_pools` should return it.
            // The current `get_pools` returns a LIST of `Pool` structs. It doesn't give the Map Key (Pool ID).
            // Fix: We need to guess the ID. Usually "TokenA/TokenB".

            // Backend `AddLiquidity` does `transaction.token.split('/')`. So ID is `A/B`.
            // So we just reconstruct it from the found pool struct.

            const nonceRes = await axios.post(API_URL, { command: "get_balance", address: wallet.address });
            const nonce = nonceRes.data.data.nonce || 0;

            const tx = {
                sender: wallet.address,
                receiver: isAtoB ? "SWAP_A_TO_B" : "SWAP_B_TO_A",
                amount: Math.floor(parseFloat(amountIn) * 100000000),
                token: `${pool.token_a}/${pool.token_b}`, // Pool ID
                tx_type: "Swap",
                nonce: nonce + 1,
                fee: 100000,
                timestamp: Math.floor(Date.now() / 1000),
                price: Math.floor(estimatedOut * 0.95 * 100000000), // Min Output (Slippage 5%)
                script_pub_key: { code: [], type: "P2PKH" },
                script_sig: { code: [], type: "P2PKH" }
            };

            const signature = signTransaction(tx, wallet.privateKey);
            tx.signature = signature;

            const res = await axios.post(API_URL, { command: "broadcast_transaction", data: tx });
            if (res.data.status === 'success') {
                setMsg({ type: 'success', text: 'Swap Broadcasted!' });
                setAmountIn(''); fetchPools();
            } else {
                setMsg({ type: 'error', text: res.data.message });
            }

        } catch (e) {
            console.error(e);
            setMsg({ type: 'error', text: "Swap Failed" });
        }
        setProcessing(false);
    };

    return (
        <div className="main-content">
            <div className="glass-card">
                <h1>🦄 Swap</h1>
                <div className="swap-box">
                    <div className="input-group">
                        <label>From</label>
                        <select value={tokenIn} onChange={e => setTokenIn(e.target.value)}>
                            <option value="VLT">VLT</option>
                            {pools.map(p => (
                                <React.Fragment key={p.token_a + p.token_b}>
                                    <option value={p.token_a}>{p.token_a}</option>
                                    <option value={p.token_b}>{p.token_b}</option>
                                </React.Fragment>
                            ))}
                        </select>
                        <input type="number" placeholder="0.0" value={amountIn} onChange={handleAmountChange} />
                    </div>

                    <div className="arrow">⬇️</div>

                    <div className="input-group">
                        <label>To</label>
                        <select value={tokenOut} onChange={e => { setTokenOut(e.target.value); calculateOutput(amountIn); }}>
                            <option value="">Select Token</option>
                            {[...new Set(pools.flatMap(p => [p.token_a, p.token_b]))].filter(t => t !== tokenIn).map(t => (
                                <option key={t} value={t}>{t}</option>
                            ))}
                        </select>
                        <input type="text" value={estimatedOut.toFixed(6)} disabled />
                    </div>

                    <button className="btn-primary" onClick={handleSwap} disabled={processing}>
                        {processing ? "Swapping..." : "Swap Now"}
                    </button>

                    {msg.text && <div className={`msg ${msg.type}`}>{msg.text}</div>}
                </div>
            </div>

            {tokenOut && <div className="glass-card" style={{ marginTop: '20px' }}>
                <Chart pair={`${tokenIn}/${tokenOut}`} /> {/* Approximation */}
            </div>}

            <style>{`
                .swap-box { max-width: 400px; margin: 0 auto; display: flex; flex-direction: column; gap: 20px; }
                .input-group { display: flex; flex-direction: column; gap: 10px; background: rgba(255,255,255,0.05); padding: 15px; border-radius: 12px; }
                .input-group select, .input-group input { background: transparent; border: none; color: white; font-size: 1.2rem; width: 100%; outline: none; }
                .arrow { text-align: center; font-size: 1.5rem; }
            `}</style>
        </div>
    );
}

export default Swap;
