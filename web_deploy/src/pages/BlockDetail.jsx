import { useParams, useNavigate } from 'react-router-dom';
import { useState, useEffect } from 'react';
import axios from 'axios';
import { API_URL } from '../config';
import { getApiConfig } from '../utils/apiConfig';

function BlockDetail() {
    const { id } = useParams();
    const navigate = useNavigate();
    const [block, setBlock] = useState(null);
    const [error, setError] = useState(null);

    useEffect(() => {
        const fetchBlock = async () => {
            try {
                const res = await axios.post(API_URL, { command: "get_block", height: parseInt(id) }, getApiConfig());
                if (res.data.status === 'success') {
                    const b = res.data.data;
                    setBlock({
                        height: b.index,
                        hash: b.hash,
                        prevHash: b.previous_hash,
                        timestamp: b.timestamp,
                        time: new Date(b.timestamp * 1000).toLocaleString(),
                        merkleRoot: b.merkle_root,
                        nonce: b.proof_of_work,
                        difficulty: b.difficulty,
                        txs: b.transactions
                    });
                } else {
                    setError("Block not found");
                }
            } catch (e) { setError(e.message); }
        };
        fetchBlock();
    }, [id]);

    if (!block && !error) return <div className="container" style={{ padding: '50px', textAlign: 'center' }}>Loading Block...</div>;
    if (error) return <div className="container" style={{ textAlign: 'center', padding: '50px' }}><h2 className="gradient-text">{error}</h2><button className="btn btn-primary" onClick={() => navigate('/explorer')}>Back to Explorer</button></div>;

    return (
        <div className="container" style={{ padding: '40px 20px', maxWidth: '1000px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '30px' }}>
                <h1 className="gradient-text">Block #{block.height}</h1>
                <div style={{ display: 'flex', gap: '10px' }}>
                    <button className="btn btn-secondary" style={{ padding: '8px 15px' }} onClick={() => navigate('/block/' + (block.height - 1))}>Previous</button>
                    <button className="btn btn-secondary" style={{ padding: '8px 15px' }} onClick={() => navigate('/block/' + (block.height + 1))}>Next</button>
                </div>
            </div>

            <div className="glass-card" style={{ padding: '30px', marginBottom: '40px' }}>
                <div style={{ display: 'grid', gridTemplateColumns: 'minmax(150px, 200px) 1fr', gap: '15px', fontSize: '0.9rem' }}>
                    <div style={{ color: '#888' }}>Hash</div>
                    <div style={{ fontFamily: 'monospace', wordBreak: 'break-all', color: '#fff' }}>{block.hash}</div>

                    <div style={{ color: '#888' }}>Timestamp</div>
                    <div>{block.time} ({block.timestamp})</div>

                    <div style={{ color: '#888' }}>Difficulty</div>
                    <div>{block.difficulty.toLocaleString()}</div>

                    <div style={{ color: '#888' }}>Nonce</div>
                    <div style={{ fontFamily: 'monospace' }}>{block.nonce}</div>

                    <div style={{ color: '#888' }}>Merkle Root</div>
                    <div style={{ fontFamily: 'monospace', wordBreak: 'break-all', color: '#888', fontSize: '0.8rem' }}>{block.merkleRoot}</div>

                    <div style={{ color: '#888' }}>Previous Hash</div>
                    <div style={{ fontFamily: 'monospace', color: '#38bdf8', cursor: 'pointer', wordBreak: 'break-all' }} onClick={() => navigate('/block/' + (block.height - 1))}>
                        {block.prevHash}
                    </div>
                </div>
            </div>

            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
                <h3 style={{ margin: 0 }}>Transactions ({block.txs.length})</h3>
                <span style={{ fontSize: '0.8rem', color: '#888' }}>Included in this block</span>
            </div>

            <div className="glass-card" style={{ padding: '0', overflow: 'hidden' }}>
                {block.txs.map((tx, i) => (
                    <div key={i} className="hover-row" style={{ padding: '20px', borderBottom: '1px solid rgba(255,255,255,0.05)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <div>
                            <div style={{ color: '#f472b6', fontFamily: 'monospace', cursor: 'pointer', marginBottom: '5px' }} onClick={() => navigate('/tx/' + tx.hash)}>
                                {(tx.hash || "").substr(0, 32)}...
                            </div>
                            <div style={{ display: 'flex', gap: '10px', fontSize: '0.75rem', color: '#666' }}>
                                <span>{tx.tx_type}</span>
                                <span>{(tx.sender || "").substr(0, 12)}...</span>
                            </div>
                        </div>
                        <div style={{ textAlign: 'right' }}>
                            <div style={{ fontWeight: 'bold', color: '#fff' }}>{(tx.amount / 100000000).toLocaleString()} VLT</div>
                            {tx.tx_type === 'Coinbase' && <span style={{ fontSize: '0.6rem', background: '#38bdf8', color: '#000', padding: '2px 6px', borderRadius: '4px', fontWeight: 'bold' }}>BLOCK REWARD</span>}
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
}

export default BlockDetail;
