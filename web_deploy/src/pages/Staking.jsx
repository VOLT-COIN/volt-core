import { useState, useEffect } from 'react';
import axios from 'axios';

const API_URL = '/api/rpc';

function Staking() {
    const [amount, setAmount] = useState('');
    const [staked, setStaked] = useState(0);
    const [wallet, setWallet] = useState(null);

    useEffect(() => { loadWallet(); }, []);

    const loadWallet = () => {
        const storedKey = localStorage.getItem('volt_priv_key');
        const storedAddr = localStorage.getItem('volt_address');
        if (storedKey && storedAddr) {
            setWallet({ address: storedAddr, privateKey: storedKey });
            fetchStaking(storedAddr);
        }
    };

    const fetchStaking = async (address) => {
        try {
            // Fetch real balance/staking info
            const balRes = await axios.post(API_URL, { command: "get_balance", address });
            if (balRes.data.status === 'success') {
                // Assuming get_balance returns 'staked' field as seen in api.rs
                setStaked(balRes.data.data.staked / 100000000);
            }
        } catch (e) { }
    };

    const handleStake = async () => {
        alert("Staking coming soon to Mainnet!");
    };

    return (
        <div className="container" style={{ padding: '40px 20px', maxWidth: '800px', margin: '0 auto' }}>
            <div style={{ textAlign: 'center', marginBottom: '50px' }}>
                <h1 style={{ fontSize: '3.5rem', marginBottom: '10px' }}>
                    <span className="gradient-text">EARN YIELD</span> <span style={{ textShadow: '0 0 20px rgba(74, 222, 128, 0.5)' }}>📈</span>
                </h1>
                <p style={{ color: '#888', fontSize: '1.2rem' }}>Secure the network and earn rewards.</p>
            </div>

            <div className="glass-card" style={{ padding: '40px', marginBottom: '30px' }}>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '20px', marginBottom: '30px' }}>
                    <div style={{ textAlign: 'center', padding: '20px', background: 'rgba(0,0,0,0.3)', borderRadius: '12px' }}>
                        <p style={{ color: '#888', marginBottom: '5px' }}>APY</p>
                        <h2 style={{ color: '#00f2ea', fontSize: '2.5rem', margin: 0 }}>12.5%</h2>
                    </div>
                    <div style={{ textAlign: 'center', padding: '20px', background: 'rgba(0,0,0,0.3)', borderRadius: '12px' }}>
                        <p style={{ color: '#888', marginBottom: '5px' }}>TOTAL STAKED</p>
                        <h2 style={{ color: '#ff0055', fontSize: '2.5rem', margin: 0 }}>4.2M</h2>
                    </div>
                </div>

                <div style={{ marginBottom: '20px' }}>
                    <label style={{ color: '#888', marginLeft: '10px' }}>Amount to Stake (VLT)</label>
                    <input
                        type="number"
                        placeholder="0.00"
                        value={amount}
                        onChange={e => setAmount(e.target.value)}
                        style={{ fontSize: '1.5rem', fontWeight: 'bold' }}
                    />
                </div>

                <button className="btn btn-primary" onClick={handleStake} style={{ width: '100%', fontSize: '1.2rem', padding: '15px' }}>
                    STAKE NOW
                </button>
            </div>

            <div className="glass-card" style={{ textAlign: 'center' }}>
                <h3 style={{ marginBottom: '10px' }}>My Staking Position</h3>
                <h1 style={{ fontSize: '3rem', margin: 0 }}>
                    {staked.toLocaleString('en-US', { minimumFractionDigits: 2 })} <span style={{ fontSize: '1.5rem', color: '#888' }}>VLT</span>
                </h1>
            </div>
        </div>
    );
}

export default Staking;
