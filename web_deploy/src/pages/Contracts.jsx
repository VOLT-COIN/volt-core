
import React, { useState } from 'react';
import { Container, Alert, Tabs, Tab } from 'react-bootstrap';

const Contracts = () => {
    const [key, setKey] = useState('deploy');
    const [status, setStatus] = useState({ msg: '', type: '' });

    // Deploy State
    const [bytecode, setBytecode] = useState('');
    const [gasLimit, setGasLimit] = useState(1000000);

    // Interact State
    const [contractAddr, setContractAddr] = useState('');
    const [method, setMethod] = useState('');
    const [args, setArgs] = useState(''); // Not used yet in MVP

    // View State
    const [viewAddr, setViewAddr] = useState('');
    const [contractData, setContractData] = useState(null);

    const apiCall = async (command, params) => {
        try {
            const res = await fetch('http://localhost:1337', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ command, ...params })
            });
            const json = await res.json();
            return json;
        } catch (e) {
            return { status: 'error', message: e.toString() };
        }
    };

    const handleDeploy = async () => {
        setStatus({ msg: 'Deploying...', type: 'info' });
        const res = await apiCall('deploy_contract', { bytecode, gas_limit: parseInt(gasLimit) });
        if (res.status === 'success') {
            setStatus({ msg: `Deployment Tx Sent! ${res.message}`, type: 'success' });
        } else {
            setStatus({ msg: `Error: ${res.message}`, type: 'danger' });
        }
    };

    const handleCall = async () => {
        setStatus({ msg: 'Calling...', type: 'info' });
        const res = await apiCall('call_contract', {
            to: contractAddr,
            method,
            gas_limit: parseInt(gasLimit)
        });
        if (res.status === 'success') {
            setStatus({ msg: `Call Tx Sent! ${res.message}`, type: 'success' });
        } else {
            setStatus({ msg: `Error: ${res.message}`, type: 'danger' });
        }
    };

    const handleView = async () => {
        setContractData(null);
        const res = await apiCall('get_contract', { address: viewAddr });
        if (res.status === 'success') {
            setContractData(res.data);
            setStatus({ msg: '', type: '' });
        } else {
            setStatus({ msg: `Error: ${res.message}`, type: 'danger' });
        }
    };

    // Custom Input Style matching App.css
    const inputStyle = {
        background: 'rgba(255, 255, 255, 0.05)',
        border: '1px solid rgba(255, 255, 255, 0.1)',
        color: '#fff',
        padding: '15px 25px',
        borderRadius: '50px',
        width: '100%',
        marginBottom: '20px',
        outline: 'none'
    };

    const labelStyle = {
        display: 'block',
        marginBottom: '8px',
        color: '#94a3b8',
        fontSize: '0.9rem',
        fontWeight: '600'
    };

    return (
        <Container className="mt-5 mb-5">
            <div style={{ textAlign: 'center', marginBottom: '40px' }}>
                <h2 className="gradient-text" style={{ fontSize: '3rem', fontWeight: '800' }}>SMART CONTRACTS</h2>
                <p style={{ color: '#94a3b8' }}>Deploy, Interact with, and Audit Wasm applications on Volt.</p>
            </div>

            {status.msg && (
                <div className={`glass-card mb-4 ${status.type === 'success' ? 'border-success' : status.type === 'danger' ? 'border-danger' : 'border-primary'}`} style={{ padding: '15px', color: '#fff', textAlign: 'center' }}>
                    {status.type === 'info' && <span className="spinner-border spinner-border-sm me-2" role="status" aria-hidden="true"></span>}
                    {status.type === 'success' ? '✅ ' : status.type === 'danger' ? '❌ ' : ''} {status.msg}
                </div>
            )}

            <div className="glass-card" style={{ maxWidth: '800px', margin: '0 auto' }}>
                <Tabs
                    activeKey={key}
                    onSelect={(k) => setKey(k)}
                    className="mb-4 custom-tabs"
                    variant="pills"
                    style={{ borderBottom: 'none', justifyContent: 'center', gap: '10px' }}
                >
                    <Tab eventKey="deploy" title="Deploy" tabClassName="text-light">
                        <div style={{ padding: '20px' }}>
                            <div style={{ marginBottom: '20px' }}>
                                <label style={labelStyle}>Wasm Bytecode (Hex)</label>
                                <textarea
                                    rows={6}
                                    value={bytecode}
                                    onChange={(e) => setBytecode(e.target.value)}
                                    placeholder="61736d0100..."
                                    style={{ ...inputStyle, borderRadius: '15px', fontFamily: 'monospace' }}
                                />
                            </div>
                            <div style={{ marginBottom: '30px' }}>
                                <label style={labelStyle}>Gas Limit <span style={{ fontSize: '0.7em', color: '#38bdf8' }}>(Est)</span></label>
                                <input
                                    type="number"
                                    value={gasLimit}
                                    onChange={(e) => setGasLimit(e.target.value)}
                                    style={inputStyle}
                                />
                            </div>
                            <button className="btn btn-primary w-100" onClick={handleDeploy}>
                                🚀 DEPLOY CONTRACT
                            </button>
                        </div>
                    </Tab>

                    <Tab eventKey="interact" title="Interact" tabClassName="text-light">
                        <div style={{ padding: '20px' }}>
                            <div>
                                <label style={labelStyle}>Contract Address</label>
                                <input
                                    type="text"
                                    value={contractAddr}
                                    onChange={(e) => setContractAddr(e.target.value)}
                                    placeholder="V..."
                                    style={inputStyle}
                                />
                            </div>
                            <div>
                                <label style={labelStyle}>Method Name</label>
                                <input
                                    type="text"
                                    value={method}
                                    onChange={(e) => setMethod(e.target.value)}
                                    placeholder="e.g. transfer, mint..."
                                    style={inputStyle}
                                />
                            </div>
                            <button className="btn btn-primary w-100" style={{ background: '#f472b6', border: 'none' }} onClick={handleCall}>
                                ⚡ EXECUTE
                            </button>
                        </div>
                    </Tab>

                    <Tab eventKey="view" title="Read State" tabClassName="text-light">
                        <div style={{ padding: '20px' }}>
                            <div style={{ display: 'flex', gap: '10px' }}>
                                <input
                                    placeholder="Contract Address"
                                    value={viewAddr}
                                    onChange={(e) => setViewAddr(e.target.value)}
                                    style={{ ...inputStyle, marginBottom: 0 }}
                                />
                                <button className="btn btn-secondary" onClick={handleView} style={{ borderRadius: '50px', whiteSpace: 'nowrap' }}>
                                    🔍 FIND
                                </button>
                            </div>

                            {contractData && (
                                <div className="mt-4 p-3" style={{ background: 'rgba(0,0,0,0.3)', borderRadius: '12px', border: '1px solid rgba(255,255,255,0.1)' }}>
                                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '10px' }}>
                                        <span style={{ color: '#94a3b8' }}>Owner:</span>
                                        <span style={{ fontFamily: 'monospace', color: '#f472b6' }}>{contractData.owner}</span>
                                    </div>
                                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '20px' }}>
                                        <span style={{ color: '#94a3b8' }}>Size:</span>
                                        <span style={{ color: '#fff' }}>{contractData.bytecode_size} bytes</span>
                                    </div>

                                    <h5 style={{ color: '#38bdf8', marginBottom: '10px' }}>Storage</h5>
                                    {Object.keys(contractData.storage).length === 0 ? (
                                        <div style={{ color: '#666', fontStyle: 'italic' }}>Empty Storage</div>
                                    ) : (
                                        <div style={{ display: 'grid', gap: '10px' }}>
                                            {Object.entries(contractData.storage).map(([k, v]) => (
                                                <div key={k} style={{ display: 'flex', justifyContent: 'space-between', padding: '10px', background: 'rgba(255,255,255,0.05)', borderRadius: '8px' }}>
                                                    <span style={{ color: '#ccc' }}>{k}</span>
                                                    <span style={{ fontFamily: 'monospace', color: '#fbbf24' }}>{(v || "").substr(0, 16)}...</span>
                                                </div>
                                            ))}
                                        </div>
                                    )}
                                </div>
                            )}
                        </div>
                    </Tab>

                </Tabs>
            </div>
        </Container>
    );
};

export default Contracts;
