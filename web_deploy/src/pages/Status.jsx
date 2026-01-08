
import { useState, useEffect } from 'react';
import axios from 'axios';
import { getApiConfig } from '../utils/apiConfig';

function Status() {
    const [metrics, setMetrics] = useState({
        api_status: 'Checking...',
        height: 0,
        difficulty: 0,
        peers: 0,
        peers: 0,
        mempool: 0,
        pool_hashrate: 0,
        last_hash: '...'
    });

    useEffect(() => {
        checkHealth();
        const interval = setInterval(checkHealth, 5000); // 5s update
        return () => clearInterval(interval);
    }, []);

    const checkHealth = async () => {
        try {
            const start = Date.now();
            const res = await axios.post('/api/rpc', { command: 'get_chain_info' }, getApiConfig());
            const latency = Date.now() - start;

            if (res.data.status === 'success') {
                const data = res.data.data;
                setMetrics({
                    api_status: `Operational (${latency}ms)`,
                    height: data.height || 0,
                    difficulty: data.difficulty || 0,
                    peers: data.peers || 0,
                    mempool: data.pending_count || 0,
                    pool_hashrate: data.pool_hashrate || 0,
                    last_hash: data.last_hash || 'None'
                });
            } else {
                setMetrics(prev => ({ ...prev, api_status: 'Degraded' }));
            }
        } catch (e) {
            setMetrics(prev => ({ ...prev, api_status: 'Offline' }));
        }
    };

    const formatHashrate = (hs) => {
        if (!hs) return '0 H/s';
        if (hs >= 1e9) return (hs / 1e9).toFixed(2) + ' GH/s';
        if (hs >= 1e6) return (hs / 1e6).toFixed(2) + ' MH/s';
        if (hs >= 1e3) return (hs / 1e3).toFixed(2) + ' KH/s';
        return hs.toFixed(0) + ' H/s';
    };

    const StatusCard = ({ label, value, subtext, icon, color = '#10b981' }) => (
        <div className="glass-card" style={{ padding: '25px', display: 'flex', alignItems: 'center', gap: '20px' }}>
            <div style={{
                width: '50px', height: '50px', borderRadius: '12px',
                background: `${color}20`, display: 'flex', alignItems: 'center', justifyContent: 'center',
                fontSize: '1.5rem', color: color, boxShadow: `0 0 15px ${color}40`
            }}>
                {icon}
            </div>
            <div>
                <div style={{ color: '#888', fontSize: '0.9rem', marginBottom: '5px' }}>{label}</div>
                <div style={{ fontSize: '1.4rem', fontWeight: 'bold' }}>{value}</div>
                {subtext && <div style={{ fontSize: '0.8rem', color: '#555' }}>{subtext}</div>}
            </div>
        </div>
    );

    return (
        <div className="container" style={{ paddingTop: '100px', paddingBottom: '50px', maxWidth: '1000px' }}>
            <div style={{ textAlign: 'center', marginBottom: '50px' }}>
                <img src="/logo.png?v=2" alt="Volt Logo" style={{ width: '80px', height: '80px', marginBottom: '20px' }} />
                <h1 className="gradient-text" style={{ fontSize: '2.5rem', marginBottom: '10px' }}>System Status</h1>
                <p style={{ color: '#aaa', maxWidth: '600px', margin: '0 auto' }}>
                    Real-time performance metrics of the Volt Network.
                </p>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: '20px', marginBottom: '40px' }}>
                <StatusCard
                    label="API Gateway"
                    value={metrics.api_status.split('(')[0]}
                    subtext={metrics.api_status.includes('(') ? metrics.api_status.split('(')[1].replace(')', '') + ' latency' : ''}
                    icon="⚡"
                    color={metrics.api_status.includes('Operational') ? '#10b981' : '#ef4444'}
                />
                <StatusCard
                    label="Block Height"
                    value={metrics.height.toLocaleString()}
                    subtext={`Latest Hash: ${metrics.last_hash.substring(0, 8)}...`}
                    icon="📦"
                    color="#3b82f6"
                />
                <StatusCard
                    label="Network Peers"
                    value={metrics.peers}
                    subtext="Kademlia DHT Active"
                    icon="🌐"
                    color="#8b5cf6"
                />
                <StatusCard
                    label="Mining Difficulty"
                    value={metrics.difficulty}
                    subtext="Argon2d PoW"
                    icon="⛏️"
                    color="#f59e0b"
                />
                <StatusCard
                    label="Mempool"
                    value={metrics.mempool}
                    subtext="Pending Transactions"
                    icon="⏳"
                    color="#ec4899"
                />
                <StatusCard
                    label="Pool Hashrate"
                    value={formatHashrate(metrics.pool_hashrate)}
                    subtext="Real-time (60s avg)"
                    icon="🚀"
                    color="#f43f5e"
                />
                <StatusCard
                    label="EVM Engine"
                    value="Active"
                    subtext="RevM 3.1 Compatible"
                    icon="⚙️"
                    color="#6366f1"
                />
            </div>

            <div className="glass-card" style={{ padding: '30px' }}>
                <h3 style={{ borderBottom: '1px solid var(--glass-border)', paddingBottom: '15px', marginBottom: '20px' }}>Core Services</h3>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '15px' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <span>Smart Contracts (Wasm/EVM)</span>
                        <span style={{ color: '#10b981', background: '#10b98110', padding: '2px 10px', borderRadius: '4px' }}>Operational</span>
                    </div>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <span>P2P Discovery (UDP)</span>
                        <span style={{ color: '#10b981', background: '#10b98110', padding: '2px 10px', borderRadius: '4px' }}>Operational</span>
                    </div>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <span>Web Interface</span>
                        <span style={{ color: '#10b981', background: '#10b98110', padding: '2px 10px', borderRadius: '4px' }}>Operational</span>
                    </div>
                </div>
            </div>

            <div style={{ marginTop: '50px', textAlign: 'center', color: '#444', fontSize: '0.8rem' }}>
                Volt Core v1.0.23 • Automatic Refresh (5s)
            </div>
        </div>
    );
}

export default Status;
