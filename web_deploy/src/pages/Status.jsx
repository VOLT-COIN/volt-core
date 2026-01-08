
import { useState, useEffect } from 'react';
import axios from 'axios';
import { getApiConfig } from '../utils/apiConfig';

function Status() {
    const [status, setStatus] = useState({
        api: 'Checking...',
        node: 'Checking...',
        pool: 'Checking...',
        website: 'Operational'
    });

    useEffect(() => {
        checkHealth();
        const interval = setInterval(checkHealth, 10000);
        return () => clearInterval(interval);
    }, []);

    const checkHealth = async () => {
        try {
            const start = Date.now();
            const res = await axios.post('/api/rpc', { command: 'get_chain_info' }, getApiConfig());
            const latency = Date.now() - start;

            if (res.data.status === 'success') {
                setStatus({
                    api: `Operational (${latency}ms)`,
                    node: `Synced (Height: ${res.data.data.blocks})`,
                    pool: 'Operational', // Assuming pool is up if node is up
                    website: 'Operational'
                });
            } else {
                setStatus(prev => ({ ...prev, api: 'Degraded', node: 'Connecting...' }));
            }
        } catch (e) {
            setStatus({
                api: 'Offline',
                node: 'Unreachable',
                pool: 'Unknown',
                website: 'Operational'
            });
        }
    };

    const StatusItem = ({ label, value }) => (
        <div style={{ display: 'flex', justifyContent: 'space-between', padding: '20px', borderBottom: '1px solid var(--glass-border)', alignItems: 'center' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <div style={{
                    width: '10px', height: '10px', borderRadius: '50%',
                    background: value.includes('Operational') || value.includes('Synced') ? '#10b981' : '#ef4444',
                    boxShadow: value.includes('Operational') || value.includes('Synced') ? '0 0 10px #10b981' : '0 0 10px #ef4444'
                }}></div>
                <span style={{ fontSize: '1.1rem' }}>{label}</span>
            </div>
            <span style={{ color: '#ccc', fontFamily: 'monospace' }}>{value}</span>
        </div>
    );

    return (
        <div className="container" style={{ paddingTop: '100px', maxWidth: '800px' }}>
            <div style={{ textAlign: 'center', marginBottom: '40px' }}>
                <h1 className="gradient-text">Network Status</h1>
                <p style={{ color: '#aaa' }}>Real-time uptime monitoring for Volt services.</p>
            </div>

            <div className="glass-card" style={{ padding: 0 }}>
                <StatusItem label="RPC API" value={status.api} />
                <StatusItem label="Blockchain Node" value={status.node} />
                <StatusItem label="Mining Pool" value={status.pool} />
                <StatusItem label="Web Interface" value={status.website} />
            </div>

            <div style={{ marginTop: '30px', textAlign: 'center', color: '#666', fontSize: '0.9rem' }}>
                <p>Last Updated: {new Date().toLocaleTimeString()}</p>
            </div>
        </div>
    );
}

export default Status;
