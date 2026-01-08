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

pub struct StratumServer {
    blockchain: Arc<Mutex<Blockchain>>,
    port: u16,
    pool_mode: Arc<Mutex<PoolMode>>,
    shares: Arc<Mutex<Vec<Share>>>,
}

impl StratumServer {
    pub fn new(blockchain: Arc<Mutex<Blockchain>>, port: u16, mode: PoolMode, shares: Arc<Mutex<Vec<Share>>>) -> Self {
        StratumServer { 
            blockchain, 
            port,
            pool_mode: Arc::new(Mutex::new(mode)),
            shares,
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

                        thread::spawn(move || {
                            // Protocol Detection
                            let mut buffer = [0; 4];
                            let is_websocket = if stream.peek(&mut buffer).is_ok() {
                                buffer.starts_with(b"GET ")
                            } else { false };

                            if is_websocket {
                                match tungstenite::accept(stream) {
                                    Ok(socket) => {
                                        handle_client_ws(socket, chain, mode, shares);
                                    }
                                    Err(e) => println!("[Stratum] WS Handshake Failed: {}", e),
                                }
                            } else {
                                handle_client(stream, chain, mode, shares);
                            }
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
    session_miner_addr: &Arc<Mutex<String>>,
    current_block_template: &Arc<Mutex<Option<crate::block::Block>>>,
    is_authorized: &Arc<Mutex<bool>>,
    last_notified_height: &Arc<Mutex<u64>>
) -> Option<serde_json::Value> {
    
    match req.method.as_str() {
        "mining.subscribe" => {
            Some(serde_json::json!([
                [ ["mining.set_difficulty", "0.1"], ["mining.notify", "1"] ],
                "00000000", 4
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
                let template_guard = current_block_template.lock().unwrap();
                if let Some(block_template) = template_guard.as_ref() {
                    if jid.starts_with(&block_template.index.to_string()) {
                        let mut block = block_template.clone();
                        
                        // Reconstruct Block
                        let reward_amt = block.transactions[0].amount;
                        let amt_hex = hex::encode(reward_amt.to_le_bytes());
                        let height_bytes = (block.index as u32).to_le_bytes();
                        // Fix: Use 0c (PUSH 12) matches the script constructed below (Height + Extra1 + Ex2 = 12 bytes)
                        let height_push = format!("0c{}", hex::encode(height_bytes));
                        let coinb1 = format!("010000000100000000000000000000000000000000000000000000000000000000ffffffff0d{}", height_push);
                        
                        // Dynamic P2PKH Script
                        let miner_addr_hex = session_miner_addr.lock().unwrap().clone();
                        let pub_key_bytes = hex::decode(&miner_addr_hex).unwrap_or(vec![0;33]);
                        
                        use sha2::Digest;
                        use ripemd::Ripemd160;
                        
                        let mut sha = sha2::Sha256::new();
                        sha.update(&pub_key_bytes);
                        let sha_hash = sha.finalize();
                        
                        let mut rip = Ripemd160::new();
                        rip.update(&sha_hash);
                        let pub_key_hash = rip.finalize();
                        let pub_key_hash_hex = hex::encode(pub_key_hash);
                        
                        let coinb2 = format!("ffffffff01{}1976a914{}88ac00000000", amt_hex, pub_key_hash_hex);
                        let extra_nonce_1 = "00000000";
                        let coinb = format!("{}{}{}{}", coinb1, extra_nonce_1, ex2, coinb2);
                        
                        if let Ok(coinbase_bytes) = hex::decode(&coinb) {
                             use sha2::{Sha256, Digest};
                             let mut hasher = Sha256::new(); hasher.update(&coinbase_bytes);
                             let r1 = hasher.finalize();
                             let mut h2 = Sha256::new(); h2.update(r1);
                             
                             let mut script_data = Vec::new();
                             script_data.extend_from_slice(&height_bytes);
                             script_data.extend_from_slice(&[0,0,0,0]);
                             if let Ok(ex2_bytes) = hex::decode(ex2) { script_data.extend(ex2_bytes); }
                             
                             let mut tx = block.transactions[0].clone();
                             tx.script_sig = crate::script::Script::new().push(crate::script::OpCode::OpPush(script_data));
                             block.transactions[0] = tx;
                             block.merkle_root = crate::block::Block::calculate_merkle_root(&block.transactions);
                        }

                        if let Ok(n) = u32::from_str_radix(nonce_hex, 16) { block.proof_of_work = n.swap_bytes(); }
                        if let Ok(t) = u32::from_str_radix(ntime_hex, 16) { block.timestamp = t as u64; }
                        block.hash = block.calculate_hash();

                        if block.hash.starts_with("0000") {
                            println!("[Pool] BLOCK FOUND! Hash: {}", block.hash);
                            // Submit
                            let mut chain_lock = chain.lock().unwrap();
                            if chain_lock.submit_block(block.clone()) {
                                chain_lock.save();
                                // Payout Logic (Simplified inline call or just logging)
                                // Note: Full PPLNS logic omitted for brevity in shared func, 
                                // but the block is submitted!
                                return Some(serde_json::json!(true));
                            }
                        } else {
                            // Valid Share Logic
                            // SECURITY FIX: Verify Share Weak-PoW (Fake Share Attack)
                            // Even if it's not a block, it must be hard enough to be a share (diff 0.1 ~ 1/10th of 1)
                            // 0.1 difficulty roughly means it needs SOME leading zeros or small value.
                            // For simplicity in this fix, we define Share Target equivalent to "at least 1 zero byte" or small logic.
                            // But cleaner: Use the `check_pow` we just added to Block or simplistic check locally.
                            
                            // Let's reuse Block's check. 
                            // Diff 1 = 32 bits (4 bytes) zero? No, Diff 1 is arbitrary.
                            // Let's assume Share Difficulty (0.1) implies at least 20 bits zero (easier than block 24).
                            // A simple sturdy check: Hash must start with "00" (1 byte).
                            // This prevents submitting purely random hashes (1/256 chance vs 1).
                            
                            if block.hash.starts_with("00") {
                                {
                                    let mut s_lock = shares_ref.lock().unwrap();
                                    if s_lock.len() > 5000 { s_lock.remove(0); }
                                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs();
                                    s_lock.push(Share {
                                        miner: session_miner_addr.lock().unwrap().clone(),
                                        difficulty: 0.1,
                                        timestamp: now,
                                    });
                                }
                                return Some(serde_json::json!(true));
                            } else {
                                println!("[Stratum] Rejected Low-Diff Share from {}", session_miner_addr.lock().unwrap());
                                return Some(serde_json::json!(false));
                            }
                        }
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
    shares_ref: Arc<Mutex<Vec<Share>>>
) {
    let peer_addr = stream.peer_addr().unwrap_or(std::net::SocketAddr::from(([0,0,0,0], 0)));
    println!("[Stratum TCP] Client connected: {}", peer_addr);
    
    let stream_reader = match stream.try_clone() { Ok(s) => s, Err(_) => return };
    let mut stream_writer_notify = match stream.try_clone() { Ok(s) => s, Err(_) => return };
    let mut stream_writer_resp = stream;

    let session_miner_addr = Arc::new(Mutex::new("SYSTEM_POOL".to_string()));
    let current_block_template = Arc::new(Mutex::new(None::<crate::block::Block>));
    let is_authorized = Arc::new(Mutex::new(false));
    let last_notified_height = Arc::new(Mutex::new(0u64));

    // Notifier Thread
    let (chain_n, miner_n, block_n, auth_n, height_n) = (chain.clone(), session_miner_addr.clone(), current_block_template.clone(), is_authorized.clone(), last_notified_height.clone());
    
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(500));
            if !*auth_n.lock().unwrap() { continue; }
            
            // Check Height/Time
            let (h, next_block) = {
                let c = chain_n.lock().unwrap();
                (c.get_height(), c.get_mining_candidate(miner_n.lock().unwrap().clone()))
            };
            let last_h = *height_n.lock().unwrap();
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs();

            if h != last_h || now % 30 == 0 {
                // Generate Notify JSON (Simplified)
                *block_n.lock().unwrap() = Some(next_block.clone());
                *height_n.lock().unwrap() = h;

                let job_id = format!("{}_{}", next_block.index, next_block.timestamp);
                let _notify = serde_json::json!({
                    "id": null, "method": "mining.notify",
                    "params": [ job_id, "PREVHASH", "COINB1", "COINB2", [], "00000001", "BITS", "TIME", true ]
                });
                // Actual generation is complex (see original), sending simplified placeholder to keep it compiled
                // You should assume full generation logic is here or extracted. 
                // For now, let's assume client handles it or this is sufficient for 'ping'.
                // If you need full logic, copy from previous view. 
                // CRITICAL: We need REAL data for mining to work.
                // Re-implementing simplified logic for robustness:
                let prev = hex::decode(&next_block.previous_hash).unwrap_or(vec![0;32]);
                let mut p_rev = prev; p_rev.reverse();
                let prev_hex = hex::encode(p_rev);
                 
                let reward = next_block.transactions[0].amount;
                let amt_hex = hex::encode(reward.to_le_bytes());
                let h_bytes = (next_block.index as u32).to_le_bytes();
                let h_push = format!("04{}", hex::encode(h_bytes));
                // Dynamic P2PKH Script Generation
                let miner_addr_hex = miner_n.lock().unwrap().clone();
                let pub_key_bytes = hex::decode(&miner_addr_hex).unwrap_or(vec![0;33]); // Default to 0 if invalid
                
                // HASH160(PubKey) = RIPEMD160(SHA256(PubKey))
                use sha2::Digest;
                use ripemd::Ripemd160;
                
                let mut sha = sha2::Sha256::new();
                sha.update(&pub_key_bytes);
                let sha_hash = sha.finalize();
                
                let mut rip = Ripemd160::new();
                rip.update(&sha_hash);
                let pub_key_hash = rip.finalize();
                let pub_key_hash_hex = hex::encode(pub_key_hash);

                let cb1 = format!("010000000100000000000000000000000000000000000000000000000000000000ffffffff0d{}", h_push);
                // Script: OP_DUP (76) OP_HASH160 (a9) OP_PUSH20 (14) <HASH> OP_EQUALVERIFY (88) OP_CHECKSIG (ac)
                // P2PKH Length = 25 bytes (1 + 1 + 1 + 20 + 1 + 1) -> 0x19 (25)
                let cb2 = format!("ffffffff01{}1976a914{}88ac00000000", amt_hex, pub_key_hash_hex);
                let bits = format!("{:08x}", next_block.difficulty);
                let ntime = format!("{:08x}", next_block.timestamp);
                
                // Branch
                let mut branch = Vec::new();
                let mut hashes: Vec<Vec<u8>> = next_block.transactions.iter().map(|tx| tx.get_hash()).collect();
                if hashes.len() > 1 {
                   if hashes.len() % 2 != 0 { hashes.push(hashes.last().unwrap().clone()); }
                   if hashes.len() > 1 { branch.push(hex::encode(&hashes[1])); } // Simplified branch for 1 tx + coinbase
                }

                let final_notify = serde_json::json!({
                    "id": null, "method": "mining.notify",
                    "params": [ job_id, prev_hex, cb1, cb2, branch, "00000001", bits, ntime, true ]
                });

                if let Ok(s) = serde_json::to_string(&final_notify) {
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
            let res = process_rpc_request(req.clone(), &chain, &mode_ref, &shares_ref, &session_miner_addr, &current_block_template, &is_authorized, &last_notified_height);
            
            if let Some(val) = res {
                let resp = RpcResponse { id: req.id, result: Some(val), error: None };
                if let Ok(s) = serde_json::to_string(&resp) {
                    let _ = stream_writer_resp.write_all((s + "\n").as_bytes());
                }
            }
        }
    }
}

// -------------------------------------------------------------------------
// WEBSOCKET HANDLER (New)
// -------------------------------------------------------------------------
fn handle_client_ws(
    mut socket: WebSocket<TcpStream>, 
    chain: Arc<Mutex<Blockchain>>, 
    mode_ref: Arc<Mutex<PoolMode>>,
    shares_ref: Arc<Mutex<Vec<Share>>>
) {
    println!("[Stratum WS] Client connected via WebSocket");
    
    // Set Timeout for Loop
    if let Some(stream) = socket.get_mut().try_clone().ok() {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    }

    let session_miner_addr = Arc::new(Mutex::new("SYSTEM_POOL".to_string()));
    let current_block_template = Arc::new(Mutex::new(None::<crate::block::Block>));
    let is_authorized = Arc::new(Mutex::new(false));
    let last_notified_height = Arc::new(Mutex::new(0u64));

    loop {
        // 1. Check Notifications (Inline)
        if *is_authorized.lock().unwrap() {
             let (h, next_block) = {
                let c = chain.lock().unwrap();
                (c.get_height(), c.get_mining_candidate(session_miner_addr.lock().unwrap().clone()))
            };
            let last_h = *last_notified_height.lock().unwrap();
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs();

            if h != last_h || now % 30 == 0 {
                *current_block_template.lock().unwrap() = Some(next_block.clone());
                *last_notified_height.lock().unwrap() = h;
                
                // Gen and Send Logic (Duplicated from TCP for now, ideally extract)
                let job_id = format!("{}_{}", next_block.index, next_block.timestamp);
                
                let prev = hex::decode(&next_block.previous_hash).unwrap_or(vec![0;32]);
                let mut p_rev = prev; p_rev.reverse();
                let prev_hex = hex::encode(p_rev);
                 
                let reward = next_block.transactions[0].amount;
                let amt_hex = hex::encode(reward.to_le_bytes());
                let _h_bytes = (next_block.index as u32).to_le_bytes();
                let h_bytes = (next_block.index as u32).to_le_bytes();
                // Fix: Use 0c (PUSH 12)
                let h_push = format!("0c{}", hex::encode(h_bytes));
                let cb1 = format!("010000000100000000000000000000000000000000000000000000000000000000ffffffff0d{}", h_push);
                
                // Dynamic P2PKH for WS
                let miner_addr_hex = session_miner_addr.lock().unwrap().clone();
                let pub_key_bytes = hex::decode(&miner_addr_hex).unwrap_or(vec![0;33]);
                
                let mut sha = sha2::Sha256::new();
                sha.update(&pub_key_bytes);
                let sha_hash = sha.finalize();
                
                let mut rip = Ripemd160::new();
                rip.update(&sha_hash);
                let pub_key_hash = rip.finalize();
                let pub_key_hash_hex = hex::encode(pub_key_hash);

                let cb2 = format!("ffffffff01{}1976a914{}88ac00000000", amt_hex, pub_key_hash_hex);
                let bits = format!("{:08x}", next_block.difficulty);
                let ntime = format!("{:08x}", next_block.timestamp);
                
                let mut branch = Vec::new();
                let mut hashes: Vec<Vec<u8>> = next_block.transactions.iter().map(|tx| tx.get_hash()).collect();
                if hashes.len() > 1 {
                   if hashes.len() % 2 != 0 { hashes.push(hashes.last().unwrap().clone()); }
                   if hashes.len() > 1 { branch.push(hex::encode(&hashes[1])); }
                }

                let notify = serde_json::json!({
                    "id": null, "method": "mining.notify",
                    "params": [ job_id, prev_hex, cb1, cb2, branch, "00000001", bits, ntime, true ]
                });

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
                        let res = process_rpc_request(req.clone(), &chain, &mode_ref, &shares_ref, &session_miner_addr, &current_block_template, &is_authorized, &last_notified_height);
                        
                        if let Some(val) = res {
                            let resp = RpcResponse { id: req.id, result: Some(val), error: None };
                            if let Ok(s) = serde_json::to_string(&resp) {
                                let _ = socket.send(Message::Text(s));
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
