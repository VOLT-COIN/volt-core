import CryptoJS from 'crypto-js';

export function calculateTxHash(tx) {
    // Replicate Rust's calculate_hash:
    // payload = "{}:{}:{}:{}:{}:{}:{}:{:?}"
    // sender:receiver:amount:nonce:token:timestamp:price:tx_type

    // Ensure defaults if missing (matching Rust struct defaults)
    const sender = tx.sender;
    const receiver = tx.receiver;
    const amount = tx.amount;
    const nonce = tx.nonce || 0;
    const token = tx.token || "VLT";
    const timestamp = tx.timestamp;
    const price = tx.price || 0;

    // Rust Enum Debug format: passing "Transfer", "IssueToken" etc.
    // If tx_type is undefined, default to "Transfer"
    const tx_type = tx.tx_type || "Transfer";

    const payload = `${sender}:${receiver}:${amount}:${nonce}:${token}:${timestamp}:${price}:${tx_type}`;

    return CryptoJS.SHA256(payload).toString(CryptoJS.enc.Hex);
}
