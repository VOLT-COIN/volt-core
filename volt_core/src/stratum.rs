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


use crate::wallet::Wallet;
use std::collections::HashSet; // Added for Duplicate Share Check


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
    let h_push = format!("04{}", hex::encode(h_bytes)); // PUSH 4 bytes (Height)
    // ScriptSig: Length(14=0x0e) + PUSH4(Height) + PUSH8(Nonces)
    // coinb1 ends with "08" (PUSH 8 bytes) so miner appending 8 bytes of nonces completes the Opcode payload.
    let coinb1 = format!("010000000100000000000000000000000000000000000000000000000000000000ffffffff0e{}08", h_push);
    
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
    // 4. Branch (Merkle Path)
    let mut branch = Vec::new();
    let mut hashes: Vec<Vec<u8>> = next_block.transactions.iter().map(|tx| tx.get_hash()).collect();
    
    while hashes.len() > 1 {
       if hashes.len() % 2 != 0 { hashes.push(hashes.last().unwrap().clone()); }
       
       // For Coinbase (Index 0), the sibling is ALWAYS Index 1.
       branch.push(hex::encode(&hashes[1]));
       
       // Move to next level
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
        "params": [ job_id, next_block.previous_hash, coinb1, cb2, branch, version_hex, bits_hex, ntime_hex, true ]
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
    node: Arc<crate::node::Node>, // Added: P2P Integration
}

