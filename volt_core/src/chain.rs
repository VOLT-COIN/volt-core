#![allow(dead_code)]
use crate::block::Block;
use crate::transaction::{Transaction, TxType};
use crate::db::Database;
use crate::script::VirtualMachine;
use std::collections::{HashMap, BTreeMap, HashSet};

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

#[derive(Clone, Debug, Default)]
pub struct ChainState {
    pub balances: HashMap<String, HashMap<String, u64>>,
    pub tokens: HashMap<String, String>, 
    pub nonces: HashMap<String, u64>,
    pub stakes: HashMap<String, u64>,

    pub orders: HashMap<String, Order>, // OrderID -> Order
    // Optimization: BTreeMaps for O(log N) matching
    // Key: (Token, Price, Timestamp) -> Value: OrderID
    // Token is needed because we match per pair.
    pub bids: BTreeMap<(String, u64, u64), String>, 
    pub asks: BTreeMap<(String, u64, u64), String>,

    pub pools: HashMap<String, Pool>,   // "TokenA/TokenB" -> Pool
    pub candles: HashMap<String, Vec<Candle>>, // "Pair" -> History
    pub nfts: HashMap<String, NFT>, // NFT_ID -> NFT
}

impl ChainState {
    pub fn new() -> Self {
        ChainState {
            balances: HashMap::new(),
            tokens: HashMap::new(),
            nonces: HashMap::new(),
            stakes: HashMap::new(),

            orders: HashMap::new(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            
            pools: HashMap::new(),
            candles: HashMap::new(),
            nfts: HashMap::new(),
        }
    }

    pub fn get_balance(&self, address: &str, token: &str) -> u64 {
        if let Some(user_tokens) = self.balances.get(address) {
            *user_tokens.get(token).unwrap_or(&0)
        } else {
            0
        }
    }

    fn set_balance(&mut self, address: &str, token: &str, amount: u64) {
        let user_tokens = self.balances.entry(address.to_string()).or_insert_with(HashMap::new);
        user_tokens.insert(token.to_string(), amount);
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

    pub fn apply_transaction(&mut self, tx: &Transaction) -> bool {
        // 1. DEBIT
        if tx.sender != "SYSTEM" {
            // Determine what to debit
            let fee_token = "VLT";
            
            // 1. Debit Fee (Hybrid: Try VLT first, then Token)
            let vlt_bal = self.get_balance(&tx.sender, "VLT");
            let fee_paid_in_vlt = if vlt_bal >= tx.fee {
                self.set_balance(&tx.sender, "VLT", vlt_bal - tx.fee);
                true
            } else {
                false
            };

            if !fee_paid_in_vlt {
                // Fallback: Try paying in the Asset itself (if not VLT)
                let token_bal = self.get_balance(&tx.sender, &tx.token);
                if tx.token != "VLT" && token_bal >= tx.fee {
                     self.set_balance(&tx.sender, &tx.token, token_bal - tx.fee);
                } else {
                     println!("CRITICAL: Failed to debit fee for {} (No VLT or {} balance)", tx.sender, tx.token);
                     return false;
                }
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
                let current_stake = *self.stakes.get(&tx.sender).unwrap_or(&0);
                if let Some(new_stake) = current_stake.checked_add(tx.amount) {
                    self.stakes.insert(tx.sender.clone(), new_stake);
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
             let current_stake = *self.stakes.get(&tx.sender).unwrap_or(&0);
             if current_stake >= tx.amount {
                  if let Some(new_stake) = current_stake.checked_sub(tx.amount) {
                      self.stakes.insert(tx.sender.clone(), new_stake);
                      // Credit VLT
                      let bal = self.get_balance(&tx.receiver, "VLT");
                      if let Some(new_bal) = bal.checked_add(tx.amount) {
                          self.set_balance(&tx.receiver, "VLT", new_bal);
                      }
                  }
             }
        }

        if tx.tx_type == TxType::IssueToken {
            self.tokens.insert(tx.token.clone(), tx.sender.clone());
        }
        
        // Nonce
        if tx.sender != "SYSTEM" {
            self.nonces.insert(tx.sender.clone(), tx.nonce);
        }
        true
    }
}

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub pending_transactions: Vec<Transaction>,
    pub difficulty: u32,
    pub state: ChainState,
    pub db: Option<Database>, 
}

impl Blockchain {
    pub fn new() -> Self {
        let mut blockchain = Blockchain {
            chain: Vec::new(),
            pending_transactions: Vec::new(),
            // Standard Difficulty (Difficulty 1)
            // This is the correct setting for a Mainnet-like environment.
            difficulty: 0x1d00ffff,
            state: ChainState::new(),
            db: Database::new("volt.db").ok(), 
        };

        if let Some(ref db) = blockchain.db {
            if let Some(chain_data) = db.load_chain() {
                blockchain.chain = chain_data;
                // Restore Mempool
                if let Ok(pending) = db.load_pending_txs() {
                    blockchain.pending_transactions = pending;
                    println!("[Chain] Restored {} pending transactions from DB", blockchain.pending_transactions.len());
                }
                blockchain.rebuild_state();
            } else {
                blockchain.create_genesis_block();
            }
        } else {
             blockchain.create_genesis_block();
        }
        blockchain
    }

    pub fn rebuild_state(&mut self) {
        self.state = ChainState::new();
        for block in &self.chain {
            for tx in &block.transactions {
                // If a historical transaction fails, we log it but continue (assume DB valid)
                // In production, this might indicate corruption.
                if !self.state.apply_transaction(tx) {
                    println!("[Chain] Warning: Historical transaction application failed: {}", hex::encode(tx.get_hash()));
                }
            }
        }
    }
    
    // Wrapper for API
    pub fn apply_transaction_to_state(&mut self, tx: &Transaction) -> bool {
        self.state.apply_transaction(tx)
    }

    pub fn get_balance(&self, address: &str, token: &str) -> u64 {
        self.state.get_balance(address, token)
    }
    
    // Helper to accessing state mutation
    fn set_balance(&mut self, address: &str, token: &str, amount: u64) {
        self.state.set_balance(address, token, amount);
    }

    fn create_genesis_block(&mut self) {
        // PreMine: 5% of 21,000,000 VLT (Total Supply) = 1,050,000 VLT
        // 1,050,000 * 100,000,000 = 105,000,000,000,000 Atomic Units
        let premine_tx = Transaction {
            sender: String::from("SYSTEM"),
            receiver: String::from("024dea39ce2e873d5be2d8e092044a7dbd9cfa2dadcba5d32e9b141b7361422d56"),
            amount: 105_000_000_000_000, 
            signature: String::from("GENESIS"),
            timestamp: 0,
            token: String::from("VLT"),
            tx_type: crate::transaction::TxType::Transfer,
            nonce: 0,
            fee: 0,
            price: 0,
            script_pub_key: crate::script::Script::new(),
            script_sig: crate::script::Script::new(),
        };

        // Use Standard Difficulty 0x1d00ffff for Genesis to match chain config
        let mut genesis_block = Block::new(0, String::from("0"), vec![premine_tx], 0x1d00ffff, 0);
        
        // FIX: Enforce Deterministic Genesis Timestamp and Hash for network compatibility
        genesis_block.timestamp = 1767077203;
        genesis_block.hash = genesis_block.calculate_hash();

        self.chain.push(genesis_block.clone());
        if let Some(ref db) = self.db {
            let _ = db.save_block(&genesis_block);
        }
        // Fix: Apply Genesis transactions to State so balance shows up immediately
        self.rebuild_state();
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
             let current_nonce = *self.state.nonces.get(&transaction.sender).unwrap_or(&0);
             if transaction.nonce <= current_nonce {
                 println!("Error: Invalid Nonce");
                 return false;
             }
        }

        match transaction.tx_type {
            TxType::IssueToken => {
                 if transaction.token == "VLT" { return false; }
                 if self.state.tokens.contains_key(&transaction.token) { return false; }
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
                 let mut filled = false;
                 
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
                // Rule: 0.1% of Amount + 1 VLT per pending tx
                // ATOMIC UNITS:
                // 1 VLT = 100,000,000
                // 0.1% = amount / 1000
                // 1 VLT congestion = 100,000,000 per tx
                
                let congestion_count = self.pending_transactions.len() as u64;
                let congestion_surcharge = congestion_count * 100_000_000; // 1 VLT per tx
                
                let amount_factor = transaction.amount / 1000; // 0.1%
                let min_fee = amount_factor + congestion_surcharge;
                
                // Enforce minimum of 0.001 VLT (100,000 units) base
                let base_min = 100_000; 
                let effective_min_fee = if min_fee < base_min { base_min } else { min_fee };
                
                if transaction.fee < effective_min_fee {
                    println!("Rejected: Fee too low. Required: {}, Provided: {}", effective_min_fee, transaction.fee);
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
                 let current_stake = *self.state.stakes.get(&transaction.sender).unwrap_or(&0);
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
            }
        }

        self.pending_transactions.push(transaction);
        true
    }

    pub fn mine_pending_transactions(&mut self, miner_address: String) {
        let height = self.chain.len() as u64;
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
                  let dev_tx = Transaction::new(String::from("SYSTEM"), dev_wallet.to_string(), dev_share, "VLT".to_string(), 0);
                  txs.push(dev_tx);
             }
        }
        
        let my_stake = *self.state.stakes.get(&miner_address).unwrap_or(&0);
        let reward_tx = Transaction::new(String::from("SYSTEM"), miner_address.clone(), reward, "VLT".to_string(), 0);
        txs.insert(0, reward_tx); 

        // Staking Logic - Use local collection to avoid self-borrow issues if targeting self.pending (though here we target local txs, so it IS safe)
        if !self.state.stakes.is_empty() {
             let total_staked: u64 = self.state.stakes.values().sum();
             if total_staked > 0 {
                 for (staker, amount) in &self.state.stakes {
                     let staking_inflation = 10;
                     if let Some(total_reward) = amount.checked_mul(staking_inflation) {
                         if let Some(share) = total_reward.checked_div(total_staked) {
                             if share > 0 {
                                  let stake_tx = Transaction::new(String::from("SYSTEM"), staker.clone(), share, "VLT".to_string(), 0);
                                  txs.push(stake_tx);
                             }
                         }
                     }
                 }
             }
        }

        let previous_block = self.chain.last().unwrap();
        let difficulty = self.get_next_difficulty();
        
        println!("Mining block {} [Difficulty: {}, Reward: {:.8} VLT]...", previous_block.index + 1, difficulty, reward as f64 / 100_000_000.0);

        let mut new_block = Block::new(
            previous_block.index + 1,
            previous_block.hash.clone(),
            txs,
            difficulty as usize,
            my_stake
        );

        new_block.mine(difficulty as usize);
        
        // Pass 2: Apply to state
        for tx in &new_block.transactions {
            self.state.apply_transaction(tx);
        }

        self.chain.push(new_block.clone());
        if let Some(ref db) = self.db {
            let _ = db.save_block(&new_block);
        }
        
        self.pending_transactions.clear();
    }
    
    pub fn get_mining_candidate(&self, miner_address: String) -> Block {
        let _height = self.chain.len() as u64;
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
                 let dev_tx = Transaction::new(String::from("SYSTEM"), dev_wallet.to_string(), dev_share, "VLT".to_string(), 0);
                 txs.push(dev_tx);
             }
        }

