use std::net::{TcpListener, TcpStream};
use std::io::{BufRead, BufReader, Write};
use std::thread;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use crate::chain::Blockchain;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tungstenite::{Message, WebSocket};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RpcRequest {
    id: Option<u64>,
    method: String,
    params: Vec<serde_json::Value>,
}

use ripemd::{Ripemd160, Digest}; // Added for P2PKH
use crate::wallet::Wallet;


#[derive(Serialize, Deserialize, Debug, Clone)]
struct RpcResponse {
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum PoolMode {
    SOLO,
    PPS,
    PPLNS,
    FPPS,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Share {
    pub miner: String,
    pub difficulty: f64,
    pub timestamp: u64,
}

const _POOL_FEE: f64 = 0.0; 
const MAX_CONNECTIONS: usize = 1000;
use std::sync::atomic::{AtomicUsize, Ordering};

// Helper: Generate mining.notify fields
fn create_mining_notify(
    next_block: &crate::block::Block,
    job_id: &str,
    pool_addr_hex: &str
) -> serde_json::Value {
    // ... existed ...
    // 1. Previous Hash (Send as-is / Big Endian from DB)
    // Stratum miners typically expect BE string and reverse it themselves for the header.
    // If we send LE, they reverse to BE -> Mismatch.
    // let prev = hex::decode(&next_block.previous_hash).unwrap_or(vec![0;32]);
    // let _prev_hex = hex::encode(prev); // Unused warning fix
     
    // 2. Coinbase Part 1
    // Height (LE) -> Pushed as hex
    let h_bytes = (next_block.index as u32).to_le_bytes();
    let h_push = format!("0c{}", hex::encode(h_bytes)); // PUSH 12 bytes
    let cb1 = format!("010000000100000000000000000000000000000000000000000000000000000000ffffffff0d{}", h_push);
    
    // 3. Coinbase Part 2 (Payout Script)
    // Server Wallet P2PKH
    let pub_key_bytes = hex::decode(pool_addr_hex).unwrap_or(vec![0;33]);
    
    use sha2::{Sha256, Digest};
    use ripemd::Ripemd160;
    
    let mut sha = Sha256::new();
    sha.update(&pub_key_bytes);
    let sha_hash = sha.finalize();
    
    let mut rip = Ripemd160::new();
    rip.update(&sha_hash);
    let pub_key_hash = rip.finalize();
    let pub_key_hash_hex = hex::encode(pub_key_hash);

    let reward = next_block.transactions[0].amount;
    let amt_hex = hex::encode(reward.to_le_bytes()); // 8 bytes LE
    
    let cb2 = format!("ffffffff01{}1976a914{}88ac00000000", amt_hex, pub_key_hash_hex);

    // 4. Branch (Merkle Path)
    let mut branch = Vec::new();
    let mut hashes: Vec<Vec<u8>> = next_block.transactions.iter().map(|tx| tx.get_hash()).collect();
    if hashes.len() > 1 {
       if hashes.len() % 2 != 0 { hashes.push(hashes.last().unwrap().clone()); }
       // Simple Single-Branch for 1 TX (Coinbase) + n Txs
       if hashes.len() > 1 { branch.push(hex::encode(&hashes[1])); }
    }

    // FIX: Difficulty is already stored as Compact Target in Block struct (e.g. 0x1d00ffff)
    // We must send it as Big Endian Hex (Standard Stratum).
    // If we send LE ("ffff001d"), miner parses as 0xffff001d -> Wrong Header Bytes.
    // We want miner to see 0x1d00ffff.
    let nbits = next_block.difficulty; 
    let bits_hex = hex::encode(nbits.to_be_bytes()); // BE Encoded Compact Bits

    // Version (BE) - Standard Stratum sends Version as BE Hex.
    // Miner reverses to LE for header.
    let version_hex = hex::encode(1u32.to_be_bytes());

    // Time (BE) - Standard Stratum sends Time as BE Hex.
    // Miner reverses to LE for header.
    let ntime_hex = hex::encode((next_block.timestamp as u32).to_be_bytes());

    serde_json::json!({
        "id": null, "method": "mining.notify",
        "params": [ job_id, next_block.previous_hash, cb1, cb2, branch, version_hex, bits_hex, ntime_hex, true ]
    })
} 

// Optimization: Shared Job State to reduce Mutex Contention
struct JobState {
    job_id: String,
    notify_json: serde_json::Value,
    block_template: Option<crate::block::Block>,
    difficulty: u32,
    timestamp: u64,
}

pub struct StratumServer {
    blockchain: Arc<Mutex<Blockchain>>,
    port: u16,
    pool_mode: Arc<Mutex<PoolMode>>,
    shares: Arc<Mutex<Vec<Share>>>,
    server_wallet: Arc<Mutex<Wallet>>,
    active_connections: Arc<AtomicUsize>,
    shared_job: Arc<Mutex<JobState>>, // Global Job Source
}

impl StratumServer {
    pub fn new(blockchain: Arc<Mutex<Blockchain>>, port: u16, mode: PoolMode, shares: Arc<Mutex<Vec<Share>>>, server_wallet: Arc<Mutex<Wallet>>) -> Self {
        StratumServer { 
            blockchain, 
            port,
            pool_mode: Arc::new(Mutex::new(mode)),
            shares,
            server_wallet,
            active_connections: Arc::new(AtomicUsize::new(0)),
            shared_job: Arc::new(Mutex::new(JobState {
                job_id: "INIT".to_string(),
                notify_json: serde_json::Value::Null,
                block_template: None,
                difficulty: 0,
                timestamp: 0
            })),
        }
    }

    pub fn start(&self) {
        let port = self.port;
        let chain_ref = self.blockchain.clone();
        let mode_ref = self.pool_mode.clone();
        let shares_ref = self.shares.clone();
        let wallet_ref = self.server_wallet.clone();
        let active_conns = self.active_connections.clone();
        let shared_job = self.shared_job.clone();
        
        // -------------------------------------------------------------
        // GLOBAL JOB UPDATER (High Efficiency)
        // -------------------------------------------------------------
        {
            let chain = chain_ref.clone();
            let wallet = wallet_ref.clone();
            let job_state = shared_job.clone();
            
            thread::spawn(move || {
                let miner_addr = "SYSTEM_POOL".to_string(); // Placeholder, internal generator
                println!("[Stratum] Job Updater Thread Started");
                
                loop {
                    thread::sleep(Duration::from_millis(500));
                    
                    let (h, next_block) = {
                        let c = chain.lock().unwrap();
                         // Optimization: Don't get candidate if height hasn't changed AND time < 30s?
                         // For now, keep logic simple: Get candidate.
                        (c.get_height(), c.get_mining_candidate(miner_addr.clone()))
                    };
                    
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs();
                    
                    // Check if update needed
                    let mut update_needed = false;
                    {
                        let state = job_state.lock().unwrap();
                        let last_h = state.block_template.as_ref().map(|b| b.index).unwrap_or(0);
                        if h != last_h || now % 60 == 0 { // Update every 60s or new block (Reduce Stale Shares)
                            update_needed = true;
                        }
                    }
                    
                    if update_needed {
                         let job_id = format!("{}_{}", next_block.index, next_block.timestamp);
                         let pool_addr = wallet.lock().unwrap().get_address();
                         
                         if pool_addr == "LOCKED" {
                             println!("[Stratum] WARNING: Wallet LOCKED. Cannot generate valid jobs.");
                             continue;
                         }

                         let notify = create_mining_notify(&next_block, &job_id, &pool_addr);
                         
                         let mut state = job_state.lock().unwrap();
                         state.job_id = job_id;
                         state.notify_json = notify;
                         state.block_template = Some(next_block.clone());
                         state.difficulty = next_block.difficulty;
                         state.timestamp = next_block.timestamp;
                    }
                }
            });
        }
        
        let shared_job_workers = self.shared_job.clone();

        thread::spawn(move || {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).expect("Failed to bind Stratum port");
            println!("[Stratum] Listening on 0.0.0.0:{} [Mode: {:?}]", port, *mode_ref.lock().unwrap());
            
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let current_conns = active_conns.load(Ordering::Relaxed);
                        if current_conns >= MAX_CONNECTIONS {
                            println!("[Stratum] Max Connections Reached ({}/{}). Dropping...", current_conns, MAX_CONNECTIONS);
                            continue; // Drop connection
                        }
                        
                        active_conns.fetch_add(1, Ordering::Relaxed);
                        
                        let chain = chain_ref.clone();
                        let mode = mode_ref.clone();
                        let shares = shares_ref.clone();
                        let wallet = wallet_ref.clone();
                        let active_conns_inner = active_conns.clone();
                        let job_source = shared_job_workers.clone();

                        thread::spawn(move || {
                            // Protocol Detection
                            let mut buffer = [0; 4];
                            // Peek can fail if socket closed immediately
                            let is_websocket = if stream.peek(&mut buffer).is_ok() {
                                buffer.starts_with(b"GET ")
                            } else { false };

                            if is_websocket {
                                match tungstenite::accept(stream) {
                                    Ok(socket) => {
                                        handle_client_ws(socket, chain, mode, shares, wallet, job_source);
                                    }
                                    Err(e) => println!("[Stratum] WS Handshake Failed: {}", e),
                                }
                            } else {
                                handle_client(stream, chain, mode, shares, wallet, job_source);
                            }
                            
                            // Connection Closed - Decrement
                            active_conns_inner.fetch_sub(1, Ordering::Relaxed);
                        });
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
                    thread::sleep(Duration::from_secs(1800)); // Check every 30 mins
                    
                    println!("[Pool PPS] Processing Payouts...");
                    
                    let pool_priv_key_hex = std::fs::read_to_string("pool_key.txt")
                         .unwrap_or_else(|_| "00".repeat(32)).trim().to_string();
                    
                    if let Ok(key_bytes) = hex::decode(&pool_priv_key_hex) {
                        if let Ok(signing_key) = k256::ecdsa::SigningKey::from_slice(&key_bytes) {
                             // Derive Addr
                             let verifying_key = signing_key.verifying_key();
                             let pub_key_bytes = verifying_key.to_encoded_point(true);
                             let pool_addr_dynamic = hex::encode(pub_key_bytes.as_bytes());
                             let pool_addr = pool_addr_dynamic.as_str();

                             let mut txs_to_push = Vec::new();
                             let mut updates = Vec::new(); // (Miner, NewBalance)
                             
                             {
                                 let chain = chain_payout.lock().unwrap();
                                 if let Some(ref db) = chain.db {
                                     let balances = db.get_all_miner_balances();
                                     let mut current_nonce = chain.state.get_nonce(pool_addr);
                                     for tx in &chain.pending_transactions {
                                         if tx.sender == pool_addr && tx.nonce > current_nonce {
                                             current_nonce = tx.nonce;
                                         }
                                     }
                                     
                                     for (miner, bal) in balances {
                                         let fee = 100_000;
                                         if bal >= 10_000_000 && bal > fee { 
                                             current_nonce += 1;
                                             let net_payout = bal - fee;
                                             let mut tx = crate::transaction::Transaction::new(
                                                 pool_addr.to_string(), miner.clone(), net_payout, "VLT".to_string(), current_nonce, 0
                                             );
                                             tx.sign(&signing_key);
                                             txs_to_push.push(tx);
                                             updates.push((miner, bal));
                                         }
                                     }
                                 }
                             } 

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
                                 chain.save();
                             }
                        }
                    }
                }
            });
        }
    }
}