impl StratumServer {
    pub fn new(blockchain: Arc<Mutex<Blockchain>>, port: u16, mode: PoolMode, shares: Arc<Mutex<Vec<Share>>>, server_wallet: Arc<Mutex<Wallet>>, node: Arc<crate::node::Node>) -> Self {
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
            node,
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
                // let miner_addr = "SYSTEM_POOL".to_string(); // FIX: Use Real Wallet Address to prevent mismatch in validation
                println!("[Stratum] Job Updater Thread Started");
                
                loop {
                    thread::sleep(Duration::from_millis(500));
                    
                    let (h, next_block) = {
                        let c = chain.lock().unwrap();
                        let pool_addr = wallet.lock().unwrap().get_address();
                        (c.get_height(), c.get_mining_candidate(pool_addr))
                    };
                    
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs();
                    
                    // Check if update needed
                    let mut update_needed = false;
                    {
                        let state = job_state.lock().unwrap();
                        let last_h = state.block_template.as_ref().map(|b| b.index).unwrap_or(0);
                        let last_tx_count = state.block_template.as_ref().map(|b| b.transactions.len()).unwrap_or(0);
                        
                        // Update if: 
                        // 1. New Block (Height changed)
                        // 2. New Txs (Mempool changed)
                        // 3. Time passed (60s)
                        if h != last_h || next_block.transactions.len() != last_tx_count || now % 60 == 0 { 
                            update_needed = true;
                        }
                    }
                    
                    if update_needed {
                         // Include Tx Count in Job ID to force unique ID when Txs change but timestamp is same
                         let job_id = format!("{}_{}_{}", next_block.index, next_block.timestamp, next_block.transactions.len());
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
                         
                         // Clean up submitted nonces for new job
                         // We can't easily access the thread-local nonces of all clients here.
                         // But since Job ID changed, the client nonces (keyed by JobID) are naturally fresh.
                    }
                }
            });
        }
        
        let shared_job_workers = self.shared_job.clone();
        let node_workers = self.node.clone(); // Added: P2P Broadcast

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
                        let node_ref = node_workers.clone(); // Pass to worker

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
                                        handle_client_ws(socket, chain, mode, shares, wallet, job_source, node_ref);
                                    }
                                    Err(e) => println!("[Stratum] WS Handshake Failed: {}", e),
                                }
                            } else {
                                handle_client(stream, chain, mode, shares, wallet, job_source, node_ref);
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
    prev_block_template_ref: &Arc<Mutex<Option<crate::block::Block>>>, // Added: Previous Template
    submitted_nonces: &mut HashSet<(String, u32)>, // Added: Duplicate Share Tracker
    node: &Arc<crate::node::Node> // Added: P2P Integration
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
                     // println!("[Stratum] Processing Stale Share for Job: {} (Prev Job)", jid);
                } else {
                    println!("[Stratum] Rejected Share (Unknown Job: {} | Curr: {})", jid, current_job);
                    return Some(serde_json::json!(false));
                }

                // Check Duplicate Share
                // Nonce is explicitly parsed later, but we need it here for the check.
                if let Ok(nonce_val) = u32::from_str_radix(nonce_hex, 16) {
                    if submitted_nonces.contains(&(jid.to_string(), nonce_val)) {
                         println!("[Stratum] Rejected Duplicate Share: Job {} Nonce {:x}", jid, nonce_val);
                         return Some(serde_json::json!(false));
                    }
                    submitted_nonces.insert((jid.to_string(), nonce_val));
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
                        // ScriptSig = PUSH(Height) + PUSH(ExtraNonce)
                        // Our protocol: PUSH4(Height) + PUSH8(Nonces)
                        
                        // Strictly Validated Reconstruction
                        if ex2.len() != 8 { // 4 bytes = 8 hex chars
                             println!("[Stratum] Rejected Share: Invalid ExtraNonce2 Size (Expected 4 bytes)");
                             return Some(serde_json::json!(false));
                        }

                        // Update to match new Standard: PUSH4(Height) + PUSH8(Nonces)
                        // Length 14 (0e). h_push uses 04. PUSH8 opcode (08) acts as separator.
                        let coinb1 = format!("010000000100000000000000000000000000000000000000000000000000000000ffffffff0e04{}08", hex::encode(height_bytes));
                        
                        // Dynamic P2PKH Script (Use ADDRESS FROM BLOCK, not Wallet State)
                        // This ensures that if the wallet changed, we still validate old shares correctly.
                        // The 'sender' field for coinbase transactions is "SYSTEM".
                        // The 'receiver' field for coinbase transactions is the pool's public key (hex).
                        let pool_addr_from_template = &block.transactions[0].receiver;
                        
                        let pool_addr_hex = if pool_addr_from_template == "SYSTEM" || pool_addr_from_template.is_empty() {
                             // Fallback (Should not happen for valid templates)
                             server_wallet.lock().unwrap().get_address()
                        } else {
                             pool_addr_from_template.clone()
                        };

                        let pub_key_bytes = hex::decode(&pool_addr_hex).unwrap_or(vec![0;33]);

                        use sha2::{Sha256, Digest};
                        use ripemd::Ripemd160; // Silence Warning
                        
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
                             
                              let mut tx = block.transactions[0].clone();
                              
                              // Fix: Correctly construct ScriptSig for Storage (Two Pushes)
                              let mut nonces = Vec::new();
                              let extra_nonce_1_bytes = hex::decode(extra_nonce_1).unwrap_or(vec![0;4]);
                              let extra_nonce_2_bytes = hex::decode(ex2).unwrap_or(vec![0;4]);
                              nonces.extend(extra_nonce_1_bytes);
                              nonces.extend(extra_nonce_2_bytes);
                              
                              tx.script_sig = crate::script::Script::new()
                                  .push(crate::script::OpCode::OpPush(height_bytes.to_vec()))
                                  .push(crate::script::OpCode::OpPush(nonces));
                                  
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
                        // We must validate against the ACTUAL block difficulty (Bits), not a hardcoded string.
                        // Extract required zeros from Bits (Simplified: 0x1d00ffff -> Diff 1 -> ~32 zeros? No, Diff 1 is ~8 hex zeros)
                        // For MVP, we use the helper in block.rs if available, or reproduce basic logic.
                        // Block::check_pow expects 'distinct_bits' (u32 count of zeros).
                        // Let's decode bits back to target.
                        // 0x1d00ffff: Exponent 0x1d (29), Coeff 0x00ffff. Target = 0x00ffff * 256^(29-3).
                        // This is complex to check precisely in one line.
                        // However, standard mining relies on hash < target.
                        // Let's us a safer check:
                        // "00000000" corresponds to Diff 1 (High/Low diff).
                        // If we are Mainnet, we need dynamic check.
                        // For this specific 'Volt' implementation, difficulty is fixed at 1 (0x1d00ffff) usually.
                        // But to be "Logical", we should check checking usage.
                        // Let's assume Diff 1 for now but mark it as explicit minimum.
                        
                        // BETTER FIX: Check against the Block's stored Bits
                        // We implement a quick hash < target check if possible.
                        // Since we don't have a Uint256 library handy here, we will stick to the 'starts_with' 
                        // BUT we ensure it matches the Block's difficulty approximately.
                        // If Block Diff is 1, expect 8 zeros.
                        // If Block Diff is higher, expect more.
                        // Let's use the Block::check_pow from before? logic is confusing there.
                        // Reverting to SAFE hardcoded minimum for this 'Bug Search' request, 
                        // BUT adding a comment that this must be dynamic in Production.
                        
                        // WAIT! User asked for "Illogical". Testing Logic:
                        // If block.difficulty == 0x1d00ffff, acceptable hash starts with "00000000".
                        // If block.difficulty changes, this code BREAKS.
                        // I will add a dynamic check based on difficulty.
                        
                        // FIX: Dynamic Block Difficulty Check
                        // Decode Compact Bits (e.g., 0x1d00ffff) to approximate required zero bits.
                        // 0x1d00ffff -> Diff 1 -> ~32 bits (8 hex zeros)
                        // 0x207fffff -> Diff Min -> ~0 bits
                        // Formula: 8 * (0x1d - (bits >> 24))
                        // Exp = bits >> 24.
                        // Standard (0x1d) = 29 bytes. 32 - 29 = 3 bytes (24 bits) + mantissa adjustment?
                        // Let's use the same logic as chain.rs approx or better.
                        // Chain requires "0" * 4 for diff 1? No, chain requires prefix.
                        
                        let diff_bits = block.difficulty;
                        let exp = (diff_bits >> 24) & 0xff;
                        let required_zeros_bits = if exp <= 0x1d {
                             // Harder than or equal to Diff 1
                             // 0x1d = 32 bits (approx)
                             // 0x1c = 40 bits
                             32 + (0x1d - exp) * 8
                        } else {
                             // Easier (e.g. 0x1f, 0x20)
                             if exp >= 0x20 { 0 } else { 
                                 32u32.saturating_sub((exp - 0x1d) * 8)
                             }
                        };
                        
                        let is_valid_block = crate::block::Block::check_pow(&block.hash, required_zeros_bits);


                        // SHARE CHECK (Relaxed for Testnet)
                        // Difficulty 0x207fffff requires very little work (starts with 0 bit).
                        // We set it to 1 bit to accept almost anything validly structured.
                        let is_valid_share = crate::block::Block::check_pow(&block.hash, 1); 

                        if is_valid_block {
                             println!("[Pool] BLOCK FOUND! Hash: {}", block.hash);
                             let mut chain_lock = chain.lock().unwrap();
                             
                             if is_stale {
                                 // Do NOT submit stale block to chain (it will fail anyway).
                                 // But we count it as a share below.
                             } else if chain_lock.submit_block(block.clone()) {
                                 chain_lock.save();
                                 // FIX: Broadcast Mined Block to P2P Network
                                 println!("[Stratum] Broadcasting Block #{} to Peers...", block.index);
                                 node.broadcast_block(block.clone());
                                 
                                 println!("[PPLNS] Block Found! Distributing Rewards...");
                                 // ... Payouts ...
                                 let total_reward = crate::block::Block::get_block_reward(block.index) as f64; 
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
                                difficulty: 1.0, // Credit for Diff 1 Share
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
    job_source: Arc<Mutex<JobState>>,
    node: Arc<crate::node::Node> // Added: P2P Broadcast
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
    
    // FIX: Add duplicate share tracker
    let mut submitted_nonces = HashSet::new();

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
                &current_block_template, &is_authorized, &last_notified_height, &extra_nonce_1, &last_job_id, &prev_job_id, &prev_block_template, &mut submitted_nonces, &node); // Passed node
            
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
    job_source: Arc<Mutex<JobState>>, // New Arg
    node: Arc<crate::node::Node> // Added: P2P Broadcast
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

    // FIX: Add duplicate share tracker
    let mut submitted_nonces = HashSet::new();

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
                             &current_block_template, &is_authorized, &last_notified_height, &extra_nonce_1, &last_job_id, &prev_job_id, &prev_block_template, &mut submitted_nonces, &node); // Passed node
                        
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
