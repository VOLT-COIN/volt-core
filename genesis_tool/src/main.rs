
use sha2::{Sha256, Digest};

fn main() {
    let target_hash = "6f22e8ff0d766afb8b685c50677bf7fc2d98f8769236e769414a060f916c9bae";
    println!("Target: {}", target_hash);

    // 1. Calculate Transaction Hash (SYSTEM)
    // From transaction.rs: get_hash for SYSTEM
    // bytes.extend(self.sender.as_bytes());
    // bytes.extend(self.receiver.as_bytes());
    // bytes.extend(&self.amount.to_le_bytes());
    // bytes.extend(&self.timestamp.to_le_bytes());

    let sender = "SYSTEM";
    let receiver = "GENESIS";
    let amount: u64 = 0;
    let tx_timestamp: u64 = 1767225600; // 2026-01-01

    let mut tx_bytes = Vec::new();
    tx_bytes.extend(sender.as_bytes());
    tx_bytes.extend(receiver.as_bytes());
    tx_bytes.extend(&amount.to_le_bytes());
    tx_bytes.extend(&tx_timestamp.to_le_bytes());

    let mut hasher = Sha256::new();
    hasher.update(&tx_bytes);
    let res1 = hasher.finalize();
    let mut hasher2 = Sha256::new();
    hasher2.update(res1);
    let tx_hash = hasher2.finalize().to_vec();
    
    // Merkle Root (Single Tx)
    // From block.rs: calculate_merkle_root
    // In block.rs, it doubles SHA256 of the tx hash again?
    // "let mut hasher = Sha256::new(); hasher.update(&chunk[0]); ... let res = hasher.finalize().to_vec();"
    // "let mut hasher2 = Sha256::new(); hasher2.update(res); new_hashes.push(hasher2.finalize().to_vec());"
    
    // Wait, block.rs takes hashes of transactions. Transaction::get_hash returns double-sha256.
    // So we have H(Tx).
    // calculate_merkle_root takes [H(Tx), H(Tx)] (if 1 tx, it duplicates).
    // Then it concatenates and double-sha256s matches.
    
    // Correct Merkle Logic for Single Tx (No hashing)
    let merkle_root = tx_hash.clone();
    println!("Merkle Root: {}", hex::encode(&merkle_root));

    // Block Header
    // Version (4) + PrevHash (32) + Merkle (32) + Time (4) + Bits (4) + Nonce (4)
    
    let version: u32 = 1;
    let prev_hash = vec![0u8; 32];
    let difficulty: u32 = 0x1d00ffff;
    let nonce: u32 = 0;
    
    // Brute Force Range: 2025-01-01 to 2027-01-01
    // 1735689600 to 1798761600
    // Optimizing: Check around target 2026-01-01 (1767225600) +/- 1 month
    let start = 1764547200; // Dec 2025
    let end = 1769827200;   // Feb 2026
    
    // Verify Local Match first
    let local_ts = 1767077203;
    let local_hash = calculate_block_hash(version, &prev_hash, &merkle_root, local_ts, difficulty, nonce);
    println!("Local TS {} -> Hash {}", local_ts, local_hash);
    
    let expected_local = "84f5f416582057fb80d83b6de4f671df7df7dab537e44ef7863f435c17aa9fb5";
    if local_hash == expected_local {
        println!("Algorithm CORRECT: Matches local genesis.");
    } else {
        println!("Algorithm INCORRECT: Does NOT match local genesis.");
        println!("Expected: {}", expected_local);
        println!("Got:      {}", local_hash);
    }

    println!("Brute forcing checks from {} to {} ...", start, end);
    
    for ts in start..end {
        let h = calculate_block_hash(version, &prev_hash, &merkle_root, ts, difficulty, nonce);
        if h == target_hash {
             println!("!!! MATCH FOUND !!!");
             println!("Timestamp: {}", ts);
             println!("Hash: {}", h);
             return;
        }
        if (ts - start) % 100000 == 0 {
             println!("Checked {}...", ts);
        }
    }
    println!("No match found in range.");
}

fn calculate_block_hash(version: u32, prev: &[u8], merkle: &[u8], timestamp: u64, difficulty: u32, nonce: u32) -> String {
    let mut bytes = Vec::new();
    bytes.extend(&version.to_le_bytes());
    
    let mut prev_le = prev.to_vec();
    prev_le.reverse();
    bytes.extend(&prev_le);
    
    // Merkle is NOT reversed in block.rs currently (comment says "FIX: Removed incorrect reversal")
    bytes.extend(merkle);
    
    bytes.extend(&(timestamp as u32).to_le_bytes());
    bytes.extend(&difficulty.to_le_bytes());
    bytes.extend(&nonce.to_le_bytes());
    
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let res1 = hasher.finalize();
    let mut hasher2 = Sha256::new();
    hasher2.update(res1);
    hex::encode(hasher2.finalize())
}
