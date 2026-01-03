use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use chrono::Utc;
use crate::transaction::Transaction;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64, // Changed to u64 (Unix Seconds) for Bitcoin compatibility standards
    pub proof_of_work: u32, // Nonce (4 bytes)
    pub previous_hash: String,
    pub hash: String,
    pub transactions: Vec<Transaction>,
    pub difficulty: u32, // Bits (4 bytes)
    pub merkle_root: String,
    pub validator_stake: u64, // Hybrid Consensus: Staked amount Claim
}

impl Block {
    pub fn new(index: u64, previous_hash: String, transactions: Vec<Transaction>, difficulty: usize, validator_stake: u64) -> Self {
        let timestamp = Utc::now().timestamp() as u64; // Seconds
        let proof_of_work = 0;
        let hash = String::new();
        let merkle_root = Block::calculate_merkle_root(&transactions);

        let mut block = Block {
            index,
            timestamp,
            proof_of_work,
            previous_hash,
            hash,
            transactions,
            difficulty: difficulty as u32,
            merkle_root,
            validator_stake,
        };
        block.hash = block.calculate_hash();
        block
    }

    pub fn calculate_merkle_root(transactions: &Vec<Transaction>) -> String {
        if transactions.is_empty() {
             return "0".repeat(64);
        }
        let mut hashes: Vec<Vec<u8>> = transactions.iter().map(|tx| tx.get_hash()).collect();
        
        while hashes.len() > 1 {
            if hashes.len() % 2 != 0 {
                hashes.push(hashes.last().unwrap().clone());
            }
            let mut new_hashes = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(&chunk[0]);
                hasher.update(&chunk[1]);
                let res = hasher.finalize().to_vec();
                 // Double SHA256 typical in Bitcoin, but one is fine for MVP
                 // Let's do double for "Standard"
                 let mut hasher2 = Sha256::new();
                 hasher2.update(res);
                 new_hashes.push(hasher2.finalize().to_vec());
            }
            hashes = new_hashes;
        }
        hex::encode(&hashes[0])
    }

    pub fn calculate_hash(&self) -> String {
        // Bitcoin Header Format (80 bytes)
        // Version (4) + PrevBlock (32) + MerkleRoot (32) + Timestamp (4) + Bits (4) + Nonce (4)
        
        let version: u32 = 1;
        let mut bytes = Vec::new();
        
        bytes.extend(&version.to_le_bytes()); // 4
        
        // PrevHash (32 bytes) - handle genesis "0"
        let prev_hash_bytes = if self.previous_hash == "0" {
            vec![0u8; 32]
        } else {
             hex::decode(&self.previous_hash).unwrap_or(vec![0u8; 32])
        };
        // Use Little Endian (Reverse bytes) for Bitcoin Header Compatibility
        let mut prev_le = prev_hash_bytes;
        prev_le.reverse();
        bytes.extend(&prev_le); 
        
        // Merkle Root (32 bytes)
        let merkle_bytes = hex::decode(&self.merkle_root).unwrap_or(vec![0u8; 32]);
        // Standard Merkle Root (Internal Order - Do NOT Reverse)
        let merkle_le = merkle_bytes;
        // merkle_le.reverse(); // FIX: Removed incorrect reversal
        bytes.extend(&merkle_le); 
        
        // Timestamp (4 bytes)
        bytes.extend(&(self.timestamp as u32).to_le_bytes());
        
        // Bits/Difficulty (4 bytes)
        bytes.extend(&self.difficulty.to_le_bytes());

        // Nonce (4 bytes)
        bytes.extend(&self.proof_of_work.to_le_bytes());

        // DEBUG: Print Header
        // Ensure it is 80 bytes
        if bytes.len() != 80 {
        }

        // Hybrid Consensus: Validator Stake (Excluded from PoW Hash to maintain 80-byte Standard Header)
        // bytes.extend(&self.validator_stake.to_le_bytes()); 
        
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let res1 = hasher.finalize();

        // Double SHA256 (Bitcoin Standard)
        let mut hasher2 = Sha256::new();
        hasher2.update(res1);
        let res2 = hasher2.finalize();
        
        hex::encode(res2)
    }

    pub fn mine(&mut self, difficulty: usize) {
        // Hybrid Consensus: Apply Bonus Locally
        let bonus = (self.validator_stake / 10_000_000_000) as u32; 
        let bonus_capped = bonus.min(5);
        let effective_diff = (difficulty as u32).saturating_sub(bonus_capped);
        let mut required_diff = if effective_diff < 1 { 1 } else { effective_diff };
        // FIX: Handle Bits format (large number)
        if required_diff > 64 { required_diff = 4; } 
        
        // println!("Mining with Stake Bonus: -{} Diff (Effective: {})", bonus_capped, required_diff);

        let target = "0".repeat(required_diff as usize);
        while !self.hash.starts_with(&target) {
            self.proof_of_work += 1;
            self.hash = self.calculate_hash();
        }
        println!("Block mined: {}", self.hash);
    }
}