// -------------------------------------------------------------------------
// SHARED LOGIC: Process Request (Pure Function approach)
// -------------------------------------------------------------------------
// Returns: (Response JSON Option, Notification JSON Option for broadcast?)
// Since Notification logic is 'Pull' based on state changes in the loop, we check that separately.
// This function handles: subscribe, authorize, submit.

fn process_rpc_request(
    req: RpcRequest,
    chain: &Arc<Mutex<Blockchain>>,
    _mode_ref: &Arc<Mutex<PoolMode>>,
    shares_ref: &Arc<Mutex<Vec<Share>>>,
    server_wallet: &Arc<Mutex<Wallet>>,
    session_miner_addr: &Arc<Mutex<String>>,
    current_block_template: &Arc<Mutex<Option<crate::block::Block>>>,
    is_authorized: &Arc<Mutex<bool>>,
    last_notified_height: &Arc<Mutex<u64>>,
    extra_nonce_1_ref: &Arc<Mutex<String>>,
    last_job_id_ref: &Arc<Mutex<String>>, // Passed strict job id
    prev_job_id_ref: &Arc<Mutex<String>>, // Added: Previous Job ID
    prev_block_template_ref: &Arc<Mutex<Option<crate::block::Block>>> // Added: Previous Template
) -> Option<serde_json::Value> {
    
    match req.method.as_str() {
        "mining.subscribe" => {
            let en1 = extra_nonce_1_ref.lock().unwrap().clone();
            // Valid Stratum Response: [[ ["mining.set_difficulty", "id1"], ["mining.notify", "id2"] ], Extranonce1, Extranonce2_Size]
            // We use Unique Strings for IDs to prevent confusion with difficulty values.
            Some(serde_json::json!([
                [ ["mining.set_difficulty", "sd"], ["mining.notify", "sn"] ],
                en1, 4
            ]))
        },
        "mining.authorize" => {
            *is_authorized.lock().unwrap() = true;
            if let Some(user_full) = req.params.get(0).and_then(|v| v.as_str()) {
                let addr_part = user_full.split('.').next().unwrap_or(user_full);
                *session_miner_addr.lock().unwrap() = addr_part.to_string();
                println!("[Stratum] Authorized Miner: {} (Worker: {})", addr_part, user_full);
            }
            // Reset height to force immediate notify
            *last_notified_height.lock().unwrap() = 0; 
            Some(serde_json::json!(true))
        },
        "mining.submit" => {
            if let (Some(jid), Some(ex2), Some(ntime_hex), Some(nonce_hex)) = (
                req.params.get(1).and_then(|v|v.as_str()), 
                req.params.get(2).and_then(|v|v.as_str()), 
                req.params.get(3).and_then(|v|v.as_str()),
                req.params.get(4).and_then(|v|v.as_str())
            ) {
                // Strict Job ID Check with 1-Deep History Buffer
                let current_job = last_job_id_ref.lock().unwrap().clone();
                let prev_job = prev_job_id_ref.lock().unwrap().clone();
                
                let mut target_template: Option<crate::block::Block> = None;
                let mut is_stale = false;

                if jid == current_job {
                     // Current Job
                     target_template = current_block_template.lock().unwrap().clone();
                } else if jid == prev_job && !prev_job.is_empty() {
                     // Previous Job (Latency/Stale) - Valid for Payouts, Invalid for Block
                     target_template = prev_block_template_ref.lock().unwrap().clone();
                     is_stale = true;
                     // println!("[Stratum] Processing Stale Share for Job: {}", jid);
                } else {
                    println!("[Stratum] Rejected Share (Unknown Job: {} | Curr: {})", jid, current_job);
                    return Some(serde_json::json!(false));
                }

                if let Some(block_template) = target_template {
                    // Reconstruct Block
                    let mut block = block_template.clone();
                                        // Reconstruct Block (Sync with create_mining_notify)
                        let reward_amt = block.transactions[0].amount;
                        let amt_hex = hex::encode(reward_amt.to_le_bytes());
                        let height_bytes = (block.index as u32).to_le_bytes();
                        
                        // Debug Ex2
                        // println!("[Stratum Debug] ... "); 


                        // Dynamic Script Length Calculation
                        // ScriptSig = PUSH(Height) + Height + PUSH(ExtraNonce) + En1 + Ex2
                        // Wait. Bitcoin standard Stratum puts Height alone.
                        // Then En1/Ex2 are usually in the script too.
                        // Our protocol: Push Height, Pad Zeros to 12 bytes? No.
                        // We generated "0c" + Height + En1 + Ex2.
                        // "0c" matches 12 bytes total. Length(Height=4 + En1=4 + Ex2=4) = 12.
                        // If Ex2 is longer, we MUST update the opcode.
                        
                        let total_extra_len = 4 + 4 + (ex2.len() / 2); // Height(4) + En1(4) + Ex2(Bytes)
                        let push_opcode = match total_extra_len {
                            0..=75 => format!("{:02x}", total_extra_len), // Direct byte
                            _ => "4c".to_string(), // OP_PUSHDATA1 (Not handled fully here, assuming small ex2)
                        };
                        
                        // BUT create_mining_notify hardcoded "0c" and "0d".
                        // If we change it here, it mismatches notify!
                        // So correct fix is to ensure notify uses same logic or accept mismatch if notification was wrong.
                        // But Notify sends coinb1 ending in 0d....
                        // Let's assume notify used standard 4-byte Ex2 logic.
                        // If Ex2 is NOT 8 chars (4 bytes), we have a problem.
                        
                        let h_push = format!("{}{}", push_opcode, hex::encode(height_bytes)); 
                        
                        // Recalc Script Len (PUSH + Len)
                        // Script = Opcode(1) + Height(4) + En1(4) + Ex2(Len/2).
                        let script_len = 1 + total_extra_len; 
                        let script_len_hex = format!("{:02x}", script_len);
                        
                        let coinb1 = format!("010000000100000000000000000000000000000000000000000000000000000000ffffffff{}{}", script_len_hex, h_push);
                        
                        // Dynamic P2PKH Script (Server Wallet)
                        let pool_addr_hex = server_wallet.lock().unwrap().get_address();
                        let pub_key_bytes = hex::decode(&pool_addr_hex).unwrap_or(vec![0;33]);

                        use sha2::{Sha256, Digest};
                        use ripemd::{Ripemd160, Digest as RipeDigest};
                        
                        let mut sha = Sha256::new();
                        sha.update(&pub_key_bytes);
                        let sha_hash = sha.finalize();
                        
                        let mut rip = Ripemd160::new();
                        rip.update(&sha_hash);
                        let pub_key_hash = rip.finalize();
                        let pub_key_hash_hex = hex::encode(pub_key_hash);
                        
                        let coinb2 = format!("ffffffff01{}1976a914{}88ac00000000", amt_hex, pub_key_hash_hex);
                        let extra_nonce_1 = extra_nonce_1_ref.lock().unwrap().clone(); // Use session En1
                        let coinb = format!("{}{}{}{}", coinb1, extra_nonce_1, ex2, coinb2);
                        
                        // println!("[Stratum Debug] Reconstructed Coinbase: {}", coinb);

                        
                        if let Ok(coinbase_bytes) = hex::decode(&coinb) {
                             use sha2::{Sha256, Digest};
                             let mut hasher = Sha256::new(); hasher.update(&coinbase_bytes);
                             let r1 = hasher.finalize();
                             let mut h2 = Sha256::new(); h2.update(r1);
                             let _coinbase_hash = h2.finalize();
                             
                             let mut script_data = Vec::new();
                             script_data.extend_from_slice(&height_bytes);
                             script_data.extend_from_slice(&[0,0,0,0]);
                             if let Ok(ex2_bytes) = hex::decode(ex2) { script_data.extend(ex2_bytes); }
                             
                             let mut tx = block.transactions[0].clone();
                             tx.script_sig = crate::script::Script::new().push(crate::script::OpCode::OpPush(script_data));
                             block.transactions[0] = tx; // Store for valid chain data
                             
                             // Force Merkle Root from Manual Coinbase (Exact match with Miner)
                             // This bypasses potential serialization mismatches in Transaction struct
                             let mut sha_tx = sha2::Sha256::new();
                             sha_tx.update(&coinbase_bytes);
                             let sha_tx_res = sha_tx.finalize();
                             let mut sha_tx2 = sha2::Sha256::new();
                             sha_tx2.update(&sha_tx_res);
                             let coinbase_hash_manual = sha_tx2.finalize(); // Internal (BE)
                             
                             // Update Merkle Root
                             if block.transactions.len() == 1 {
                                 // Single Tx: Root = Coinbase Hash (Reversed for LE)
                                 let mut root_le = coinbase_hash_manual.to_vec();
                                 root_le.reverse();
                                 // println!("[Stratum Debug] Calculated Merkle Root (LE): {}", hex::encode(&root_le));
                                 block.merkle_root = hex::encode(root_le);
                             } else {
                                 // Multi-Tx: Determine if we trust manual hash or struct hash
                                 // Ideally we should trust manual hash for the FIRST element.
                                 // Let's manually reconstruct the Merkle Tree with [coinbase_hash_manual, tx1, tx2...]
                                 // But Block::calculate_merkle_root takes &Vec<Transaction>.
                                 // We need to override it.
                                 let mut hashes: Vec<Vec<u8>> = block.transactions.iter().skip(1).map(|t| t.get_hash()).collect();
                                 hashes.insert(0, coinbase_hash_manual.to_vec());
                                 
                                 // Re-implement simplified Merkle Loop here to ensure consistency
                                 while hashes.len() > 1 {
                                     if hashes.len() % 2 != 0 { hashes.push(hashes.last().unwrap().clone()); }
                                     let mut new_hashes = Vec::new();
                                     for chunk in hashes.chunks(2) {
                                         let mut hasher = sha2::Sha256::new();
                                         hasher.update(&chunk[0]);
                                         hasher.update(&chunk[1]);
                                         let res = hasher.finalize();
                                         let mut hasher2 = sha2::Sha256::new();
                                         hasher2.update(res);
                                         new_hashes.push(hasher2.finalize().to_vec());
                                     }
                                     hashes = new_hashes;
                                 }
                                 let root_be = hashes[0].clone();
                                 // let mut root_le = root_be;
                                 // root_le.reverse(); // FIX: Do not reverse. Internal Block struct uses BE.
                                 block.merkle_root = hex::encode(root_be);
                             }
                        }

                        // Parse Nonce (Standard Stratum: BE Hex String -> u32 -> LE in Header)
                        if let Ok(n) = u32::from_str_radix(nonce_hex, 16) { block.proof_of_work = n; }
                        
                        // Parse Time (Standard Stratum: BE Hex String -> u32 -> LE in Header)
                        // Note: We sent ntime as BE Hex in notify. Miner should return it similarly.
                        // We parse it as a number (BE), and Block::calculate_hash converts it to LE bytes.
                        if let Ok(t) = u32::from_str_radix(ntime_hex, 16) { 
                            block.timestamp = t as u64; 
                        } else {
                            // Fallback if hex parsing fails (unlikely)
                        // Fallback if hex parsing fails (unlikely)
                            println!("[Stratum] ERROR: Failed to parse ntime: {}", ntime_hex);
                        }
                        
                        block.hash = block.calculate_hash();


                        // TARGET CHECKS
                        // TARGET CHECKS
                        // We use simple string prefix checks for MVP optimization.
                        
                        let is_valid_block = block.hash.starts_with("00000000"); // Diff 1

                        // SHARE CHECK (Strict Mode)
                        // Must meet Share Difficulty (0.0001) which is "0000" start.
                        let is_valid_share = block.hash.starts_with("0000"); 

                        if is_valid_block {
                             println!("[Pool] BLOCK FOUND! Hash: {}", block.hash);
                             let mut chain_lock = chain.lock().unwrap();
                             
                             if is_stale {
                                 // Do NOT submit stale block to chain (it will fail anyway).
                                 // But we count it as a share below.
                             } else if chain_lock.submit_block(block.clone()) {
                                 chain_lock.save();
                                 println!("[PPLNS] Block Found! Distributing Rewards...");
                                 // ... Payouts ...
                                 let total_reward = 50.0 * 1e8; 
                                 let fee = 0.0;
                                 let distributable = total_reward - fee;
                                 let mut shares_lock = shares_ref.lock().unwrap();
                                 let total_shares: f64 = shares_lock.iter().map(|s| s.difficulty).sum();
                                 if total_shares > 0.0 {
                                     let reward_per_share = distributable / total_shares;
                                     let mut payouts: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
                                     for s in shares_lock.iter() {
                                         *payouts.entry(s.miner.clone()).or_insert(0.0) += s.difficulty * reward_per_share;
                                     }
                                     // FIX: Iterate by reference to avoid moving 'payouts'
                                     for (_miner, _amount) in &payouts {
                                         // If stale, we might reduce reward? For PPLNS, usually full credit.
                                         // But WE CANNOT SUBMIT STALE BLOCK TO CHAIN.
                                         if is_stale { continue; } 
                                     } 
                                     if is_stale {
                                          println!("[Stratum] Stale Block - Valid PoW but old parent. Submitting as Share only.");
                                     } else {
                                          // Logic continues...
                                          let pool_addr = server_wallet.lock().unwrap().get_address();
                                          let mut current_nonce = chain_lock.state.get_nonce(&pool_addr);
                                     for (miner, amount) in payouts { // Consume here is fine
                                         let amount_u64 = amount as u64;
                                         let base_fee = 100_000;
                                         let percentage_fee = (amount_u64 as f64 * 0.001) as u64;
                                         let tx_fee = base_fee + percentage_fee;
                                         if amount_u64 > tx_fee + 1000 {
                                              current_nonce += 1;
                                              let net_amount = amount_u64 - tx_fee;
                                              let mut tx = crate::transaction::Transaction::new(
                                                  pool_addr.clone(), miner.clone(), net_amount, "VLT".to_string(), current_nonce, tx_fee
                                              );
                                              if let Some(pk) = &server_wallet.lock().unwrap().private_key {
                                                 use k256::ecdsa::signature::Signer;
                                                 let signature: k256::ecdsa::Signature = pk.sign(&tx.get_hash());
                                                 let mut signed_tx = tx.clone();
                                                 signed_tx.signature = hex::encode(signature.to_bytes());
                                                 chain_lock.pending_transactions.push(signed_tx);
                                              }
                                         }
                                     }
                                     shares_lock.clear();
                                 }
                                 } // Close total_shares > 0.0
                                 chain_lock.save();
                                 return Some(serde_json::json!(true));
                             }
                             // If stale, we fall through to Share Accepted
                         } else if is_valid_share {
                            // Valid Share
                            // println!("[Stratum] Share Accepted ...");

                            
                            let mut s_lock = shares_ref.lock().unwrap();
                            if s_lock.len() > 5000 { s_lock.remove(0); } // Prevent Memory Leak
                            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or(std::time::Duration::from_secs(0)).as_secs();
                            
                            s_lock.push(crate::stratum::Share { // Fully qualified just in case
                                miner: session_miner_addr.lock().unwrap().clone(),
                                difficulty: 0.0001, 
                                timestamp: now,
                            });
                            return Some(serde_json::json!(true));
                        } else {
                            // println!("[Stratum] Rejected Share ...");

                            // Return false to let miner know it was rejected? 
                            // Stratum usually expects a bool result for submit.
                            return Some(serde_json::json!(false));
                        }

                }
            }
            Some(serde_json::json!(false))
        },
        _ => None
    }
}

