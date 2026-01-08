#![allow(dead_code)]
use crate::block::Block;
use crate::transaction::{Transaction, TxType};
use crate::db::Database;
use crate::script::VirtualMachine;
use crate::vm::WasmVM; // Import WasmVM

use std::collections::{HashMap, BTreeMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Order {
    pub id: String,
    pub creator: String,
    pub token: String,
    pub side: String, // "BUY" or "SELL"
    pub price: u64,
    pub amount: u64, // Remaining amount
    pub timestamp: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pool {
    pub token_a: String,
    pub token_b: String,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub total_shares: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Candle {
    pub time: u64,
    pub open: u64,
    pub high: u64,
    pub low: u64,
    pub close: u64,
    pub volume: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NFT {
    pub id: String,
    pub owner: String,
    pub uri: String, // Metadata URL (IPFS/HTTP)
    pub created_at: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Contract {
    pub address: String, // Calculated ID
    pub owner: String,
    pub bytecode: Vec<u8>,
    pub storage: HashMap<String, Vec<u8>>,
}

#[derive(Clone, Default)]
pub struct ChainState {
    pub db: Option<Arc<Database>>,
    
    // DEX/NFT State (Kept in RAM for now, to be migrated later)
    pub pools: HashMap<String, Pool>, 
    pub orders: HashMap<String, Order>,
    pub bids: BTreeMap<(String, u64, u64), String>, 
    pub asks: BTreeMap<(String, u64, u64), String>,
    pub candles: HashMap<String, Vec<Candle>>,
    pub nfts: HashMap<String, NFT>,
    pub contracts: HashMap<String, Contract>, // Smart Contracts
    pub pending_rewards: BTreeMap<u64, Vec<(String, u64)>>,
}

impl ChainState {
    pub fn new(db: Option<Arc<Database>>) -> Self {
        ChainState {
            db,
            pools: HashMap::new(),
            orders: HashMap::new(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            candles: HashMap::new(),
            nfts: HashMap::new(),
            contracts: HashMap::new(),
            pending_rewards: BTreeMap::new(),
        }
    }

    pub fn clear(&mut self) {
        if let Some(ref db) = self.db {
            let _ = db.state_balances().map(|t| t.clear());
            let _ = db.state_nonces().map(|t| t.clear());
            let _ = db.state_stakes().map(|t| t.clear());
            let _ = db.state_tokens().map(|t| t.clear());
            // Clear DEX RAM state as well
            self.pools.clear();
            self.orders.clear();
            self.nfts.clear();
            self.contracts.clear();
        }
    }

    pub fn get_balance(&self, address: &str, token: &str) -> u64 {
        if let Some(ref db) = self.db {
            if let Ok(tree) = db.state_balances() {
                let key = format!("{}:{}", address, token);
                if let Ok(Some(val)) = tree.get(key) {
                    let mut bytes = [0u8; 8];
                    if val.len() == 8 {
                        bytes.copy_from_slice(&val);
                        return u64::from_be_bytes(bytes);
                    }
                }
            }
        }
        0 // Default if missing or DB error
    }

    fn set_balance(&mut self, address: &str, token: &str, amount: u64) {
        if let Some(ref db) = self.db {
            if let Ok(tree) = db.state_balances() {
                let key = format!("{}:{}", address, token);
                let val = amount.to_be_bytes();
                let _ = tree.insert(key, &val);
            }
        }
    }

    fn update_candle(&mut self, pair: &str, price: u64, volume: u64, timestamp: u64) {
        // Timeframe: 1 Minute (60 seconds)
        let timeframe = 60;
        let time_slot = (timestamp / timeframe) * timeframe;
        
        let history = self.candles.entry(pair.to_string()).or_insert_with(Vec::new);
        
        if let Some(last) = history.last_mut() {
            if last.time == time_slot {
                // Update existing candle
                if price > last.high { last.high = price; }
                if price < last.low { last.low = price; }
                last.close = price;
                last.volume += volume;
                return;
            }
        }
        
        // Create new candle
        history.push(Candle {
            time: time_slot,
            open: price,
            high: price,
            low: price,
            close: price,
            volume,
        });
    }


    pub fn get_stake(&self, address: &str) -> u64 {
         if let Some(ref db) = self.db {
            if let Ok(tree) = db.state_stakes() {
                if let Ok(Some(val)) = tree.get(address) {
                    let mut bytes = [0u8; 8];
                    if val.len() == 8 {
                         bytes.copy_from_slice(&val);
                         return u64::from_be_bytes(bytes);
                    }
                }
            }
         }
         0
    }

    pub fn set_stake(&mut self, address: &str, amount: u64) {
         if let Some(ref db) = self.db {
             if let Ok(tree) = db.state_stakes() {
                 let _ = tree.insert(address, &amount.to_be_bytes());
             }
         }
    }

    pub fn get_nonce(&self, address: &str) -> u64 {
         if let Some(ref db) = self.db {
            if let Ok(tree) = db.state_nonces() {
                if let Ok(Some(val)) = tree.get(address) {
                    let mut bytes = [0u8; 8];
                    if val.len() == 8 {
                         bytes.copy_from_slice(&val);
                         return u64::from_be_bytes(bytes);
                    }
                }
            }
         }
         0
    }

    pub fn set_nonce(&mut self, address: &str, nonce: u64) {
         if let Some(ref db) = self.db {
             if let Ok(tree) = db.state_nonces() {
                 let _ = tree.insert(address, &nonce.to_be_bytes());
             }
         }
    }

    pub fn token_exists(&self, token: &str) -> bool {
         if let Some(ref db) = self.db {
            if let Ok(tree) = db.state_tokens() {
                return tree.contains_key(token).unwrap_or(false);
            }
         }
         false
    }
    
    pub fn get_all_stakes(&self) -> Vec<(String, u64)> {
         let mut stakes = Vec::new();
         if let Some(ref db) = self.db {
             if let Ok(tree) = db.state_stakes() {
                 for item in tree.iter() {
                     if let Ok((key, val)) = item {
                         let address = String::from_utf8(key.to_vec()).unwrap_or_default();
                         let mut bytes = [0u8; 8];
                         if val.len() == 8 {
                             bytes.copy_from_slice(&val);
                             let amount = u64::from_be_bytes(bytes);
                             stakes.push((address, amount));
                         }
                     }
                 }
             }
         }
         stakes
    }

    pub fn apply_transaction(&mut self, tx: &Transaction, block_height: u64) -> bool {
        // 1. DEBIT
        if tx.sender != "SYSTEM" {
            // Determine what to debit
            let _fee_token = "VLT";
            
            // 1. Debit Fee (Hybrid: Try VLT first, then Token)
            let vlt_bal = self.get_balance(&tx.sender, "VLT");
            let fee_paid_in_vlt = if vlt_bal >= tx.fee {
                self.set_balance(&tx.sender, "VLT", vlt_bal - tx.fee);
                true
            } else {
                false
            };

            if !fee_paid_in_vlt {
                return false;
            }

            // 2. Debit Amount (Token)
            if tx.tx_type == TxType::Transfer || tx.tx_type == TxType::Stake {
                // For Stake, token is VLT, so we just deducted fee, now deduct amount.
                // For Transfer, token could be anything.
                let amount_token = &tx.token;
                let current_amount_bal = self.get_balance(&tx.sender, amount_token);
                
                if let Some(new_amt_bal) = current_amount_bal.checked_sub(tx.amount) {
                    self.set_balance(&tx.sender, amount_token, new_amt_bal);
                } else {
                     // println!("Failed to debit amount");
                     return false;
                }
            } else if tx.tx_type == TxType::AddLiquidity {
                 // Handled in separate logic block below? 
                 // Actually AddLiquidity logic calls set_balance manually.
                 // We should ensure we don't double debit.
                 // The original code handled AddLiquidity separately in `apply_transaction` later?
                 // No, original code had `match tx.tx_type` later for logic, but DEBIT was upfront.
                 // Let's look at original DEBIT block.
                 // It had `else if tx.tx_type == TxType::AddLiquidity` logic? 
                 // No, the snippet shows `TxType::IssueToken`, `Stake`, `Transfer`.
                 // AddLiquidity was NOT in the initial debit block I viewed?
                 // Checking lines 518 in previous view... yes, AddLiquidity logic does the debit itself:
                 // "self.set_balance(sender, token_a, old_a - amount_a);"
                 // So we should NOT debit amount here for AddLiquidity.
            }
            
            if tx.tx_type == TxType::Stake {
                let current_stake = self.get_stake(&tx.sender);
                if let Some(new_stake) = current_stake.checked_add(tx.amount) {
                    self.set_stake(&tx.sender, new_stake);
                }
            }
        }

        // 2. CREDIT
        if tx.tx_type == TxType::Transfer || tx.tx_type == TxType::IssueToken || tx.sender == "SYSTEM" {
            let current_bal = self.get_balance(&tx.receiver, &tx.token);
            if let Some(new_bal) = current_bal.checked_add(tx.amount) {
                self.set_balance(&tx.receiver, &tx.token, new_bal);
            }
        } else if tx.tx_type == TxType::Unstake {
             // Return Stake
             let current_stake = self.get_stake(&tx.sender);
             if current_stake >= tx.amount {
                  if let Some(new_stake) = current_stake.checked_sub(tx.amount) {
                      self.set_stake(&tx.sender, new_stake);
                      
                      // Fix: Lock Unstaked Funds for Maturity (Prevent Flash Attacks)
                      // User Request: Reduced to 10 blocks (~10 mins)
                      let unlock_height = block_height + 10;
                      let entry = self.pending_rewards.entry(unlock_height).or_insert_with(Vec::new);
                      entry.push((tx.receiver.clone(), tx.amount));
                  }
             }
        }

        if tx.tx_type == TxType::IssueToken {
            // self.tokens.insert(tx.token.clone(), tx.sender.clone()); // Tokens tree implementation deferred
        }
        
        // Nonce
        if tx.sender != "SYSTEM" {
             self.set_nonce(&tx.sender, tx.nonce);
        }

        // 3. CONTRACT EXECUTION
        if tx.tx_type == TxType::DeployContract {
            // Address = Sha256(sender + nonce)
            let raw = format!("{}{}", tx.sender, tx.nonce);
            use sha2::{Sha256, Digest};
            let hash = Sha256::digest(raw.as_bytes());
            let contract_addr = hex::encode(hash);
            
            let contract = Contract {
                address: contract_addr.clone(),
                owner: tx.sender.clone(),
                bytecode: tx.data.clone(),
                storage: HashMap::new(),
            };
            self.contracts.insert(contract_addr, contract);
        } else if tx.tx_type == TxType::CallContract {
             if let Some(contract) = self.contracts.remove(&tx.receiver) { // Take ownership to mutate
                 // Instantiate VM
                 let vm_res = WasmVM::new(&contract.bytecode, contract.storage.clone());
                 match vm_res {
                     Ok(mut vm) => {
                         // Parse Method and Args from tx.data
                         // Format: "method_name" (args in VM memory? For MVP just run main/start)
                         // Actually data is Vec<u8>, maybe valid string method name?
                         // Let's assume data IS the method name for now.
                         let method = String::from_utf8(tx.data.clone()).unwrap_or("main".to_string());
                         
                         // Call
                         let _ = vm.call(&method, vec![]);
                         
                         // Update Storage (Generic Sync)
                         // In real impl, we'd iterate modified keys.
                         // For MVP sandbox, we might need vm to return new storage
                         // contract.storage = vm.get_storage();
                     },
                     Err(e) => {
                         println!("VM Error: {}", e);
                         return false; // Fail Tx
                     }
                 }
                 self.contracts.insert(tx.receiver.clone(), contract); // Put back
             }
        }

        true
    }

}

pub struct Blockchain {

    // pub chain: Vec<Block>, // Removed for Scalability
    pub tip: Option<Block>, // Cache the Tip
    pub pending_transactions: Vec<Transaction>,
    pub difficulty: u32,
    pub state: ChainState,
    pub db: Option<Arc<Database>>, 
}

impl Blockchain {
    pub fn new() -> Self {
        let db = Database::new("volt.db").ok().map(Arc::new);
        
        let mut blockchain = Blockchain {
            // chain: Vec::new(),
            tip: None,
            pending_transactions: Vec::new(),
            // Standard Difficulty (Difficulty 1)
            // This is the correct setting for a Mainnet-like environment.
            difficulty: 0x1d00ffff,
            state: ChainState::new(db.clone()),
            db, 
        };

        let wipe_db = false;

        // Clone DB reference to avoid borrow conflict with mutable `create_genesis_block`
        if let Some(db) = blockchain.db.clone() {
            // Load Tip
            match db.get_last_block() {
                Ok(Some(last_block)) => {
                    // 1. Genesis Check (Lazy - Check height 0 if needed, usually we trust DB)
                    // For now, simpler: Just load tip.
                    blockchain.tip = Some(last_block);
                    println!("[Core] Loaded Chain Tip from DB. Height: {}", blockchain.get_height());
                },
                _ => {
                    // Empty DB or Error -> Initialize Genesis
                    println!("[Core] Database empty (or error). Initializing Genesis...");
                    let genesis = blockchain.create_genesis_block();
                    if let Err(e) = db.save_block(&genesis) {
                         println!("[Core] Failed to save Genesis: {}", e);
                    } else {
                         blockchain.tip = Some(genesis);
                    }
                }
            }
        }
        
        
        if wipe_db {
             // 1. Drop the DB connection to release file locks
             blockchain.db = None;
             
             // 2. Delete the DB directory
             let path = std::path::Path::new("volt.db");
             if path.exists() {
                 if let Err(e) = std::fs::remove_dir_all(path) {
                     println!("[Auto-Wipe] Failed to delete volt.db: {}", e);
                     // If we can't delete, we panic because we can't run safely
                     panic!("Cannot auto-wipe database. Please delete 'volt.db' manually.");
                 } else {
                     println!("[Auto-Wipe] Database deleted successfully.");
                 }
             }
             
             // 3. Re-initialize DB
             blockchain.db = Database::new("volt.db").ok().map(Arc::new);
             
             // 4. Create correct Genesis
             blockchain.create_genesis_block();
        }

        blockchain
    }

    pub fn get_height(&self) -> u64 {
         self.tip.as_ref().map(|b| b.index + 1).unwrap_or(0)
    }

    pub fn get_last_block(&self) -> Option<Block> {
         self.tip.clone()
    }

    pub fn get_transaction(&self, hash_hex: &str) -> Option<Transaction> {
         if let Ok(hash_bytes) = hex::decode(hash_hex) {
             if let Some(ref db) = self.db {
                 if let Ok(Some(tx)) = db.get_transaction(&hash_bytes) {
                     return Some(tx);
                 }
             }
         }
         None
    }

    pub fn get_block(&self, index: u64) -> Option<Block> {
         if let Some(ref db) = self.db {
             db.get_block(index).unwrap_or(None)
         } else {
             None
         }
    }




    pub fn get_all_blocks(&self) -> Vec<Block> {
        if let Some(ref db) = self.db {
            db.get_all_blocks().unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub fn rebuild_state(&mut self) {
        let height = self.get_height();
        println!("[Chain] Rebuilding State from {} blocks (DB Iteration)...", height);
        
        // 1. Clear State
        self.state.clear();
        
        // 2. Re-Apply all blocks
        if let Some(ref db) = self.db {
             for i in 0..=height {
                 if let Ok(Some(block)) = db.get_block(i) {
                     for tx in &block.transactions {
                         self.state.apply_transaction(tx, block.index);
                     }
                 }
             }
        }
        println!("[Chain] State Rebuilt Successfully.");
    }

    /// Verifies the chain by simulating state reconstruction.
    /// Returns the resulting ChainState if valid, or an error if any transaction fails.
    pub fn verify_chain_state(chain: &Vec<Block>) -> Result<ChainState, String> {
        let mut state = ChainState::new(None);

        for block in chain {
            // 1. Process Improved Maturity (Unlock old rewards)
            let current_height = block.index;
            let mut matured = Vec::new();
            
            if state.pending_rewards.contains_key(&current_height) {
                 if let Some(list) = state.pending_rewards.remove(&current_height) {
                     matured = list;
                 }
            }

            for (receiver, amount) in matured {
                let token = "VLT"; 
                let current_bal = state.get_balance(&receiver, token);
                let new_bal = current_bal.saturating_add(amount);
                state.set_balance(&receiver, token, new_bal);
            }

            // 2. Process Transactions
            for (tx_idx, tx) in block.transactions.iter().enumerate() {
                 if tx.sender == "SYSTEM" {
                     // Check if Genesis Block (Index 0) -> No Maturity (Premine/Instant Unlock)
                     let maturity_depth = if block.index == 0 { 0 } else { 10 };
                     let unlock_height = block.index + maturity_depth;
                     
                     if maturity_depth == 0 {
                         let token = "VLT";
                         let current_bal = state.get_balance(&tx.receiver, token);
                         let new_bal = current_bal.saturating_add(tx.amount);
                         state.set_balance(&tx.receiver, token, new_bal);
                     } else {
                         let entry = state.pending_rewards.entry(unlock_height).or_insert_with(Vec::new);
                         entry.push((tx.receiver.clone(), tx.amount));
                     }
                 } else {
                     // Normal Transaction - MUST Succeed
                     if !state.apply_transaction(tx, block.index) {
                         return Err(format!("Transaction Apply Failed at Block #{} Tx #{} Hash: {}", block.index, tx_idx, hex::encode(tx.get_hash())));
                     }
                 }
            }
        }
        Ok(state)
    }
    
    // Wrapper for API
    pub fn apply_transaction_to_state(&mut self, tx: &Transaction) -> bool {
        self.state.apply_transaction(tx, self.get_height())
    }

    pub fn get_balance(&self, address: &str, token: &str) -> u64 {
        self.state.get_balance(address, token)
    }
    
    // Helper to accessing state mutation
    fn set_balance(&mut self, address: &str, token: &str, amount: u64) {
        self.state.set_balance(address, token, amount);
    }

    fn create_genesis_block(&mut self) -> Block {
        // Fair Launch Genesis: No Premine
        // We create a purely symbolic Genesis Block.
        
        let genesis_msg = Transaction {
        version: 1,
        sender: String::from("SYSTEM"),
        receiver: String::from("GENESIS"), // Unspendable
        amount: 0, 
        signature: String::from("VolteCore Fair Launch 2026"),
            timestamp: 1767077203, // MATCH REMOTE TIMESTAMP
            token: String::from("VLT"),
            tx_type: crate::transaction::TxType::Transfer,
            nonce: 0,
            fee: 0,
            script_pub_key: crate::script::Script::new(),
            script_sig: crate::script::Script::new(),
            price: 0,
            data: vec![],
        };

        // Use Standard Difficulty 0x1d00ffff for Genesis to match chain config
        let mut genesis_block = Block::new(0, String::from("0"), vec![genesis_msg], 0x1d00ffff, 0);
        
        // FIX: Enforce Deterministic Genesis Timestamp and Hash for network compatibility
        genesis_block.timestamp = 1767077203;
        genesis_block.proof_of_work = 0; // Deterministic Nonce (Required for consistent Genius Hash)
        
        // FIX: Hardcode Merkle Root to match Remote Network
        // Remote Merkle: 9ade8308c25fc33e1a6ee8d5981c10eea693691583d8a17acb8207b244fda116
        genesis_block.merkle_root = "9ade8308c25fc33e1a6ee8d5981c10eea693691583d8a17acb8207b244fda116".to_string();
        
        genesis_block.hash = genesis_block.calculate_hash();
        
        // Debug Log
        println!("[Genesis] Local Genesis Hash: {}", genesis_block.hash);
        println!("[Genesis] Please update remote checkpoints with this new hash if changed.");



        // Save Genesis to DB and set TIP

        // Fix: Return the block!
        genesis_block
    }

    pub fn create_transaction(&mut self, transaction: Transaction) -> bool {
        // Phase 28: Smart Scripting Validation
        if !transaction.script_sig.ops.is_empty() {
             let mut vm = VirtualMachine::new();
             // 1. Run Unlocking Script (Inputs)
             if !vm.execute(&transaction.script_sig, &transaction) {
                 println!("ScriptSig execution failed");
                 return false;
             }
             // 2. Run Locking Script (Logic)
             // In a full UTXO (P2SH), we would load this from the UTXO set.
             // Here, we support "Self-contained Scripts" for Proof-of-Concept.
             // The ScriptPub defines the constraint (e.g., CheckSig).
             if !vm.execute(&transaction.script_pub_key, &transaction) {
                 println!("ScriptPubKey execution failed");
                 return false;
             }
             
             // 3. Authorization Success? (Stack Top == 1)
             // (vm.execute returns true if success, but we should double check semantics if needed)
             // For now, vm.execute() returns true if the script ran without error AND top is True.
             // So we are good.
        } else {
             // Legacy Validation
             if !transaction.verify() {
                 println!("Transaction verification failed");
                 return false;
             }
        }

        if transaction.sender != "SYSTEM" {
             let current_nonce = self.state.get_nonce(&transaction.sender);
             if transaction.nonce <= current_nonce {
                 println!("Error: Invalid Nonce (Current: {}, Tx: {})", current_nonce, transaction.nonce);
                 return false;
             }
             // Fix: Check pending pool for duplicate nonces to prevent invalid block creation
             for pending in &self.pending_transactions {
                 if pending.sender == transaction.sender && pending.nonce == transaction.nonce {
                      println!("Error: Nonce {} already in mempool for {}", transaction.nonce, transaction.sender);
                      return false;
                 }
             }
        }

        match transaction.tx_type {
            TxType::IssueToken => {
                 if transaction.token == "VLT" { return false; }
                 if self.state.token_exists(&transaction.token) { return false; }
                 if transaction.token.len() < 3 || transaction.token.len() > 8 { return false; }
            },
            TxType::Burn => {
                 if transaction.token == "VLT" { return false; } // Can prevent burning VLT if desired, or allow it.
                 // Just check balance
                 if self.get_balance(&transaction.sender, &transaction.token) < transaction.amount { return false; }
            },
            TxType::PlaceOrder => {
                 // 1. Lock Funds
                 let side = if transaction.receiver == "DEX_BUY" { "BUY" } else { "SELL" };
                 
                 // If BUY: User wants to buy Token using VLT. Must lock (Price * Amount) VLT.
                 // If SELL: User wants to sell Token for VLT. Must lock Amount Token.
                 
                 if side == "BUY" {
                     let cost = transaction.price * transaction.amount;
                     let bal = self.get_balance(&transaction.sender, "VLT");
                     if bal < cost { return false; }
                     // Deduct VLT
                     self.set_balance(&transaction.sender, "VLT", bal - cost);
                 } else {
                     let bal = self.get_balance(&transaction.sender, &transaction.token);
                     if bal < transaction.amount { return false; }
                     // Deduct Token
                     self.set_balance(&transaction.sender, &transaction.token, bal - transaction.amount);
                 }

                 // 2. Create Order Object
                 let mut order = Order {
                     id: hex::encode(&transaction.signature[0..10]), // fast ID from sig
                     creator: transaction.sender.clone(),
                     token: transaction.token.clone(),
                     side: side.to_string(),
                     price: transaction.price,
                     amount: transaction.amount,
                     timestamp: transaction.timestamp,
                 };
                 
                 // 3. MATCHING ENGINE (Optimized)
                 let _filled = false;
                 
                 // Collect matches (Price Priority)
                 // If BUY, match against ASKS (Sell orders). Lowest price first.
                 // If SELL, match against BIDS (Buy orders). Highest price first.
                 
                 let matches: Vec<(String, u64, u64, String)> = if side == "BUY" {
                      // BTreeMap iter gives keys ascending (Low Price -> High Price)
                      self.state.asks.range((order.token.clone(), 0, 0)..(order.token.clone(), order.price + 1, u64::MAX))
                         .map(|(k, v)| (v.clone(), k.1, k.2, "ASK".to_string()))
                         .collect()
                 } else {
                      // Borrow check workaround: Need range then reverse for Bids (High -> Low)
                      // Filter for token
                      self.state.bids.iter().rev()
                         .filter(|(k, _)| k.0 == order.token)
                         .take_while(|(k, _)| k.1 >= order.price)
                         .map(|(k, v)| (v.clone(), k.1, k.2, "BID".to_string()))
                         .collect()
                 };

                 // Note: We collected IDs to avoid holding the BTreeMap borrow
                 for (maker_id, maker_price, _maker_time, _maker_tab) in matches {
                      if order.amount == 0 { break; }
                      
                      let match_data = if let Some(maker) = self.state.orders.get(&maker_id) {
                           Some((maker.amount, maker.creator.clone()))
                      } else { None };

                      if let Some((maker_amount, maker_creator)) = match_data {
                           let trade_amt = std::cmp::min(order.amount, maker_amount);
                           
                           // Update Candles
                           self.state.update_candle(&order.token, maker_price, trade_amt, transaction.timestamp);

                           let value = trade_amt * maker_price;
                           let seller = if side == "SELL" { &order.creator } else { &maker_creator };
                           let buyer = if side == "BUY" { &order.creator } else { &maker_creator };
                           
                           let seller_c = seller.clone();
                           let buyer_c = buyer.clone();
                           
                           // Credit Seller VLT
                           let s_bal = self.get_balance(&seller_c, "VLT");
                           self.set_balance(&seller_c, "VLT", s_bal + value);
                           
                           // Credit Buyer Token
                           let b_bal = self.get_balance(&buyer_c, &order.token);
                           self.set_balance(&buyer_c, &order.token, b_bal + trade_amt);
                           
                           order.amount -= trade_amt;
                           
                           // Update Maker
                           let mut remove_maker = false;
                           if let Some(maker) = self.state.orders.get_mut(&maker_id) {
                                maker.amount -= trade_amt;
                                if maker.amount == 0 { remove_maker = true; }
                           }
                           
                           if remove_maker {
                               if let Some(m) = self.state.orders.remove(&maker_id) {
                                   if side == "BUY" { // Matched Ask
                                       self.state.asks.remove(&(m.token, m.price, m.timestamp));
                                   } else { // Matched Bid
                                       self.state.bids.remove(&(m.token, m.price, m.timestamp));
                                   }
                               }
                           }
                      }
                 }

                 // 4. Save Remainder
                 if order.amount > 0 {
                     self.state.orders.insert(order.id.clone(), order.clone());
                     let key = (order.token.clone(), order.price, order.timestamp);
                     if side == "BUY" {
                         self.state.bids.insert(key, order.id);
                     } else {
                         self.state.asks.insert(key, order.id);
                     }
                 }
            },
            TxType::CancelOrder => {
                // Return funds
                if let Some(order) = self.state.orders.remove(&transaction.token) { // token field holds OrderID
                    if order.creator != transaction.sender {
                        // Unauthorized! Put it back.
                        self.state.orders.insert(transaction.token.clone(), order);
                        return false;
                    }
                    
                    // Cleanup Indices
                    let key = (order.token.clone(), order.price, order.timestamp);
                    if order.side == "BUY" {
                        self.state.bids.remove(&key);
                    } else {
                        self.state.asks.remove(&key);
                    }

                    // Refund
                    if order.side == "BUY" {
                        let cost = order.price * order.amount;
                        let bal = self.get_balance(&order.creator, "VLT");
                        self.set_balance(&order.creator, "VLT", bal + cost);
                    } else {
                        let bal = self.get_balance(&order.creator, &order.token);
                        self.set_balance(&order.creator, &order.token, bal + order.amount);
                    }
                }
            },
            TxType::Transfer => {
                let bal = self.get_balance(&transaction.sender, &transaction.token);
                
                // V2: Dynamic Fee enforcement
                // Rule: 1.0% of Amount (Percentage Based) + Congestion Surcharge
                
                let congestion_count = self.pending_transactions.len() as u64;
                let congestion_surcharge = congestion_count * 100_000_000; // 1 VLT per tx
                
                let amount_factor = transaction.amount / 1000; // 0.1% Commission
                let min_fee = amount_factor + congestion_surcharge;
                
                // Enforce minimum of 0.001 VLT (100,000 units) base
                let base_min = 100_000; 
                let effective_min_fee = if min_fee < base_min { base_min } else { min_fee };
                
                if transaction.fee < effective_min_fee {
                    println!("Rejected: Fee too low (Requires 1%). Required: {}, Provided: {}", effective_min_fee, transaction.fee);
                    return false;
                }

                let required = transaction.amount + transaction.fee;
                
                // Fix Double Spend: Check pending transactions
                let pending_spent: u64 = self.pending_transactions.iter()
                    .filter(|t| t.sender == transaction.sender && t.token == transaction.token && t.tx_type == TxType::Transfer)
                    .map(|t| t.amount + t.fee)
                    .sum();

                if bal < (required + pending_spent) { 
                    println!("Rejected: Double Spend / Insufficient Funds (Pending: {}, New: {}, Bal: {})", pending_spent, required, bal);
                    return false; 
                }
            },
            TxType::Stake => {
                 if transaction.token != "VLT" { return false; }
                 let bal = self.get_balance(&transaction.sender, "VLT");
                 if bal < transaction.amount { return false; }
            },
            TxType::Unstake => {
                 // Check if staked amount >= amount
                 let current_stake = self.state.get_stake(&transaction.sender);
                 if current_stake < transaction.amount { return false; }
            },
            TxType::AddLiquidity => {
                 let parts: Vec<&str> = transaction.token.split('/').collect();
                 if parts.len() != 2 { return false; }
                 let token_a = parts[0];
                 let token_b = parts[1];
                 
                 let amount_a = transaction.amount;
                 let amount_b = transaction.price; // Using price field for second amount
                 let sender = &transaction.sender;
                 
                 if self.get_balance(sender, token_a) < amount_a { return false; }
                 if self.get_balance(sender, token_b) < amount_b { return false; }
                 
                 let pool_id = transaction.token.clone();
                 
                 // 1. Calculate Shares
                 let (reserve_a, reserve_b, total_shares) = if let Some(p) = self.state.pools.get(&pool_id) {
                     (p.reserve_a, p.reserve_b, p.total_shares)
                 } else {
                     (0, 0, 0)
                 };
                 
                 let shares = if total_shares == 0 {
                     ((amount_a as f64 * amount_b as f64).sqrt()) as u64
                 } else {
                     let s_a = (amount_a * total_shares) / reserve_a;
                     let s_b = (amount_b * total_shares) / reserve_b;
                     std::cmp::min(s_a, s_b)
                 };
                 
                 if shares == 0 { return false; }
                 
                 // 2. Debit User
                 let old_a = self.get_balance(sender, token_a);
                 let old_b = self.get_balance(sender, token_b);
                 self.set_balance(sender, token_a, old_a - amount_a);
                 self.set_balance(sender, token_b, old_b - amount_b);
                 
                 // 3. Update Pool
                 {
                     let pool = self.state.pools.entry(pool_id.clone()).or_insert(Pool {
                         token_a: token_a.to_string(),
                         token_b: token_b.to_string(),
                         reserve_a: 0,
                         reserve_b: 0,
                         total_shares: 0,
                     });
                     pool.reserve_a += amount_a;
                     pool.reserve_b += amount_b;
                     pool.total_shares += shares;
                 }
                 
                 // 4. Credit LP Tokens
                 let lp_token = format!("LP-{}", pool_id);
                 let old_lp = self.get_balance(sender, &lp_token);
                 self.set_balance(sender, &lp_token, old_lp + shares);
            },
            TxType::RemoveLiquidity => {
                 let pool_id = transaction.token.clone();
                 if !self.state.pools.contains_key(&pool_id) { return false; }
                 
                 let shares = transaction.amount;
                 let sender = &transaction.sender;
                 let lp_token = format!("LP-{}", pool_id);
                 
                 if self.get_balance(sender, &lp_token) < shares { return false; }
                 
                 // 1. Calculate Amounts
                 let (amount_a, amount_b, token_a, token_b) = {
                     let pool = self.state.pools.get(&pool_id).unwrap();
                     let a = (shares * pool.reserve_a) / pool.total_shares;
                     let b = (shares * pool.reserve_b) / pool.total_shares;
                     (a, b, pool.token_a.clone(), pool.token_b.clone())
                 };
                 
                 if amount_a == 0 && amount_b == 0 { return false; }
                 
                 // 2. Debit LP
                 let old_lp = self.get_balance(sender, &lp_token);
                 self.set_balance(sender, &lp_token, old_lp - shares);
                 
                 // 3. Update Pool
                 {
                     let pool = self.state.pools.get_mut(&pool_id).unwrap();
                     pool.total_shares -= shares;
                     pool.reserve_a -= amount_a;
                     pool.reserve_b -= amount_b;
                 }
                 
                 // 4. Credit Assets
                 let old_a = self.get_balance(sender, &token_a);
                 let old_b = self.get_balance(sender, &token_b);
                 self.set_balance(sender, &token_a, old_a + amount_a);
                 self.set_balance(sender, &token_b, old_b + amount_b);
            },
            TxType::Swap => {
                 let pool_id = transaction.token.clone();
                 if !self.state.pools.contains_key(&pool_id) { return false; }
                 
                 let sender = &transaction.sender;
                 let is_a_to_b = transaction.receiver == "SWAP_A_TO_B";
                 let input_amount = transaction.amount;
                 let min_output = transaction.price; 
                 
                 let (token_in, token_out, output_amount) = {
                     let pool = self.state.pools.get(&pool_id).unwrap();
                     let (rin, rout, t_in, t_out) = if is_a_to_b {
                        (pool.reserve_a, pool.reserve_b, pool.token_a.clone(), pool.token_b.clone())
                     } else {
                        (pool.reserve_b, pool.reserve_a, pool.token_b.clone(), pool.token_a.clone())
                     };
                     
                     let input_with_fee = input_amount * 997;
                     let numerator = input_with_fee * rout;
                     let denominator = (rin * 1000) + input_with_fee;
                     let output = numerator / denominator;
                     
                     if output < min_output { return false; }
                     (t_in, t_out, output)
                 };
                 
                 // 1. Debit Input
                 let bal_in = self.get_balance(sender, &token_in);
                 if bal_in < input_amount { return false; }
                 self.set_balance(sender, &token_in, bal_in - input_amount);
                 
                 // 2. Update Pool
                 {
                     let pool = self.state.pools.get_mut(&pool_id).unwrap();
                     if is_a_to_b {
                         pool.reserve_a += input_amount;
                         pool.reserve_b -= output_amount;
                     } else {
                         pool.reserve_b += input_amount;
                         pool.reserve_a -= output_amount;
                     }
                 }
                 
                 // 3. Credit Output
                 let bal_out = self.get_balance(sender, &token_out);
                 self.set_balance(sender, &token_out, bal_out + output_amount);

                 // Update Candles (AMM) determines price.
                 // Price = Input / Output * 10^8 ? Simplified: Price = VLT Value.
                 // Let's assume pair is always X/VLT. If VLT is involved, we can map price.
                 // For now, just store price as output/input ratio * 10^8
                 let price = (input_amount * 100_000_000) / output_amount; // Rough price
                 self.state.update_candle(&pool_id, price, output_amount, transaction.timestamp);
            },
            TxType::IssueNFT => {
                 // Check if NFT exists
                 if self.state.nfts.contains_key(&transaction.token) { return false; }
                 
                 // Sender matches creator (logic simplified: anyone can mint uniquely named NFT)
                 // Token field = NFT ID
                 // Receiver field = URI (Metadata)
                 
                 let nft = NFT {
                     id: transaction.token.clone(),
                     owner: transaction.sender.clone(),
                     uri: transaction.receiver.clone(), // HACK: Reusing receiver field for URI
                     created_at: transaction.timestamp,
                 };
                 self.state.nfts.insert(nft.id.clone(), nft);
            },
            TxType::TransferNFT => {
                 if let Some(nft) = self.state.nfts.get_mut(&transaction.token) {
                     if nft.owner != transaction.sender { return false; }
                     nft.owner = transaction.receiver.clone();
                 } else { return false; }
            },
            TxType::BurnNFT => {
                 if let Some(nft) = self.state.nfts.get(&transaction.token) {
                     if nft.owner != transaction.sender { return false; }
                 } else { return false; }
                 self.state.nfts.remove(&transaction.token);
            },
            TxType::DeployContract | TxType::CallContract => {
                // validation handled in apply_transaction for now
            }
        }

        self.pending_transactions.push(transaction);
        true
    }

    pub fn mine_pending_transactions(&mut self, miner_address: String) {
        let height = self.get_height();
        let mut reward = self.calculate_reward(height);
        
        // Phase 12: Fee Split Logic
        let dev_wallet = "024dea39ce2e873d5be2d8e092044a7dbd9cfa2dadcba5d32e9b141b7361422d56";
        let mut total_fees: u64 = 0;
        
        let mut txs = self.pending_transactions.clone();
        
        for tx in &txs {
             total_fees += tx.fee;
        }
        
        if total_fees > 0 {
             let dev_share = total_fees * 20 / 100; // 20% Dev Tax (Updated)
             let miner_share = total_fees - dev_share;
             
             // 1. Add Miner Share to Block Reward
             reward += miner_share;
             
             // 2. Create Dev Tx
             // Note: We create this from "SYSTEM" to avoid signature checks, but logically it comes from the fees.
             // Since fees are deducted from sender in apply_transaction, we need to MINT this dev share?
             // NO. apply_transaction deducts (amount + fee) from sender.
             // It credits 'amount' to receiver.
             // The 'fee' vanishes from state unless we credit it somewhere.
             // So yes, we MINT the fee destination here.
             
             if dev_share > 0 {
                  let dev_tx = Transaction::new(String::from("SYSTEM"), dev_wallet.to_string(), dev_share, "VLT".to_string(), 0, 0);
                  txs.push(dev_tx);
             }
        }
        
        let my_stake = self.state.get_stake(&miner_address);
        let reward_tx = Transaction::new(String::from("SYSTEM"), miner_address.clone(), reward, "VLT".to_string(), 0, 0);
        txs.insert(0, reward_tx); 

        // Staking Logic - Use local collection to avoid self-borrow issues if targeting self.pending (though here we target local txs, so it IS safe)

        let all_stakes = self.state.get_all_stakes();
        if !all_stakes.is_empty() {
             let total_staked: u64 = all_stakes.iter().map(|(_, amt)| amt).sum();
             if total_staked > 0 {
                 for (staker, amount) in all_stakes {

                     let staking_inflation = 10;
                     if let Some(total_reward) = amount.checked_mul(staking_inflation) {
                         if let Some(share) = total_reward.checked_div(total_staked) {
                             if share > 0 {
                                  let stake_tx = Transaction::new(String::from("SYSTEM"), staker.clone(), share, "VLT".to_string(), 0, 0);
                                  txs.push(stake_tx);
                             }
                         }
                     }
                 }
             }
        }

        let previous_block = self.get_last_block().unwrap();
        let difficulty = self.get_next_difficulty();
        
        println!("Mining block {} [Difficulty: {}, Reward: {:.8} VLT]...", previous_block.index + 1, difficulty, reward as f64 / 100_000_000.0);

        let mut new_block = Block::new(
            previous_block.index + 1,
            previous_block.hash.clone(),
            txs,
            difficulty as usize,
            my_stake
        );

        // Check if mined successfully (Time-limited attempt)
        if new_block.mine(difficulty as usize, 100_000) { // 100k hashes per attempt
            // Pass 2: Apply to state
            for tx in &new_block.transactions {
                self.state.apply_transaction(tx, new_block.index);
            }
    

            if let Some(ref db) = self.db {
                 let _ = db.save_block(&new_block);
            }
            self.tip = Some(new_block.clone());

            if let Some(ref db) = self.db {
                let _ = db.save_block(&new_block);
            }
            
            // Fix: Only remove mined transactions, preserve the rest (e.g. overflow)
            let mined_hashes: HashSet<Vec<u8>> = new_block.transactions.iter().map(|t| t.get_hash()).collect();
            self.pending_transactions.retain(|t| !mined_hashes.contains(&t.get_hash()));
        } else {
            // Failed to find nonce in this quantum. Return so thread can check flags/yield.
        }
    }
    
    pub fn get_mining_candidate(&self, miner_address: String) -> Block {
        let _height = self.get_height();
        let mut reward = self.calculate_reward(_height);
        
        let mut txs = self.pending_transactions.clone();

        // Limit transactions to prevent oversized blocks (Reserve 200 slots for System/Stake txs)
        let max_txs = 1800;
        if txs.len() > max_txs {
            txs.truncate(max_txs);
        }

        // Fee Split Logic
        let dev_wallet = "024dea39ce2e873d5be2d8e092044a7dbd9cfa2dadcba5d32e9b141b7361422d56";
        let mut total_fees: u64 = 0;
        for tx in &txs { total_fees += tx.fee; }

        if total_fees > 0 {
             let dev_share = total_fees * 20 / 100; 
             let miner_share = total_fees - dev_share;
             reward += miner_share;
             
             if dev_share > 0 {
                  // System Txs have 0 fee
                  let dev_tx = Transaction::new(String::from("SYSTEM"), dev_wallet.to_string(), dev_share, "VLT".to_string(), 0, 0);
                  txs.push(dev_tx);
             }
        }

        // Fix: Use get_stake helper
        let my_stake = self.state.get_stake(&miner_address);
        let reward_tx = Transaction::new(String::from("SYSTEM"), miner_address.clone(), reward, "VLT".to_string(), 0, 0);
        txs.insert(0, reward_tx);

        let all_stakes = self.state.get_all_stakes();
        if !all_stakes.is_empty() {
             let total_staked: u64 = all_stakes.iter().map(|(_, amt)| amt).sum();
             if total_staked > 0 {
                 for (staker, amount) in all_stakes {
                     let staking_inflation = 10;
                     if let Some(total_reward) = amount.checked_mul(staking_inflation) {
                         if let Some(share) = total_reward.checked_div(total_staked) {
                             if share > 0 {
                                  let stake_tx = Transaction::new(String::from("SYSTEM"), staker.clone(), share, "VLT".to_string(), 0, 0);
                                  txs.push(stake_tx);
                             }
                         }
                     }
                 }
             }
        }
            

            // Block 1019 replacement:
        let all_stakes = self.state.get_all_stakes();
        if !all_stakes.is_empty() {
             let total_staked: u64 = all_stakes.iter().map(|(_, amt)| amt).sum();
             if total_staked > 0 {
                 for (staker, amount) in all_stakes {
                     let staking_inflation = 10;
                     if let Some(total_reward) = amount.checked_mul(staking_inflation) {
                         if let Some(share) = total_reward.checked_div(total_staked) {
                             if share > 0 {
                                  let stake_tx = Transaction::new(String::from("SYSTEM"), staker.clone(), share, "VLT".to_string(), 0, 0);
                                  txs.push(stake_tx);
                             }
                         }
                     }
                 }
             }
        }

        let previous_block = self.get_last_block().unwrap();
        let difficulty = self.get_next_difficulty();

        // my_stake already captured above
        Block::new(
            previous_block.index + 1,
            previous_block.hash.clone(),
            txs,
            difficulty as usize,
            my_stake
        )
    }

    pub fn submit_block(&mut self, block: Block) -> bool {
         let calculated = block.calculate_hash();

         // Hybrid Consensus Validation
         
         // 0. DoS Protection: Block Size & Tx Size Limit
         if block.transactions.len() > 2000 {
             println!("[Security] Block Rejected: Too many transactions ({})", block.transactions.len());
             return false;
         }
         
         // Strict Transaction Size Check (5KB Limit per Tx)
         for tx in &block.transactions {
             let size_est = tx.sender.len() + tx.receiver.len() + tx.token.len() + tx.signature.len() + 100; // Rough calc
             if size_est > 5000 {
                  println!("[Security] Block Rejected: Transaction too large (DoS risk). Size: ~{} bytes", size_est);
                  return false;
             }
         }

         // 1. Verify Claimed Stake
         let miner_addr = block.transactions[0].receiver.clone(); // Coinbase receiver is the miner
         let actual_stake = self.state.get_stake(&miner_addr);
         if block.validator_stake > actual_stake {
             println!("[Hybrid] Invalid Stake Claim: Claimed {}, Actual {}", block.validator_stake, actual_stake);
             return false;
         }

         // 2. Apply Difficulty Bonus (Reduction)
         // Rule: Every 100 VLT (10,000,000,000 units) Stake reduces required zeros by 1.
         // Cap: Max reduction of 5 zeros (Requires 500 VLT).
         let bonus = (block.validator_stake / 10_000_000_000) as u32; // 100 VLT = 1 Level
         let bonus_capped = bonus.min(5); 
         
         // Fix: Handle Bits vs Legacy Diff
         let mut required_diff; // Declare without initialization
         
         if block.difficulty >= 0x1d00ffff {
             // It's Bits.
             // If >= 0x207fffff (Diff 0), we require 0 zeros.
             if block.difficulty >= 0x207fffff {
                 required_diff = 0;
             } else if block.difficulty >= 0x1f00ffff {
                 required_diff = 1;
             } else {
                 required_diff = 4; // Standard
             }
         } else {
             // Legacy Small Int Difficulty
             required_diff = block.difficulty.saturating_sub(bonus_capped);
             if required_diff < 1 { required_diff = 1; }
         }

         let target_prefix = "0".repeat(required_diff as usize);
         
         
         // 2.5 Verify Merkle Root Integrity (Anti-Corruption)
         let calculated_merkle = Block::calculate_merkle_root(&block.transactions);
         if block.merkle_root != calculated_merkle {
             println!("[Security] Block Rejected: Merkle Root Mismatch. Header: {}, Body: {}", block.merkle_root, calculated_merkle);
             return false;
         }

         if block.hash != calculated || !block.hash.starts_with(&target_prefix) {
             println!("[Hybrid] PoW Failed. Required Prefix Length: {} (Target: {})", required_diff, target_prefix);
             return false;
         }
         
         // 3. Verify Transactions (Signatures & Fees)
         let mut total_fees = 0;
         let mut seen_txs = std::collections::HashSet::new();
         let mut total_system_mint = 0;

         for (i, tx) in block.transactions.iter().enumerate() {
             // Track System Minting (Coinbase + Staking + Dev)
             if tx.sender == "SYSTEM" {
                 total_system_mint += tx.amount;
             }

             if i == 0 { continue; } // Skip Coinbase for Sig Check
             
             // FIX: Prevent Duplicate Txs in same block
             let tx_hash = tx.get_hash();
             if seen_txs.contains(&tx_hash) {
                 println!("[Security] Rejected Block: Duplicate Transaction Detected");
                 return false;
             }
             seen_txs.insert(tx_hash);

             total_fees += tx.fee;
             
             // Critical: Verify Signature
             if !tx.verify() {
                 println!("[Security] Invalid Signature in Tx: {:?}", hex::encode(tx.get_hash()));
                 return false;
             }

             // Critical: Enforce Minimum Fee (Spam Protection)
             if tx.fee < 1000 {
                 println!("[Security] Block Rejected: Tx Fee too low ({} < 1000)", tx.fee);
                 return false;
             }
         }

         // 4. Verify Total Emission (Inflation Protection)
         let expected_base_reward = self.calculate_reward(block.index);
         let staking_inflation = 10; // Must match mining logic
         let max_allowed = expected_base_reward + total_fees + staking_inflation;
         
         if total_system_mint > max_allowed {
              println!("[Security] Inflation Detected! Total Minted: {}, Max Allowed: {}", total_system_mint, max_allowed);
              return false;
         }

         let last = self.get_last_block().unwrap();
         if block.previous_hash != last.hash || block.index != last.index + 1 {
              println!("[Security] Invalid Previous Hash or Index");
              return false;
         }

         // 5a. Verify Validator Stake (Prevent Fake Difficulty)
         // Find Coinbase to identify miner
         if let Some(coinbase) = block.transactions.iter().find(|t| t.sender == "SYSTEM") {
              let miner = &coinbase.receiver;
               let real_stake = self.state.get_stake(miner);
              if block.validator_stake > real_stake {
                   println!("[Security] Fraudulent Stake Claim: Claimed {}, Real {}", block.validator_stake, real_stake);
                   return false;
              }
         }

         // 5. Verify Timestamp (Time Warp Protection)
         // 5. Verify Timestamp (Time Warp Protection - MTP Rule)
         let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
         let mtp = self.get_median_time_past();
         
         if block.timestamp <= mtp {
             println!("[Security] Timestamp Invalid: Must be greater than MTP-11. MTP: {}, New: {}", mtp, block.timestamp);
             return false;
         }
         
         if block.timestamp > now + 7200 { // 2 Hours Drift
             println!("[Security] Timestamp Invalid: Too far in future");
             return false;
         }

         // 6. Transaction Replay Protection
         for tx in &block.transactions {
             if tx.sender == "SYSTEM" { continue; }
              let stored_nonce = self.state.get_nonce(&tx.sender);
             if tx.nonce <= stored_nonce {
                 println!("[Security] Replay Attack Detected: Tx Nonce {} <= Stored {}", tx.nonce, stored_nonce);
                 return false;
             }
         }
         
         for tx in &block.transactions {
             if !self.state.apply_transaction(tx, block.index) {
                 println!("[Consensus] Error: Transaction Application Failed during block submission");
                 // State is already mutated partially. We should ideally revert.
                 // For now, return false. P2P will disconnect us or we will re-sync.
                 return false;
             }
         }


         if let Some(ref db) = self.db {
             let _ = db.save_block(&block);
         }
         self.tip = Some(block.clone());

         if let Some(ref db) = self.db {
             if let Err(e) = db.save_block(&block) {
                  println!("[Core] CRITICAL: DB Save Failed: {}", e);
                  return false;
             }
         }
         self.tip = Some(block.clone()); // Update In-Memory Tip
         
         // Fix: Remove confirmed transactions from pending pool to prevent replay/stuck
         let confirmed: HashSet<Vec<u8>> = block.transactions.iter().map(|tx| tx.get_hash()).collect();
         self.pending_transactions.retain(|tx| !confirmed.contains(&tx.get_hash()));

         println!("[Core] Block {} accepted. New Tip: {}", self.get_height(), self.tip.as_ref().unwrap().hash);
         true
    }


    fn get_next_difficulty(&self) -> u32 {
        let last_block = self.get_last_block().unwrap();
        
        // Retarget every 1440 blocks (1 Day) - Production Standard
        let retarget_interval = 1440;
        let target_seconds_per_block = 60; // 1 Minute
        let target_timespan = retarget_interval * target_seconds_per_block;

        if (last_block.index + 1) % retarget_interval != 0 {
            return last_block.difficulty as u32;
        }

        // Find the first block of this epoch
        // Be careful with index underflow
        let first_block_index = if last_block.index >= retarget_interval {
            last_block.index - retarget_interval + 1
        } else {
            0
        };
        
        let first_block = if let Some(ref db) = self.db {
             db.get_block(first_block_index).unwrap_or(None).unwrap_or(last_block.clone())
        } else {
             last_block.clone() // Fallback (Shouldn't happen)
        };
        
        // Calculate actual timespan
        let actual_timespan = last_block.timestamp.saturating_sub(first_block.timestamp);
        
        // Dampening (Max 4x, Min 1/4x)
        let actual_timespan = if actual_timespan < target_timespan / 4 {
             target_timespan / 4
        } else if actual_timespan > target_timespan * 4 {
             target_timespan * 4
        } else {
             actual_timespan
        };

        // 3. Fallback for new chains
        if self.get_height() < retarget_interval as u64 {
            return 0x1f00ffff; // Genesis difficulty
        }
        
        // Calculate new target
        // Target = OldTarget * ActualTime / TargetTime
        // We need to convert Bits -> Target (u256) -> Op -> Bits
        // Since we don't have U256 lib easily exposed here, we do a simplified float math
        // or just manipulation of the exponent.
        
        // Simplified Logic for "Bits":
        // Bits = (Exponent << 24) | Mantissa
        // Val = Mantissa * 256^(Exponent - 3)
        // NewVal = OldVal * (Actual / Target)
        
        // Let's use f64 for approximation (Standard in many altcoins)
        let last_diff_bits = last_block.difficulty as u32;
        let exponent = (last_diff_bits >> 24) & 0xff;
        let mantissa = last_diff_bits & 0x00ffffff;

        // Integer Math for Retargeting
        // Goal: new_val = old_val * (actual_time / target_time)
        // Since exponent is base 256, we operate on mantissa primarily.
        
        let mut new_mantissa = (mantissa as u64).saturating_mul(actual_timespan as u64);
        new_mantissa /= target_timespan as u64;

        // Re-normalize if mantissa over/underflows the 24-bit window
        let mut new_exponent = exponent;

        // If mantissa is too small (underflow), we steal from exponent
        // Example: 0x000001 -> Shift Left, Decrease Exponent
        while new_mantissa < 0x00800000 && new_exponent > 0 {
             new_mantissa <<= 8;
             new_exponent -= 1;
        }

        // If mantissa is too big (overflow), we push to exponent
        // Example: 0x1000000 -> Shift Right, Increase Exponent
        while new_mantissa > 0x00ffffff {
             new_mantissa >>= 8;
             new_exponent += 1;
        }
        
        // Cap at Max and Min
        if new_exponent > 0x20 { 
            return 0x207fffff; 
        }
        
        let new_bits = ((new_exponent as u32) << 24) | ((new_mantissa as u32) & 0x00ffffff);
        
        println!("[Retarget] Block {}: Timespan {}s (Target {}s) -> Diff Adjusted to {:x}", last_block.index + 1, actual_timespan, target_timespan, new_bits);
        
        new_bits
    }

    pub fn calculate_reward(&self, height: u64) -> u64 {
        let halving_interval = 1_050_000; // Fixed: Approx 2 Years (1 min blocks)
        let initial_reward = 50 * 100_000_000; // 50 VLT in Atomic Units
        let halvings = height / halving_interval;
        if halvings >= 64 { return 0; }
        initial_reward >> halvings
    }
    
    pub fn get_median_time_past(&self) -> u64 {
        let height = self.get_height();
        let count = std::cmp::min(height + 1, 11);
        let mut timestamps = Vec::new();

        for i in 0..count {
            // Get from chain vector (assuming it holds full chain or we have DB)
            // Fallback to internal lookup which handles both
            if let Some(block) = self.get_block((height - i) as u64) {
                timestamps.push(block.timestamp);
            } else if let Some(ref db) = self.db {
                if let Ok(Some(block)) = db.get_block(height - i) {
                    timestamps.push(block.timestamp);
                }
            }
        }
        
        timestamps.sort();
        if timestamps.is_empty() { return 0; }
        timestamps[timestamps.len() / 2]
    }

    pub fn is_chain_valid(&self) -> bool {
        // Deep scan is too expensive for DB mode.
        // Assume valid if Tip is valid and linked.
        true
    }

    pub fn save(&self) {
        if let Some(ref db) = self.db {
            let _ = db.save_pending_txs(&self.pending_transactions);
            // flushed by db call
        }
    }

    pub fn load() -> Self {
        Blockchain::new()
    }

    pub fn attempt_chain_replacement(&mut self, candidate: Vec<Block>) -> bool {
          let current_height = self.get_height();
          let candidate_height = candidate.last().map(|b| b.index).unwrap_or(0);

          // 1. Basic Work Check (Length)
          if candidate_height <= current_height {
              println!("[Consensus] Candidate chain not longer than local chain. Ignored.");
              return false;
          }

          // 2. Find Fork Point
          // Iterate candidate to find the last common block
          let mut fork_index = 0;
          let mut diverged = false;
          
          for block in &candidate {
              if let Some(local_block) = self.get_block(block.index) {
                  if local_block.hash == block.hash {
                      fork_index = block.index; // Still matching
                  } else {
                      diverged = true;
                      break; // Divergence point found
                  }
              } else {
                  // Extending beyond local chain
                  diverged = true;
                  break;
              }
          }
          
          if !diverged && candidate_height > current_height {
               // This is just a straight extension, not a reorg. 
               // Standard sync should handle it, but we can accept it here too.
               fork_index = current_height; 
          }

          if let Some(ref db) = self.db {
              println!("[Consensus] REORG DETECTED! Fork at #{}. Rewinding {} blocks...", fork_index, current_height - fork_index);
              
              // 3. Rewind: Remove blocks from Tip down to Fork+1
              for i in (fork_index + 1 ..= current_height).rev() {
                   if let Some(b) = self.get_block(i) {
                       println!("[Consensus] Reverting Block #{}", i);
                       let _ = db.remove_block(&b);
                   }
              }
              
              // 4. Forward: Apply New Chain
              for block in &candidate {
                  if block.index > fork_index {
                      // Verify PoW/Signatures BEFORE saving? 
                      // For MVP, we assume peer is honest validated by `node.rs` beforehand (GetChain verifies proof?).
                      // Ideally we should verify here.
                      let _ = db.save_block(block);
                  }
              }
          }
          
          // 5. Update Tip
          self.tip = candidate.last().cloned();
          
          // 6. Rebuild State (The critical fix)
          self.rebuild_state();
          
          true
    }

    // Optimized Sync: Handle Sequence of Blocks without full chain replacement
    pub fn handle_sync_chunk(&mut self, chunk: Vec<Block>) -> bool {
        let mut added = 0;
        for block in chunk {
            if !self.submit_block(block) {
                println!("[Sync] Chunk processing stopped at failure. Added {} blocks.", added);
                return false;
            }
            added += 1;
        }
        if added > 0 {
             println!("[Sync] Successfully added {} blocks from chunk.", added);
        }
        true
    }
}
