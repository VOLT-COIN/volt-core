use std::net::{TcpListener, TcpStream};
use std::io::{BufRead, BufReader, Write};
use std::thread;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use crate::chain::Blockchain;
use std::time::{SystemTime, UNIX_EPOCH};
use k256::ecdsa::signature::Signer; // Added for PPLNS signing
use k256::elliptic_curve::sec1::ToEncodedPoint; // For dynamic address derivation

#[derive(Serialize, Deserialize, Debug)]
struct RpcRequest {
    id: Option<u64>,
    method: String,
    params: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RpcResponse {
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PoolMode {
    SOLO,
    PPS,
    PPLNS,
    FPPS, // Full Pay Per Share (Reward + Fees)
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Share {
    miner: String,
    difficulty: f64,
    timestamp: u64,
}

// CONFIG: Pool Fee (0.0 = 0%, 0.02 = 2%)
const POOL_FEE: f64 = 0.0; 

pub struct StratumServer {
    blockchain: Arc<Mutex<Blockchain>>,
    port: u16,
    pool_mode: Arc<Mutex<PoolMode>>,
    shares: Arc<Mutex<Vec<Share>>>,
}

impl StratumServer {
    pub fn new(blockchain: Arc<Mutex<Blockchain>>, port: u16, mode: PoolMode) -> Self {
        StratumServer { 
            blockchain, 
            port,
            pool_mode: Arc::new(Mutex::new(mode)),
            shares: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn start(&self) {
        let port = self.port;
        let chain_ref = self.blockchain.clone();
        let mode_ref = self.pool_mode.clone();
        let shares_ref = self.shares.clone();
        
        thread::spawn(move || {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).expect("Failed to bind Stratum port");
            println!("[Stratum] Listening on 0.0.0.0:{} [Mode: {:?}]", port, *mode_ref.lock().unwrap());
            
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let chain = chain_ref.clone();
                        let mode = mode_ref.clone();
                        let shares = shares_ref.clone();
                        thread::spawn(move || handle_client(stream, chain, mode, shares));
                    }
                    Err(e) => println!("Connection failed: {}", e),
                }
            }
        });

        // -------------------------------------------------------------
        // PPS PAYOUT PROCESSOR (Runs if Mode == PPS)
        // -------------------------------------------------------------
        let settings_mode = *self.pool_mode.lock().unwrap();
        if settings_mode == PoolMode::PPS {
            let chain_payout = self.blockchain.clone();
            thread::spawn(move || {
                loop {
                    thread::sleep(std::time::Duration::from_secs(1800)); // Check every 30 mins
                    
                    println!("[Pool PPS] Processing Payouts...");
                    
                    // 1. Load Pool Key
                    let pool_priv_key_hex = std::fs::read_to_string("pool_key.txt")
                         .unwrap_or_else(|_| "00".repeat(32)).trim().to_string();
                    
                    if let Ok(key_bytes) = hex::decode(&pool_priv_key_hex) {
                        if let Ok(signing_key) = k256::ecdsa::SigningKey::from_slice(&key_bytes) {
                             // Derive Addr
                             let verifying_key = signing_key.verifying_key();
                             let pub_key_bytes = verifying_key.to_encoded_point(true);
                             let pool_addr_dynamic = hex::encode(pub_key_bytes.as_bytes());
                             let pool_addr = pool_addr_dynamic.as_str();

                             // 2. Lock Chain (Short duration)
                             let mut txs_to_push = Vec::new();
                             let mut updates = Vec::new(); // (Miner, NewBalance)
                             
                             {
                                 let chain = chain_payout.lock().unwrap();
                                 if let Some(ref db) = chain.db {
                                     let balances = db.get_all_miner_balances();
                                     
                                     // Calculate Nonce Base
                                     let mut current_nonce = *chain.state.nonces.get(pool_addr).unwrap_or(&0);
                                     for tx in &chain.pending_transactions {
                                         if tx.sender == pool_addr && tx.nonce > current_nonce {
                                             current_nonce = tx.nonce;
                                         }
                                     }
                                     
                                     for (miner, bal) in balances {
                                         let fee = 100_000;
                                         if bal >= 10_000_000 && bal > fee { // Threshold: 0.1 VLT
                                             current_nonce += 1;
                                             let net_payout = bal - fee;
                                             let mut tx = crate::transaction::Transaction::new(
                                                 pool_addr.to_string(), miner.clone(), net_payout, "VLT".to_string(), current_nonce
                                             );
                                             tx.sign(&signing_key);
                                             txs_to_push.push(tx);
                                             updates.push((miner, bal));
                                         }
                                     }
                                 }
                             } // Release Chain Lock

                             // 3. Apply Updates (Push Txs & Debit Ledger)
                             if !txs_to_push.is_empty() {
                                 println!("[Pool PPS] Sending {} Payouts...", txs_to_push.len());
                                 let mut chain = chain_payout.lock().unwrap();
                                 
                                 for tx in txs_to_push {
                                     chain.pending_transactions.push(tx);
                                 }
                                 
                                 if let Some(ref db) = chain.db {
                                     for (miner, waiting_bal) in updates {
                                         let _ = db.debit_miner(&miner, waiting_bal);
                                         println!("[Pool PPS] Paid {} VLT to {}", waiting_bal as f64 / 1e8, miner);
                                     }
                                 }
                                 
                                 // CRITICAL: Save chain to persist pending transactions!
                                 chain.save();
                             }
                        }
                    }
                }
            });
        }
    }
}

fn handle_client(
    stream: TcpStream, 
    chain: Arc<Mutex<Blockchain>>, 
    mut mode_ref: Arc<Mutex<PoolMode>>,
    shares_ref: Arc<Mutex<Vec<Share>>>
) {
    let peer_addr = stream.peer_addr().unwrap_or(std::net::SocketAddr::from(([0,0,0,0], 0)));
    println!("[Stratum] Client connected: {}", peer_addr);
    
    let stream_reader = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut stream_writer_notify = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut stream_writer_resp = stream; // Move original stream here

    // Shared State between Reader and Notifier
    let session_miner_addr = Arc::new(Mutex::new("SYSTEM_POOL".to_string()));
    let current_block_template = Arc::new(Mutex::new(None::<crate::block::Block>));
    let is_authorized = Arc::new(Mutex::new(false));
    let last_notified_height = Arc::new(Mutex::new(0u64));

    // 1. Spawn Notifier Thread (Proactive Broadcast)
    let chain_notify = chain.clone();
    let miner_notify = session_miner_addr.clone();
    let block_notify = current_block_template.clone();
    let auth_notify_thread = is_authorized.clone(); // Renamed for clarity
    let height_notify = last_notified_height.clone();

    thread::spawn(move || {
        loop {
            thread::sleep(std::time::Duration::from_millis(500));
            
            // Check if authorized
            if !*auth_notify_thread.lock().unwrap() { continue; }

            let current_height = {
                if let Ok(c) = chain_notify.lock() {
                    c.chain.len() as u64
                } else { continue; }
            };

            let last_h = *height_notify.lock().unwrap();
            let now_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let should_update = current_height != last_h || (now_time % 30 == 0); // Update every 30s

            // Notify if New Block (Height changed) OR Periodic Refresh
            if should_update {
                // Generate New Job
                let miner = miner_notify.lock().unwrap().clone();
                let new_block = { // Removed mut, not needed for get_mining_candidate
                    if let Ok(c) = chain_notify.lock() {
                        c.get_mining_candidate(miner)
                    } else { continue; }
                };
                
                // Update Template
                *block_notify.lock().unwrap() = Some(new_block.clone());
                *height_notify.lock().unwrap() = current_height;

                // Send Notify
                let ntime = format!("{:08x}", new_block.timestamp);
                // FIX: Unique Job ID (Height + Timestamp) to force miner update on periodic refresh
                let job_id = format!("{}_{}", new_block.index, new_block.timestamp);
                
                let prev_bytes = hex::decode(&new_block.previous_hash).unwrap_or(vec![0u8; 32]);
                let mut prev_le = prev_bytes;
                prev_le.reverse();
                let prev_hash_hex = hex::encode(prev_le);
                
                let reward_amt = new_block.transactions[0].amount;
                let amt_hex = hex::encode(reward_amt.to_le_bytes());
                
                // Dynamic Coinb1
                let height_bytes = (new_block.index as u32).to_le_bytes();
                let height_push = format!("04{}", hex::encode(height_bytes));
                let coinb1 = format!("010000000100000000000000000000000000000000000000000000000000000000ffffffff0d{}", height_push);
                
                let coinb2 = format!("ffffffff01{}1976a91439209d6f37e633202573205730305f523030303088ac00000000", amt_hex);
                let bits_hex = format!("{:08x}", new_block.difficulty);

                // Merkle Branch
                let mut branch: Vec<String> = Vec::new();
                let mut hashes: Vec<Vec<u8>> = new_block.transactions.iter().map(|tx| tx.get_hash()).collect();
                let mut match_idx = 0;
                while hashes.len() > 1 {
                    if hashes.len() % 2 != 0 { hashes.push(hashes.last().unwrap().clone()); }
                    let mut new_hashes = Vec::new();
                    if match_idx % 2 == 0 {
                        if match_idx + 1 < hashes.len() {
                            branch.push(hex::encode(&hashes[match_idx + 1]));
                        }
                    }
                    for chunk in hashes.chunks(2) {
                         use sha2::{Sha256, Digest};
                         let mut hasher = Sha256::new();
                         hasher.update(&chunk[0]); hasher.update(&chunk[1]);
                         let r1 = hasher.finalize();
                         let mut h2 = Sha256::new(); h2.update(r1);
                         new_hashes.push(h2.finalize().to_vec());
                    }
                    hashes = new_hashes;
                    match_idx /= 2;
                }

                let notify = serde_json::json!({
                    "id": null, "method": "mining.notify",
                    "params": [ job_id, prev_hash_hex, coinb1, coinb2, branch, "00000001", bits_hex, ntime, true ]
                });
                
                if let Ok(n_str) = serde_json::to_string(&notify) {
                    if stream_writer_notify.write_all((n_str + "\n").as_bytes()).is_err() {
                        break; // Exit if pipe broken
                    }
                }
            }
        }
    });


    // 2. Reader Loop (Responses & Submissions)
    let mut reader = BufReader::new(stream_reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                if let Ok(req) = serde_json::from_str::<RpcRequest>(&line) {
                    let mut result = None;
                    
                    match req.method.as_str() {
                        "mining.subscribe" => {
                            result = Some(serde_json::json!([
                                [ ["mining.set_difficulty", "0.1"], ["mining.notify", "1"] ],
                                "00000000", 4
                            ]));
                        },
                        "mining.authorize" => {
                            result = Some(serde_json::json!(true));
                            *is_authorized.lock().unwrap() = true; // Use original Arc, not the moved clone
                            
                            // Send Initial Diff (Standard)
                            let diff_notify = serde_json::json!({ "id": null, "method": "mining.set_difficulty", "params": [0.1] });
                            let _ = stream_writer_resp.write_all((serde_json::to_string(&diff_notify).unwrap() + "\n").as_bytes());

                            if let Some(user) = req.params.get(0).and_then(|v| v.as_str()) {
                                *session_miner_addr.lock().unwrap() = user.to_string();
                            }
                            // Trigger immediate update by resetting height
                            *last_notified_height.lock().unwrap() = 0; 
                        },
                        "mining.submit" => {
                            // Params: [worker_name, job_id, extranonce2, ntime, nonce]
                            if let (Some(jid), Some(ex2), Some(ntime_hex), Some(nonce_hex)) = (
                                req.params.get(1).and_then(|v|v.as_str()), 
                                req.params.get(2).and_then(|v|v.as_str()), 
                                req.params.get(3).and_then(|v|v.as_str()),
                                req.params.get(4).and_then(|v|v.as_str())
                            ) {
                                let template_guard = current_block_template.lock().unwrap();
                                if let Some(block_template) = template_guard.as_ref() {
                                    // Robust check: Job ID must contain the Index
                                    if jid.starts_with(&block_template.index.to_string()) {
                                        let mut block = block_template.clone();
                                        
                                        // 1. Reconstruct Merkle Root
                                        let reward_amt = block.transactions[0].amount;
                                        let amt_hex = hex::encode(reward_amt.to_le_bytes());
                                        
                                        let height_bytes = (block.index as u32).to_le_bytes();
                                        let height_push = format!("04{}", hex::encode(height_bytes));
                                        let coinb1 = format!("010000000100000000000000000000000000000000000000000000000000000000ffffffff0d{}", height_push);
                                        let coinb2 = format!("ffffffff01{}1976a91439209d6f37e633202573205730305f523030303088ac00000000", amt_hex);
                                        
                                        let extra_nonce_1 = "00000000";
                                        let coinb = format!("{}{}{}{}", coinb1, extra_nonce_1, ex2, coinb2);
                                        
                                        if let Ok(coinbase_bytes) = hex::decode(&coinb) {
                                            use sha2::{Sha256, Digest};
                                            let mut hasher = Sha256::new(); hasher.update(&coinbase_bytes);
                                            let r1 = hasher.finalize();
                                            let mut h2 = Sha256::new(); h2.update(r1);
                                            
                                            // Update Body (For Integrity Check)
                                            let mut tx = block.transactions[0].clone();
                                            let mut script_data = Vec::new();
                                            script_data.extend_from_slice(&height_bytes);
                                            script_data.extend_from_slice(&[0,0,0,0]); // extra1
                                            if let Ok(ex2_bytes) = hex::decode(ex2) {
                                                script_data.extend(ex2_bytes);
                                            }
                                            
                                            // Update Tx
                                            let mut tx = block.transactions[0].clone();
                                            // Fix: Use builder pattern correctly (push consumes self)
                                            tx.script_sig = crate::script::Script::new().push(crate::script::OpCode::OpPush(script_data));
                                            block.transactions[0] = tx;
                                            
                                            // Now recalculate root from transactions
                                            block.merkle_root = crate::block::Block::calculate_merkle_root(&block.transactions);
                                            
                                        }

                                        // 2. Update Header
                                        if let Ok(n) = u32::from_str_radix(nonce_hex, 16) { block.proof_of_work = n.swap_bytes(); }
                                        if let Ok(t) = u32::from_str_radix(ntime_hex, 16) { block.timestamp = t as u64; }
                                        
                                        block.hash = block.calculate_hash();
                                        
                                        // 3. Validate
                                        // Standard PoW Check
                                        // For 0x1d00ffff, hash must start with approx "00000000"
                                        // We will enforce the standard 4-zero Stratum check for now as requested.
                                        let is_block = block.hash.starts_with("0000");

                                        // Only print if block or valid share (reduce spam)
                                        // Only print if block or valid share (reduce spam)
                                        if is_block {
                                            let miner_addr = session_miner_addr.lock().unwrap().clone();
                                            println!("[Pool] BLOCK FOUND by {}! Hash: {}", miner_addr, block.hash);
                                            // PPLNS LOGIC: Distribute Rewards
                                            
                                            // 1. Submit Block (Critical Section)
                                            let current_height_val = {
                                                let mut chain_lock = chain.lock().unwrap();
                                                let block_clone = block.clone();
                                                
                                                if chain_lock.submit_block(block_clone) {
                                                    chain_lock.save();
                                                    result = Some(serde_json::json!(true));
                                                    chain_lock.chain.len() as u64
                                                } else {
                                                    0 // Failed
                                                }
                                            }; // LOCK RELEASED HERE? NO, explicitly drop or use scope.
                                            
                                            // We need to drop lock BEFORE PPLNS math, but we are inside `chain_lock` scope.
                                            // Let's drop it explicitly.
                                            // drop(chain_lock); // Not needed, scope handles it.

                                            if current_height_val > 0 {
                                                // ---------------------------------------------------------
                                                // PPLNS PAYOUT LOGIC (Unlocked Phase)
                                                // ---------------------------------------------------------
                                                let current_mode = *mode_ref.lock().unwrap();
                                                if current_mode == PoolMode::PPLNS {
                                                    let total_valid_shares = shares_ref.lock().unwrap().len() as u64;
                                                    if total_valid_shares > 0 {
                                                        println!("[Pool PPLNS] Block Accepted! Distributing rewards via PPLNS to {} shares...", total_valid_shares);
                                                        
                                                        // Pool Config (IO - Slow)
                                                        let pool_priv_key_hex = std::fs::read_to_string("pool_key.txt")
                                                            .unwrap_or_else(|_| "00".repeat(32)).trim().to_string();

                                                        // Load Key
                                                        if let Ok(key_bytes) = hex::decode(&pool_priv_key_hex) {
                                                            if let Ok(signing_key) = k256::ecdsa::SigningKey::from_slice(&key_bytes) {
                                                                // Address Derivation
                                                                let verifying_key = signing_key.verifying_key();
                                                                let pub_key_bytes = verifying_key.to_encoded_point(true);
                                                                let pool_addr_dynamic = hex::encode(pub_key_bytes.as_bytes());
                                                                let pool_addr = pool_addr_dynamic.as_str();
                                                                
                                                                println!("[Pool Config] Loaded Owner Address: {}", pool_addr_dynamic);

                                                                // Group by Miner (CPU Bound)
                                                                let mut miner_scores = std::collections::HashMap::new();
                                                                let shares = shares_ref.lock().unwrap();
                                                                for s in shares.iter() {
                                                                    *miner_scores.entry(s.miner.clone()).or_insert(0) += 1;
                                                                }
                                                                drop(shares); // Release shares early

                                                                // ---------------------------------------------------------
                                                                // PHASE 3: Transaction Creation & Push (Re-acquire Chain Lock)
                                                                // ---------------------------------------------------------
                                                                let mut chain_lock = chain.lock().unwrap();
                                                                
                                                                // Calculate Reward (Fast)
                                                                let total_reward_gross = chain_lock.calculate_reward(current_height_val);
                                                                let total_reward = (total_reward_gross as f64 * (1.0 - POOL_FEE)) as u64;
                                                                println!("[Pool PPLNS] Block Reward: {} VLT (Fee: {}%) -> Distributing: {} VLT", 
                                                                    total_reward_gross as f64 / 1e8, POOL_FEE * 100.0, total_reward as f64 / 1e8);
                                                                
                                                                // Calculate Nonce Base
                                                                let mut current_nonce = *chain_lock.state.nonces.get(pool_addr).unwrap_or(&0);
                                                                for tx in &chain_lock.pending_transactions {
                                                                    if tx.sender == pool_addr && tx.nonce > current_nonce {
                                                                        current_nonce = tx.nonce;
                                                                    }
                                                                }
                                                                
                                                                // Generate Transactions
                                                                // Generate Transactions
                                                                let mut distributed = 0;
                                                                // Use index to identify the last miner for dust adjustment (optional, or just cap)
                                                                for (i, (miner, score)) in miner_scores.iter().enumerate() {
                                                                    let share_ratio = *score as f64 / total_valid_shares as f64;
                                                                    let mut gross_payout = (total_reward as f64 * share_ratio) as u64;
                                                                    
                                                                    // SAFETY CHECK: Ensure we never exceed total_reward
                                                                    if distributed + gross_payout > total_reward {
                                                                        gross_payout = total_reward.saturating_sub(distributed);
                                                                    }
                                                                    distributed += gross_payout;

                                                                    let fee = 100_000; // Standard Tx Fee
                                                                    
                                                                    if gross_payout > fee {
                                                                        let net_payout = gross_payout - fee;
                                                                        current_nonce += 1;
                                                                        let mut tx = crate::transaction::Transaction::new(
                                                                            pool_addr.to_string(), miner.clone(), net_payout, "VLT".to_string(), current_nonce
                                                                        );
                                                                        tx.sign(&signing_key);
                                                                        println!("[Pool PPLNS] Payout: {} VLT (Gross: {}, Fee: {}) to {} (Nonce: {})", 
                                                                            net_payout as f64 / 1e8, 
                                                                            gross_payout as f64 / 1e8,
                                                                            fee as f64 / 1e8,
                                                                            miner, 
                                                                            current_nonce
                                                                        );
                                                                        chain_lock.pending_transactions.push(tx);
                                                                    } else {
                                                                        // println!("[Pool PPLNS] Skipped Dust Payout: {} VLT to {}", gross_payout as f64 / 1e8, miner);
                                                                    }
                                                                }
                                                                println!("[Pool PPLNS] Total Distributed: {} / {} VLT", distributed as f64 / 1e8, total_reward as f64 / 1e8);
                                                                
                                                                // Save Txs
                                                                chain_lock.save();
                                                                
                                                                // Clear Shares
                                                                shares_ref.lock().unwrap().clear();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                                                
                                            // Handle Failure Case
                                            if current_height_val == 0 {
                                                result = Some(serde_json::json!(false));
                                            }

                                        } else {
                                            // Share Acceptance
                                            {
                                                let mut s_lock = shares_ref.lock().unwrap();
                                                // Ring Buffer (Stats)
                                                if s_lock.len() > 5000 { s_lock.remove(0); }
                                                let miner_addr = session_miner_addr.lock().unwrap().clone();
                                                s_lock.push(Share {
                                                    miner: miner_addr.clone(),
                                                    difficulty: block.difficulty as f64,
                                                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                                                });
                                            } // Unlock shares here
                                            
                                            let miner_addr = session_miner_addr.lock().unwrap().clone();
                                            
                                            // PPS LOGIC: Immediate Credit to Ledger
                                            let current_mode = *mode_ref.lock().unwrap();
                                            if current_mode == PoolMode::PPS {
                                                let chain_lock = chain.lock().unwrap(); // Acquire Lock
                                                if let Some(ref db) = chain_lock.db {
                                                    // Formula: (PoolDiff / NetDiff) * BlockReward
                                                    let pool_diff = 0.1;
                                                    
                                                    // Dynamic Reward (Halving Aware)
                                                    let current_height = chain_lock.chain.len() as u64;
                                                    let block_reward = chain_lock.calculate_reward(current_height + 1); // +1 because we are mining the NEXT block
                                                    
                                                    // Convert Bits to Difficulty (Approx for Testnet/MVP)
                                                    // 0x1d00ffff = Diff 1
                                                    let bits = block.difficulty;
                                                    let net_diff = if bits == 0x1d00ffff { 1.0 } else { 
                                                        // Fallback: If bits are roughly standard (high 8 bits = exponent)
                                                        // Diff = 0x00ffff * 2**(8*(0x1d - 3)) / target
                                                        // Simplify: Just use 1.0 for now if unknown bits 
                                                        1.0 
                                                    };

                                                    if net_diff > 0.0 {
                                                        let ratio = pool_diff / net_diff as f64;
                                                        let gross_reward = (ratio * block_reward as f64) as u64;
                                                        let net_reward = (gross_reward as f64 * (1.0 - POOL_FEE)) as u64;
                                                        
                                                        if net_reward > 0 {
                                                            let _ = db.credit_miner(&miner_addr, net_reward);
                                                            // println!("[Pool PPS] Share Accepted! Credit: {} VLT (Fee: {}%) to {}", net_reward as f64 / 1e8, POOL_FEE * 100.0, miner_addr);
                                                        }
                                                    }
                                                }
                                            }

                                            result = Some(serde_json::json!(true)); 
                                        }
                                    }
                                }
                            }
                        },
                        _ => {}
                    }
                    
                    if let Some(res) = result {
                        let resp = RpcResponse { id: req.id, result: Some(res), error: None };
                        if let Ok(resp_str) = serde_json::to_string(&resp) {
                            let _ = stream_writer_resp.write_all((resp_str + "\n").as_bytes());
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }
}
