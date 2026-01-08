import { useParams, useNavigate } from 'react-router-dom';
import { useState, useEffect } from 'react';
import axios from 'axios';
import { API_URL } from '../config';
import { getApiConfig } from '../utils/apiConfig';

function TxDetail() {
    const { id } = useParams();
    const navigate = useNavigate();
    const [tx, setTx] = useState(null);
    const [error, setError] = useState(null);

    useEffect(() => {
        const init = async () => {
            try {
                const txRes = await axios.post(API_URL, { command: "get_transaction", hash: id }, getApiConfig());
                if (txRes.data.status === 'success') {
                    const realTx = txRes.data.data;
                    setTx({
                        hash: realTx.hash,
                        type: realTx.tx_type,
                        block: realTx.block_height || 'Mempool',
                        time: realTx.timestamp ? new Date(realTx.timestamp * 1000).toLocaleString() : 'Recent',
                        from: realTx.sender,
                        to: realTx.receiver || 'N/A (Contract/Issue)',
                        amount: realTx.amount / 100000000,
                        fee: realTx.fee / 100000000,
                        nonce: realTx.nonce,
                        data: realTx.data,
                        confirmations: realTx.confirmations || 0
                    });
                } else {
                    setError(txRes.data.message || "Transaction Not Found");
                }
            } catch (e) { setError(e.message); }
        };
        init();
    }, [id]);

    if (!tx && !error) return <div className="container" style={{ padding: '50px', textAlign: 'center' }}>Loading Transaction...</div>;
    if (error) return <div className="container" style={{ textAlign: 'center', padding: '50px' }}><h2 className="gradient-text">{error}</h2><button className="btn btn-primary" onClick={() => navigate('/explorer')}>Back to Explorer</button></div>;

    const isSmartContract = tx.type === 'DeployContract' || tx.type === 'CallContract';

    return (
        <div className="container" style={{ padding: '40px 20px', maxWidth: '1000px' }}>
            <h1 className="gradient-text" style={{ marginBottom: '10px' }}>Transaction Details</h1>
            <p style={{ color: '#888', marginBottom: '30px', fontFamily: 'monospace' }}>{tx.hash}</p>

            <div className="glass-card" style={{ padding: '30px' }}>
                <div style={{ display: 'grid', gap: '20px' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <span style={{ color: '#888' }}>TYPE</span>
                        <span style={{ background: 'rgba(244, 114, 182, 0.2)', color: '#f472b6', padding: '4px 12px', borderRadius: '12px', fontSize: '0.8rem', fontWeight: 'bold' }}>{tx.type.toUpperCase()}</span>
                    </div>

                    <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                        <span style={{ color: '#888' }}>STATUS</span>
                        <span style={{ color: tx.block === 'Mempool' ? '#fbbf24' : '#4ade80', fontWeight: 'bold' }}>
                            {tx.block === 'Mempool' ? 'Pending' : 'Confirmed'} ({tx.confirmations} Confirms)
                        </span>
                    </div>

                    <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                        <span style={{ color: '#888' }}>BLOCK</span>
                        <span style={{ color: '#38bdf8', cursor: 'pointer' }} onClick={() => tx.block !== 'Mempool' && navigate('/block/' + tx.block)}>
                            {tx.block === 'Mempool' ? 'N/A' : '#' + tx.block}
                        </span>
                    </div>

                    <div style={{ borderTop: '1px solid rgba(255,255,255,0.1)', paddingTop: '20px', marginTop: '10px' }}>
                        <div style={{ background: 'rgba(0,0,0,0.2)', padding: '20px', borderRadius: '12px', display: 'flex', flexDirection: 'column', gap: '15px' }}>
                            <div>
                                <div style={{ color: '#555', fontSize: '0.75rem', marginBottom: '5px' }}>SENDER / FROM</div>
                                <div style={{ fontFamily: 'monospace', color: '#fff', fontSize: '0.9rem', wordBreak: 'break-all' }}>{tx.from}</div>
                            </div>
                            <div style={{ textAlign: 'center', color: '#444' }}>⬇</div>
                            <div>
                                <div style={{ color: '#555', fontSize: '0.75rem', marginBottom: '5px' }}>RECEIVER / TO</div>
                                <div style={{ fontFamily: 'monospace', color: '#fff', fontSize: '0.9rem', wordBreak: 'break-all' }}>{tx.to}</div>
                            </div>
                        </div>
                    </div>

                    <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '1.4rem', fontWeight: 'bold', margin: '10px 0' }}>
                        <span style={{ color: '#888', fontSize: '1rem' }}>VALUE</span>
                        <span>{tx.amount.toLocaleString()} VLT</span>
                    </div>

                    <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.9rem' }}>
                        <span style={{ color: '#888' }}>NETWORK FEE</span>
                        <span>{tx.fee} VLT</span>
                    </div>

                    {isSmartContract && tx.data && (
                        <div style={{ marginTop: '20px' }}>
                            <div style={{ color: '#888', fontSize: '0.8rem', marginBottom: '10px' }}>CONTRACT DATA (HEX)</div>
                            <div style={{ background: 'rgba(0,0,0,0.3)', padding: '15px', borderRadius: '8px', fontFamily: 'monospace', fontSize: '0.8rem', color: '#f472b6', wordBreak: 'break-all' }}>
                                {tx.data}
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}

export default TxDetail;