// -------------------------------------------------------------------------
// TCP HANDLER (Legacy)
// -------------------------------------------------------------------------
fn handle_client(
    stream: TcpStream, 
    chain: Arc<Mutex<Blockchain>>, 
    mode_ref: Arc<Mutex<PoolMode>>,
    shares_ref: Arc<Mutex<Vec<Share>>>,
    wallet_ref: Arc<Mutex<Wallet>>,
    job_source: Arc<Mutex<JobState>>
) {
    let peer_addr = stream.peer_addr().unwrap_or(std::net::SocketAddr::from(([0,0,0,0], 0)));
    println!("[Stratum TCP] Client connected: {}", peer_addr);
    
    let stream_reader = match stream.try_clone() { Ok(s) => s, Err(_) => return };
    let mut stream_writer_notify = match stream.try_clone() { Ok(s) => s, Err(_) => return };
    let mut stream_writer_resp = stream;

    let session_miner_addr = Arc::new(Mutex::new("SYSTEM_POOL".to_string()));
    let current_block_template = Arc::new(Mutex::new(None::<crate::block::Block>));
    let prev_block_template = Arc::new(Mutex::new(None::<crate::block::Block>)); // Added
    let is_authorized = Arc::new(Mutex::new(false));
    let last_job_id = Arc::new(Mutex::new("".to_string()));
    let prev_job_id = Arc::new(Mutex::new("".to_string())); // Added
    let last_notified_height = Arc::new(Mutex::new(0u64)); 
    
    // Generate Unique ExtraNonce1 (Random 4 bytes hex)
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_u32: u32 = rng.gen();
    let extra_nonce_1_val = format!("{:08x}", random_u32);
    let extra_nonce_1 = Arc::new(Mutex::new(extra_nonce_1_val)); // Session State 

    // Notifier Thread
    let (block_n, prev_block_n, auth_n, last_job_n, prev_job_n, job_src_n) = (
        current_block_template.clone(), prev_block_template.clone(), is_authorized.clone(), last_job_id.clone(), prev_job_id.clone(), job_source.clone()
    );
     
    
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(200)); 
            if !*auth_n.lock().unwrap() { continue; }
            
            let (new_notify, new_block, new_id) = {
                 let state = job_src_n.lock().unwrap();
                 if state.job_id == "INIT" { continue; }
                 
                 let my_last = last_job_n.lock().unwrap().clone();
                 if state.job_id != my_last {
                     (Some(state.notify_json.clone()), state.block_template.clone(), state.job_id.clone())
                 } else { (None, None, "".to_string()) }
            };

            if let Some(notify) = new_notify {
                // Shift History
                let last = last_job_n.lock().unwrap().clone();
                if !last.is_empty() {
                    *prev_job_n.lock().unwrap() = last;
                    *prev_block_n.lock().unwrap() = block_n.lock().unwrap().clone();
                }

                *block_n.lock().unwrap() = new_block;
                *last_job_n.lock().unwrap() = new_id;
                
                if let Ok(s) = serde_json::to_string(&notify) {
                     if stream_writer_notify.write_all((s + "\n").as_bytes()).is_err() { break; }
                }
            }
        }
    }); 


    let mut reader = BufReader::new(stream_reader);
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 { break; }
        if let Ok(req) = serde_json::from_str::<RpcRequest>(&line) {
            let res = process_rpc_request(req.clone(), &chain, &mode_ref, &shares_ref, &wallet_ref, &session_miner_addr, 
                &current_block_template, &is_authorized, &last_notified_height, &extra_nonce_1, &last_job_id, &prev_job_id, &prev_block_template); // Passed new args
            
            if let Some(val) = res {
                // FIX: Send Explicit Difficulty Notification BEFORE Response
                // (Removed duplicate block - relying on the one AFTER response or merged)
                // Actually, let's keep the one AFTER response to ensure client state is ready.
                
                let resp = RpcResponse { id: req.id, result: Some(val), error: None };
                if let Ok(s) = serde_json::to_string(&resp) {
                    let _ = stream_writer_resp.write_all((s + "\n").as_bytes());
                    let _ = stream_writer_resp.flush();
                }

                // Double check: Send AGAIN after response just in case miner ignored the first one
                if req.method == "mining.subscribe" || req.method == "mining.authorize" {
                     let diff_notify = serde_json::json!({
                        "id": null, "method": "mining.set_difficulty", "params": [0.0001]
                    });
                    if let Ok(s) = serde_json::to_string(&diff_notify) {
                         let _ = stream_writer_resp.write_all((s + "\n").as_bytes());
                         let _ = stream_writer_resp.flush();
                    }
                }
            }
        }
    }
}

