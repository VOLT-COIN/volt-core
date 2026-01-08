import { ec as EC } from 'elliptic';
import CryptoJS from 'crypto-js';
import { Wallet, Mnemonic } from 'ethers';

const ec = new EC('secp256k1');

// Generate Rust-Compatible Wallet
export const createWallet = () => {
    const wallet = Wallet.createRandom();
    return restoreWallet(wallet.mnemonic.phrase);
};

// Restore from Mnemonic (Rust Logic: SHA256(Seed) -> PrivKey)
export const restoreWallet = (phrase) => {
    try {
        const mnemonic = Mnemonic.fromPhrase(phrase);
        const seed = mnemonic.computeSeed(); // Returns 0x... hex

        // Remove 0x if present
        const cleanSeed = seed.startsWith('0x') ? seed.slice(2) : seed;

        // Hash Seed directly (Rust Wallet Logic)
        const seedWords = CryptoJS.enc.Hex.parse(cleanSeed);
        const hash = CryptoJS.SHA256(seedWords);
        const privateKey = hash.toString(CryptoJS.enc.Hex);

        // Derive Address: Rust uses Compressed Public Key (Hex)
        const keyPair = ec.keyFromPrivate(privateKey);
        const address = keyPair.getPublic(true, 'hex');

        return {
            address: address, // Matches Rust "Address" (PubKey)
            privateKey: privateKey,
            mnemonic: phrase
        };
    } catch (e) {
        console.error("Restore Error:", e);
        return null;
    }
};

// Legacy single key gen (optional, but createWallet is better)
export const generateWallet = () => {
    return createWallet();
};

export const keysFromMnemonic = (mnemonic) => {
    // Redirect to standard restore
    return restoreWallet(mnemonic) || { address: '', privateKey: '' };
};

export const signTransaction = (tx, privateKey) => {
    // Deterministic Hashing: sender:receiver:amount:nonce:token:timestamp:price:tx_type
    const payload = `${tx.sender}:${tx.receiver}:${tx.amount}:${tx.nonce}:${tx.token}:${tx.timestamp}:${tx.price}:${tx.tx_type}`;
    const hash = CryptoJS.SHA256(payload).toString();

    // Use Elliptic for strict DER signature generation (Backend Compatibility)
    const key = ec.keyFromPrivate(privateKey);
    const signature = key.sign(hash);
    return signature.toDER('hex');
};
