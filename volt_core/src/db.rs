#![allow(dead_code)]
use sled::Db;
use crate::block::Block;
use crate::transaction::Transaction;

use serde_json;

pub struct Database {
    db: Db,
}

impl Database {
    pub fn new(path: &str) -> sled::Result<Self> {
        let db = sled::open(path)?;
        Ok(Database { db })
    }

    // Load entire chain for memory initialization
    pub fn load_chain(&self) -> Option<Vec<Block>> {
        if let Ok(mut blocks) = self.get_all_blocks() {
            if blocks.is_empty() { return None; }
            blocks.sort_by_key(|b| b.index); // Ensure order
            Some(blocks)
        } else {
            None
        }
    }

    // Helper: Block Tree
    fn blocks(&self) -> sled::Result<sled::Tree> {
        self.db.open_tree("blocks")
    }

    // Helper: Transaction Tree
    fn txs(&self) -> sled::Result<sled::Tree> {
        self.db.open_tree("transactions")
    }
    
    // Helper: Address Index (addr -> [tx_id_list])
    fn addr_index(&self) -> sled::Result<sled::Tree> {
        self.db.open_tree("addr_index")
    }

    // Helper: Miner Ledger (for PPS/Pool balances)
    pub fn miner_ledger(&self) -> sled::Result<sled::Tree> {
        self.db.open_tree("miner_ledger")
    }

    pub fn save_block(&self, block: &Block) -> sled::Result<()> {
        let blocks = self.blocks()?;
        let txs = self.txs()?;
        let addr_idx = self.addr_index()?;

        // 1. Save Block (Key: Height (BigEndian))
        let key = block.index.to_be_bytes(); // Sortable key
        let val = serde_json::to_vec(block).unwrap();
        blocks.insert(key, val)?;

        // 2. Save Transactions & Index Address
        for t in &block.transactions {
             let tx_id = t.get_hash(); // Vec<u8>
             let tx_id_hex = hex::encode(&tx_id); // String
             let tx_val = serde_json::to_vec(t).unwrap();
             
             // Save using binary key
             txs.insert(&tx_id, tx_val)?;
             
             // Update Indices using Hex String
             self.append_tx_to_index(&addr_idx, &t.sender, &tx_id_hex)?;
             
             // Update Indices for Receiver
             self.append_tx_to_index(&addr_idx, &t.receiver, &tx_id_hex)?;
        }
        
        // Flush to disk
        self.db.flush()?;
        Ok(())
    }

    pub fn save_chain(&self, chain: &Vec<Block>) -> sled::Result<()> {
        // Clear existing blocks? (Optional, but safer for replacement)
        // self.blocks()?.clear()?; 
        // Note: clearing might be slow. Overwriting is usually fine if index logic holds.
        // But if chain shrinks, we might have orphan blocks.
        // For MVP, we just overwrite.
        
        for block in chain {
            self.save_block(block)?;
        }
        self.db.flush()?;
        Ok(())
    }
    
    fn append_tx_to_index(&self, tree: &sled::Tree, addr: &str, tx_id: &str) -> sled::Result<()> {
        // We actally usually store list of TXIDs.
        // For simplicity: Key = "addr:txid", Val = ""
        // This allows range scan on "addr:"
        let key = format!("{}:{}", addr, tx_id);
        tree.insert(key.as_bytes(), &[])?;
        Ok(())
    }

