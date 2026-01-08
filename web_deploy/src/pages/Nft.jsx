import React, { useState, useEffect } from 'react';
import axios from 'axios';
import { signTransaction } from '../utils/wallet';

const API_URL = "http://localhost:6001/api";

function Nft() {
    const [wallet, setWallet] = useState(null);
    const [nfts, setNfts] = useState([]);
    const [msg, setMsg] = useState({ type: '', text: '' });
    const [processing, setProcessing] = useState(false);

    // Forms
    const [mintId, setMintId] = useState('');
    const [mintUri, setMintUri] = useState('');
    const [transferId, setTransferId] = useState('');
    const [transferTo, setTransferTo] = useState('');

    useEffect(() => { loadWallet(); }, []);
    useEffect(() => { if (wallet) fetchNfts(); }, [wallet]);

    const loadWallet = () => {
        const storedKey = localStorage.getItem('volt_priv_key');
        const storedAddr = localStorage.getItem('volt_address');
        if (storedKey && storedAddr) {
            setWallet({ address: storedAddr, privateKey: storedKey });
        }
    };

    const fetchNfts = async () => {
        try {
            const res = await axios.post(API_URL, { command: "get_nfts", address: wallet.address });
            if (res.data.status === 'success') {
                setNfts(res.data.data.nfts);
            }
        } catch (e) { console.error(e); }
    };

    const handleMint = async () => {
        if (!mintId || !mintUri) return setMsg({ type: 'error', text: 'Enter ID and URI' });
        setProcessing(true);
        try {
            const nonceRes = await axios.post(API_URL, { command: "get_balance", address: wallet.address });
            const nonce = nonceRes.data.data.nonce || 0;

            const tx = {
                sender: wallet.address,
                receiver: mintUri, // URI stored in receiver field
                amount: 0,
                token: mintId,     // NFT ID stored in token field
                tx_type: "IssueNFT",
                nonce: nonce + 1,
                fee: 100000,
                timestamp: Math.floor(Date.now() / 1000),
                price: 0,
                script_pub_key: { code: [], type: "P2PKH" },
                script_sig: { code: [], type: "P2PKH" }
            };

            tx.signature = signTransaction(tx, wallet.privateKey);

            const res = await axios.post(API_URL, { command: "broadcast_transaction", data: tx });
            if (res.data.status === 'success') {
                setMsg({ type: 'success', text: 'NFT Minted!' });
                setMintId(''); setMintUri('');
                setTimeout(fetchNfts, 2000);
            } else {
                setMsg({ type: 'error', text: res.data.message });
            }
        } catch (e) { setMsg({ type: 'error', text: "Mint Failed" }); }
        setProcessing(false);
    };

    const handleTransfer = async () => {
        if (!transferId || !transferTo) return setMsg({ type: 'error', text: 'Enter ID and Receiver' });
        setProcessing(true);
        try {
            const nonceRes = await axios.post(API_URL, { command: "get_balance", address: wallet.address });
            const nonce = nonceRes.data.data.nonce || 0;

            const tx = {
                sender: wallet.address,
                receiver: transferTo,
                amount: 0,
                token: transferId,
                tx_type: "TransferNFT",
                nonce: nonce + 1,
                fee: 100000,
                timestamp: Math.floor(Date.now() / 1000),
                price: 0,
                script_pub_key: { code: [], type: "P2PKH" },
                script_sig: { code: [], type: "P2PKH" }
            };

            tx.signature = signTransaction(tx, wallet.privateKey);

            const res = await axios.post(API_URL, { command: "broadcast_transaction", data: tx });
            if (res.data.status === 'success') {
                setMsg({ type: 'success', text: 'NFT Transferred!' });
                setTransferId(''); setTransferTo('');
                setTimeout(fetchNfts, 2000);
            } else {
                setMsg({ type: 'error', text: res.data.message });
            }
        } catch (e) { setMsg({ type: 'error', text: "Transfer Failed" }); }
        setProcessing(false);
    };

    if (!wallet) return <div className="loading">Connect Wallet</div>;

    return (
        <div className="main-content">
            <h1 className="gradient-text">NFT Gallery</h1>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '30px', marginBottom: '40px' }}>
                {/* MINT */}
                <div className="glass-card">
                    <h3>Mint NFT</h3>
                    <div className="input-group">
                        <label>Unique ID</label>
                        <input value={mintId} onChange={e => setMintId(e.target.value)} placeholder="e.g. GEM_001" />
                    </div>
                    <div className="input-group">
                        <label>Image URI</label>
                        <input value={mintUri} onChange={e => setMintUri(e.target.value)} placeholder="https://..." />
                    </div>
                    <button className="btn" onClick={handleMint} disabled={processing}>Mint NFT</button>
                </div>

                {/* TRANSFER */}
                <div className="glass-card">
                    <h3>Transfer NFT</h3>
                    <div className="input-group">
                        <label>NFT ID</label>
                        <input value={transferId} onChange={e => setTransferId(e.target.value)} placeholder="e.g. GEM_001" />
                    </div>
                    <div className="input-group">
                        <label>Receiver Address</label>
                        <input value={transferTo} onChange={e => setTransferTo(e.target.value)} placeholder="02..." />
                    </div>
                    <button className="btn" onClick={handleTransfer} disabled={processing} style={{ background: '#ff0055' }}>Transfer</button>
                </div>
            </div>

            {msg.text && <div className={`msg ${msg.type}`}>{msg.text}</div>}

            <div className="nft-grid" style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: '20px' }}>
                {nfts.map(nft => (
                    <div key={nft.id} className="glass-card" style={{ padding: '10px' }}>
                        <div style={{ width: '100%', height: '200px', background: '#000', borderRadius: '8px', overflow: 'hidden', marginBottom: '10px' }}>
                            <img src={nft.uri} alt={nft.id} style={{ width: '100%', height: '100%', objectFit: 'cover' }} onError={(e) => e.target.src = 'https://via.placeholder.com/200?text=No+Image'} />
                        </div>
                        <h4>{nft.id}</h4>
                        <p style={{ fontSize: '0.7rem', color: '#888', wordBreak: 'break-all' }}>Owner: {nft.owner.substr(0, 10)}...</p>
                    </div>
                ))}
            </div>
        </div>
    );
}

export default Nft;