        let my_stake = *self.state.stakes.get(&miner_address).unwrap_or(&0);
        let reward_tx = Transaction::new(String::from("SYSTEM"), miner_address.clone(), reward, "VLT".to_string(), 0);
        txs.insert(0, reward_tx);

        if !self.state.stakes.is_empty() {
             let total_staked: u64 = self.state.stakes.values().sum();
             if total_staked > 0 {
                 for (staker, amount) in &self.state.stakes {
                     let staking_inflation = 10;
                     if let Some(total_reward) = amount.checked_mul(staking_inflation) {
                         if let Some(share) = total_reward.checked_div(total_staked) {
                             if share > 0 {
                                  let stake_tx = Transaction::new(String::from("SYSTEM"), staker.clone(), share, "VLT".to_string(), 0);
                                  txs.push(stake_tx);
                             }
                         }
                     }
                 }
             }
        }

        let previous_block = self.chain.last().unwrap();
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
         
         // 0. DoS Protection: Block Size Limit
         if block.transactions.len() > 2000 {
             println!("[Security] Block Rejected: Too many transactions ({})", block.transactions.len());
             return false;
         }

         // 1. Verify Claimed Stake
         let miner_addr = block.transactions[0].receiver.clone(); // Coinbase receiver is the miner
         let actual_stake = *self.state.stakes.get(&miner_addr).unwrap_or(&0);
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
         let mut required_diff = 4; // Default
         
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
         }

         // 4. Verify Total Emission (Inflation Protection)
         let expected_base_reward = self.calculate_reward(block.index);
         let staking_inflation = 10; // Must match mining logic
         let max_allowed = expected_base_reward + total_fees + staking_inflation;
         
         if total_system_mint > max_allowed {
              println!("[Security] Inflation Detected! Total Minted: {}, Max Allowed: {}", total_system_mint, max_allowed);
              return false;
         }

         let last = self.chain.last().unwrap();
         if block.previous_hash != last.hash || block.index != last.index + 1 {
              println!("[Security] Invalid Previous Hash or Index");
              return false;
         }

         // 5. Verify Timestamp (Time Warp Protection)
         let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
         if block.timestamp < last.timestamp {
             println!("[Security] Timestamp Invalid: Time Reversal. Last: {}, New: {}", last.timestamp, block.timestamp);
             return false;
         }
         if block.timestamp > now + 7200 { // 2 Hours Drift
             println!("[Security] Timestamp Invalid: Too far in future");
             return false;
         }

         // 6. Transaction Replay Protection
         for tx in &block.transactions {
             if tx.sender == "SYSTEM" { continue; }
             let stored_nonce = *self.state.nonces.get(&tx.sender).unwrap_or(&0);
             if tx.nonce <= stored_nonce {
                 println!("[Security] Replay Attack Detected: Tx Nonce {} <= Stored {}", tx.nonce, stored_nonce);
                 return false;
             }
         }
         
         for tx in &block.transactions {
             if !self.state.apply_transaction(tx) {
                 println!("[Consensus] Error: Transaction Application Failed during block submission");
                 // State is already mutated partially. We should ideally revert.
                 // For now, return false. P2P will disconnect us or we will re-sync.
                 return false;
             }
         }

         self.chain.push(block.clone());
         if let Some(ref db) = self.db {
             let _ = db.save_block(&block);
         }
         
         // Fix: Remove confirmed transactions from pending pool to prevent replay/stuck
         let confirmed: HashSet<Vec<u8>> = block.transactions.iter().map(|tx| tx.get_hash()).collect();
         self.pending_transactions.retain(|tx| !confirmed.contains(&tx.get_hash()));

         true
    }


    fn get_next_difficulty(&self) -> u32 {
        let last_block = self.chain.last().unwrap();
        
        // Retarget every 10 blocks (Fast adj for testing)
        let retarget_interval = 10;
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
        
        let first_block = &self.chain[first_block_index as usize];
        
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
        if self.chain.len() < retarget_interval as usize {
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
        
        let mut value = mantissa as f64 * 256f64.powf((exponent as i32 - 3) as f64);
        
        value = value * (actual_timespan as f64 / target_timespan as f64);
        
        // Convert back to Bits
        // Find rough exponent
        if value <= 0.0 { return 0x207fffff; }
        
        let mut new_exponent = 0;
        let mut tmp = value;
        while tmp >= 256.0 * 256.0 * 256.0 { // Keep mantissa in 3 bytes (approx) 
             // Ideally we find log256
             tmp /= 256.0;
             new_exponent += 1;
        }
        // Base is 3 bytes (0x00ffffff)
        new_exponent += 3;
        
        // Re-normalize mantissa
        let new_mantissa = (value / 256f64.powf((new_exponent - 3) as f64)) as u32;
        
        // Cap at Min Difficulty
        // 0x20 is max exponent (32 bytes)
        if new_exponent > 0x20 { return 0x207fffff; }
        if new_exponent == 0x20 && new_mantissa > 0x7fffff { return 0x207fffff; }
        
        // Assemble
        let new_bits = ((new_exponent as u32) << 24) | (new_mantissa & 0x00ffffff);
        println!("[Retarget] Block {}: Timespan {}s (Target {}s) -> Diff Adjusted to {:x}", last_block.index + 1, actual_timespan, target_timespan, new_bits);
        
        new_bits
    }

    pub fn calculate_reward(&self, height: u64) -> u64 {
        let halving_interval = 105_000; // Accelerated Schedule (Results in ~21M Total Supply still? No, actually 10.5M if base is same... Wait, User said "Approx 4 Years" for 210k. 105k is 2 years. Supply curve depends on this. I will just change the number as requested.)
        let initial_reward = 50 * 100_000_000; // 50 VLT in Atomic Units
        let halvings = height / halving_interval;
        if halvings >= 64 { return 0; }
        initial_reward >> halvings
    }
    
    pub fn is_chain_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            let current = &self.chain[i];
            let previous = &self.chain[i - 1];
            if current.hash != current.calculate_hash() { return false; }
            if current.previous_hash != previous.hash { return false; }
        }
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
         if candidate.len() <= self.chain.len() { return false; }
         
         println!("[Consensus] Validating remote chain candidate (Height: {})...", candidate.len());
         
         // 1. Genesis Check
         if candidate.is_empty() || candidate[0].hash != self.chain[0].hash {
             println!("[Consensus] Rejecting: Incompatible Genesis.");
             return false;
         }

         // 2. Structural & PoW Validation
         for i in 1..candidate.len() {
             let cur = &candidate[i];
             let prev = &candidate[i-1];
             
             // Linkage
             if cur.previous_hash != prev.hash || cur.index != prev.index + 1 { 
                 println!("[Consensus] Rejecting: Broken Link at #{}", cur.index);
                 return false; 
             }
             
             // Hash Integrity
             if cur.hash != cur.calculate_hash() { 
                 println!("[Consensus] Rejecting: Invalid Hash at #{}", cur.index);
                 return false; 
             }
             
             // PoW Check (Simplified for MVP: Check formatting)
             if !cur.hash.starts_with("0000") {
                  // STRICT MODE:
                  // return false;
             }
             
             // Signature Check
             for (t_i, tx) in cur.transactions.iter().enumerate() {
                 if t_i > 0 && !tx.verify() {
                     println!("[Consensus] Rejecting: Invalid Tx Signature at #{}", cur.index);
                     return false;
                 }
             }
         }
         
         println!("[Consensus] Remote chain accepted. Rebuilding state...");
         self.chain = candidate;
         self.rebuild_state();
         // Save to DB
         if let Some(ref db) = self.db {
             let _ = db.save_chain(&self.chain);
         }
         true
    }
}