    pub fn get_last_block(&self) -> sled::Result<Option<Block>> {
        let blocks = self.blocks()?;
        if let Some((_k, v)) = blocks.last()? {
            let block: Block = serde_json::from_slice(&v).unwrap();
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }
    
    pub fn get_all_blocks(&self) -> sled::Result<Vec<Block>> {
        let blocks = self.blocks()?;
        let mut result = Vec::new();
        for item in blocks.iter() {
            if let Ok((_k, v)) = item {
                let block: Block = serde_json::from_slice(&v).unwrap();
                result.push(block);
            }
        }
        Ok(result)
    }

    // Helper: Pending Transactions Tree
    fn pending_txs_tree(&self) -> sled::Result<sled::Tree> {
        self.db.open_tree("pending_txs")
    }

    pub fn save_pending_txs(&self, txs: &Vec<Transaction>) -> sled::Result<()> {
        let tree = self.pending_txs_tree()?;
        tree.clear()?; 
        
        for (i, tx) in txs.iter().enumerate() {
            let key = i.to_be_bytes(); 
            let val = serde_json::to_vec(tx).unwrap();
            tree.insert(key, val)?;
        }
        self.db.flush()?;
        Ok(())
    }

    pub fn load_pending_txs(&self) -> sled::Result<Vec<Transaction>> {
        let tree = self.pending_txs_tree()?;
        let mut txs = Vec::new();
        // Since we used index as key, robust iteration is sorted if we scan, but iter() is usually sorted by key in sled.
        // Let's verify sort. Sled iter is over keys. Keys are BigEndian Block Index? No here it is index i.
        // usize to_be_bytes sorts correctly.
        for item in tree.iter() {
            if let Ok((_, val)) = item {
                if let Ok(tx) = serde_json::from_slice::<Transaction>(&val) {
                    txs.push(tx);
                }
            }
        }
        Ok(txs)
    }

    pub fn get_balance(&self, address: &str, token: &str) -> u64 {
        // Sled doesn't have SQL SUM. We must iterate history.
        // Optimization: In a real DB we'd store "Balances" tree.
        // For now, let's reconstruct from history (fast enough for single user).
        
        let mut balance: i64 = 0; // Use signed to detect errors, but supply is u64
        // Check "addr_index" prefix scan
        if let Ok(tree) = self.addr_index() {
             let prefix = format!("{}:", address);
             let iter = tree.scan_prefix(prefix.as_bytes());
             
             for item in iter {
                 if let Ok((key, _)) = item {
                     // Key is "addr:txid"
                     if let Ok(key_str) = str::from_utf8(&key) {
                         let parts: Vec<&str> = key_str.split(':').collect();
                         if parts.len() == 2 {
                             let txid = parts[1];
                             if let Ok(Some(tx_vec)) = self.txs().and_then(|t| t.get(txid)) {
                                 if let Ok(tx) = serde_json::from_slice::<Transaction>(&tx_vec) {
                                     let tx_token = tx.token.clone();
                                     if tx_token == token {
                                         if tx.receiver == address {
                                             balance += tx.amount as i64;
                                         }
                                         if tx.sender == address {
                                             balance -= tx.amount as i64;
                                         }
                                     }
                                 }
                             }
                         }
                     }
                 }
             }
        }
        
        if balance < 0 { 0 } else { balance as u64 }
    }
    
    pub fn get_history(&self, address: &str) -> Vec<Transaction> {
        let mut history = Vec::new();
        if let Ok(tree) = self.addr_index() {
             let prefix = format!("{}:", address);
             // Scan gives generic order (lexicographic on TXID usually).
             // We want Timestamp desc. 
             // We'll collect all, then sort.
             let iter = tree.scan_prefix(prefix.as_bytes());
             for item in iter {
                 if let Ok((key, _)) = item {
                     if let Ok(key_str) = std::str::from_utf8(&key) {
                         let parts: Vec<&str> = key_str.split(':').collect();
                         if parts.len() == 2 {
                             let txid = parts[1];
                             if let Ok(Some(tx_vec)) = self.txs().and_then(|t| t.get(txid)) {
                                 if let Ok(tx) = serde_json::from_slice::<Transaction>(&tx_vec) {
                                     history.push(tx);
                                 }
                             }
                         }
                     }
                 }
             }
        }
        // Sort by timestamp desc
        history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        history
    }

    // --- PPS Ledger Methods ---
    pub fn credit_miner(&self, miner: &str, amount: u64) -> sled::Result<u64> {
        let ledger = self.miner_ledger()?;
        let old_bal = self.get_miner_balance(miner);
        let new_bal = old_bal + amount;
        
        ledger.insert(miner.as_bytes(), &new_bal.to_le_bytes())?;
        Ok(new_bal)
    }

    pub fn debit_miner(&self, miner: &str, amount: u64) -> sled::Result<u64> {
        let ledger = self.miner_ledger()?;
        let old_bal = self.get_miner_balance(miner);
        if old_bal < amount { return Ok(old_bal); } // Should check before call
        
        let new_bal = old_bal - amount;
        ledger.insert(miner.as_bytes(), &new_bal.to_le_bytes())?;
        Ok(new_bal)
    }

    pub fn get_miner_balance(&self, miner: &str) -> u64 {
        if let Ok(ledger) = self.miner_ledger() {
            if let Ok(Some(bytes)) = ledger.get(miner) {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&bytes);
                return u64::from_le_bytes(arr);
            }
        }
        0
    }
    
    pub fn get_all_miner_balances(&self) -> Vec<(String, u64)> {
        let mut res = Vec::new();
        if let Ok(ledger) = self.miner_ledger() {
             for item in ledger.iter() {
                 if let Ok((k, v)) = item {
                     if let Ok(miner) = std::str::from_utf8(&k) {
                         let mut arr = [0u8; 8];
                         arr.copy_from_slice(&v);
                         res.push((miner.to_string(), u64::from_le_bytes(arr)));
                     }
                 }
             }
        }
        res
    }
}