// -------------------------------------------------------------------------
// WEBSOCKET HANDLER
// -------------------------------------------------------------------------
fn handle_client_ws(
    mut socket: WebSocket<TcpStream>, 
    chain: Arc<Mutex<Blockchain>>, 
    mode_ref: Arc<Mutex<PoolMode>>,
    shares_ref: Arc<Mutex<Vec<Share>>>,
    wallet_ref: Arc<Mutex<Wallet>>,
    job_source: Arc<Mutex<JobState>> // New Arg
) {
    println!("[Stratum WS] Client connected via WebSocket");
    
    // Set Timeout for Loop
    if let Some(stream) = socket.get_mut().try_clone().ok() {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(100))); // Fast check
    }

    let session_miner_addr = Arc::new(Mutex::new("SYSTEM_POOL".to_string()));
    let current_block_template = Arc::new(Mutex::new(None::<crate::block::Block>));
    let prev_block_template = Arc::new(Mutex::new(None::<crate::block::Block>)); // Added for Stale Support
    let is_authorized = Arc::new(Mutex::new(false));
    let last_job_id = Arc::new(Mutex::new("".to_string()));
    let prev_job_id = Arc::new(Mutex::new("".to_string())); // Added for Stale Support
    let last_notified_height = Arc::new(Mutex::new(0u64));
    
    // Generate Unique ExtraNonce1 (Random 4 bytes hex)
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_u32: u32 = rng.gen();
    let extra_nonce_1_val = format!("{:08x}", random_u32);
    let extra_nonce_1 = Arc::new(Mutex::new(extra_nonce_1_val)); // Session State

    loop {
        // 1. Check Notifications (Inline - Single Threaded Loop)
        if *is_authorized.lock().unwrap() {
            let (new_notify, new_block, new_id) = {
                 let state = job_source.lock().unwrap();
                 if state.job_id != "INIT" {
                     let my_last = last_job_id.lock().unwrap().clone();
                     if state.job_id != my_last {
                        (Some(state.notify_json.clone()), state.block_template.clone(), state.job_id.clone())
                     } else {
                        (None, None, "".to_string())
                     }
                 } else { (None, None, "".to_string()) }
            };

            if let Some(notify) = new_notify {
                // Shift History
                let last = last_job_id.lock().unwrap().clone();
                if !last.is_empty() {
                    *prev_job_id.lock().unwrap() = last;
                    *prev_block_template.lock().unwrap() = current_block_template.lock().unwrap().clone();
                }

                *current_block_template.lock().unwrap() = new_block;
                *last_job_id.lock().unwrap() = new_id;
                
                if let Ok(s) = serde_json::to_string(&notify) {
                     let _ = socket.send(Message::Text(s));
                }
            }
        }

        // 2. Read Message (with timeout)
        match socket.read() {
            Ok(msg) => {
                if msg.is_text() || msg.is_binary() {
                    let text = msg.to_text().unwrap_or("");
                    if let Ok(req) = serde_json::from_str::<RpcRequest>(text) {
                        let res = process_rpc_request(req.clone(), &chain, &mode_ref, &shares_ref, &wallet_ref, &session_miner_addr, 
                            &current_block_template, &is_authorized, &last_notified_height, &extra_nonce_1, &last_job_id, &prev_job_id, &prev_block_template); // Passed new args
                        
                        if let Some(val) = res {
                            let resp = RpcResponse { id: req.id, result: Some(val), error: None };
                            if let Ok(s) = serde_json::to_string(&resp) {
                                let _ = socket.send(Message::Text(s));
                            }
                            // FIX: Send Explicit Difficulty Notification after Subscribe
                            if req.method == "mining.subscribe" {
                                let diff_notify = serde_json::json!({
                                    "id": null, "method": "mining.set_difficulty", "params": [0.0001]
                                });
                                if let Ok(s) = serde_json::to_string(&diff_notify) {
                                     let _ = socket.send(Message::Text(s));
                                }
                            }
                        }
                    }
                } else if msg.is_close() { break; }
            },
            Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                continue; 
            },
            Err(_) => break, // Connection closed
        }
    }
}
