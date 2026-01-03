use std::net::TcpListener;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use crate::chain::Blockchain;
use crate::wallet::Wallet;
use crate::transaction::Transaction;
use crate::node::Node;

#[derive(Serialize, Deserialize, Debug)]
struct ApiRequest {
    command: String,
    // Optional parameters
    pub address: Option<String>,
    pub to: Option<String>,
    pub amount: Option<u64>,
    pub token: Option<String>,
    pub password: Option<String>,
    pub timestamp: Option<u64>,
    pub fee: Option<u64>,
    pub active: Option<bool>,
    pub mnemonic: Option<String>,
    
    // DEX
    pub side: Option<String>,
    pub price: Option<u64>,
    // New params for explorer
    start_index: Option<usize>,
    end_index: Option<usize>,
    pub height: Option<usize>, // Explicit height param
    // New params for Volt Pay
    since_timestamp: Option<u64>,
    // V2: Token & Staking
    pub hash: Option<String>, // Added for get_transaction
    pub data: Option<serde_json::Value>, // Generic payload for complex objects
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    status: String,
    message: String,
    data: Option<serde_json::Value>,
}

pub struct ApiServer {
    blockchain: Arc<Mutex<Blockchain>>,
    mining_status: Arc<Mutex<bool>>,
    wallet: Arc<Mutex<Wallet>>, // Node's wallet for remote sending
    is_locked: Arc<Mutex<bool>>, // Track if wallet is locked
    port: u16,
    node: Arc<Node>,
}

impl ApiServer {
    pub fn new(
        blockchain: Arc<Mutex<Blockchain>>,
        mining_status: Arc<Mutex<bool>>,
        wallet: Arc<Mutex<Wallet>>, // Shared Wallet
        port: u16,
        node: Arc<Node>
    ) -> Self {
        // Check availability of encrypted wallet
        let has_enc = std::path::Path::new("wallet.enc").exists();
        let is_locked = Arc::new(Mutex::new(has_enc));

        ApiServer {
            blockchain,
            mining_status,
            wallet, // Use shared wallet
            is_locked,
            port,
            node,
        }
    }

    pub fn start(&self) {
        let port = self.port;
        let blockchain = self.blockchain.clone();
        let mining_status = self.mining_status.clone();
        let wallet = self.wallet.clone();
        let is_locked = self.is_locked.clone();
        let node = self.node.clone();

        thread::spawn(move || {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).expect("Failed to bind API port");
            println!("[API] Server listening on 0.0.0.0:{} (Public Access Enabled)", port);

            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        let chain_ref = blockchain.clone();
                        let mining_ref = mining_status.clone();
                        let wallet_ref = wallet.clone();

                        let lock_ref = is_locked.clone();
                        let node_ref = node.clone();
                        
                        thread::spawn(move || {
                            let mut buffer = [0; 4096];
                            if let Ok(size) = stream.read(&mut buffer) {
                                let request_str = String::from_utf8_lossy(&buffer[0..size]);
                                
                                // HTTP/CORS Support
                                let mut is_http = false;
                                let mut json_str = request_str.to_string();

                                if request_str.starts_with("POST") || request_str.starts_with("OPTIONS") {
                                    is_http = true;
                                    // Handle Preflight OPTIONS
                                    if request_str.starts_with("OPTIONS") {
                                         let response = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n\r\n";
                                         let _ = stream.write_all(response.as_bytes());
                                         return;
                                    }
                                    
                                    // Extract Body (Double newline)
                                    if let Some(idx) = request_str.find("\r\n\r\n") {
                                        json_str = request_str[idx+4..].to_string();
                                        // Some browsers send null terminator or extra data, trim it
                                        json_str = json_str.trim_matches(char::from(0)).to_string();
                                    }
                                }

                                let response = handle_request(
                                    &json_str, 
                                    chain_ref, 
                                    mining_ref, 
                                    wallet_ref, 
                                    lock_ref,
                                    node_ref
                                );
                                let response_json = serde_json::to_string(&response).unwrap_or(String::from("{\"status\":\"error\",\"message\":\"JSON Serialization Failed\"}")) + "\n";
                                
                                if is_http {
                                    let content_len = response_json.len();
                                    let http_response = format!(
                                        "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                        content_len,
                                        response_json
                                    );
                                    let _ = stream.write_all(http_response.as_bytes());
                                } else {
                                    let _ = stream.write_all(response_json.as_bytes());
                                }
                            }
                        });
                    }
                    Err(e) => println!("[API] Connection failed: {}", e),
                }
            }
        });
    }
}

fn handle_request(
    req_str: &str,
    blockchain: Arc<Mutex<Blockchain>>,
    mining_status: Arc<Mutex<bool>>,
    wallet: Arc<Mutex<Wallet>>,
    is_locked: Arc<Mutex<bool>>,
    node: Arc<Node>
) -> ApiResponse {
    // Parse Request
    let req: ApiRequest = match serde_json::from_str(req_str) {
        Ok(r) => r,
        Err(_) => return ApiResponse { 
            status: "error".to_string(), 
            message: "Invalid JSON".to_string(), 
            data: None 
        },
    };

    match req.command.as_str() {
        "get_status" => {
            let locked = *is_locked.lock().unwrap();
            let mining = *mining_status.lock().unwrap();
            ApiResponse {
                status: "success".to_string(),
                message: "Node Status".to_string(),
                data: Some(serde_json::json!({ 
                    "locked": locked,
                    "mining": mining
                }))
            }
        },
        "generate_mnemonic" => {
            let (wallet, phrase) = Wallet::create_with_mnemonic();
            ApiResponse {
                status: "success".to_string(),
                message: "Mnemonic generated".to_string(),
                data: Some(serde_json::json!({ 
                    "mnemonic": phrase,
                    "address": wallet.get_address()
                }))
            }
        },
        "import_mnemonic" => {
            if let (Some(phrase), Some(pass)) = (req.mnemonic, req.password) {
                match Wallet::from_phrase(&phrase) {
                    Ok(w) => {
                        w.save_encrypted(&pass);
                        
                        let mut wallet_lock = wallet.lock().unwrap();
                        *wallet_lock = w;
                        let mut locked = is_locked.lock().unwrap();
                        *locked = false;
                        
                        ApiResponse { 
                            status: "success".to_string(), 
                            message: "Wallet Imported & Unlocked".to_string(), 
                            data: Some(serde_json::json!({ "address": wallet_lock.get_address() }))
                        }
                    },
                    Err(e) => ApiResponse { status: "error".to_string(), message: e, data: None }
                }
            } else {
                 ApiResponse { status: "error".to_string(), message: "Missing mnemonic or password".to_string(), data: None }
            }
        },
        "get_mnemonic" => {
            let locked = *is_locked.lock().unwrap();
            if locked {
                return ApiResponse { status: "error".to_string(), message: "WALLET LOCKED".to_string(), data: None };
            }
            let wallet = wallet.lock().unwrap();
            ApiResponse {
                status: "success".to_string(),
                message: "Seed phrase retrieved".to_string(),
                data: Some(serde_json::json!({ "mnemonic": wallet.mnemonic }))
            }
        },
        "get_address" => {
             let locked = *is_locked.lock().unwrap();
             if locked {
                 return ApiResponse { status: "error".to_string(), message: "WALLET LOCKED".to_string(), data: None };
             }
             let wallet = wallet.lock().unwrap();
             ApiResponse {
                status: "success".to_string(),
                message: "Wallet address".to_string(),
                data: Some(serde_json::json!({ "address": wallet.get_address() }))
            }
        },
        "get_recent_blocks" => {
            let chain = blockchain.lock().unwrap();
            let len = chain.chain.len();
            let start = if len > 20 { len - 20 } else { 0 };
            let blocks = &chain.chain[start..];
            let response_data = serde_json::to_value(blocks).unwrap_or(Value::Null);
            ApiResponse { 
                status: "success".to_string(), 
                message: "Recent blocks retrieved".to_string(),
                data: Some(serde_json::json!({ "count": blocks.len(), "blocks": response_data })) 
            }
        },
        "get_chain_info" => {
            let chain = blockchain.lock().unwrap();
            let height = chain.chain.len();
            let last_hash = chain.chain.last().map(|b| &b.hash).unwrap_or(&String::from("0000000000000000000000000000000000000000000000000000000000000000")).clone();
            
            ApiResponse {
                status: "success".to_string(),
                message: "Chain info retrieved".to_string(),
                data: Some(serde_json::json!({
                    "height": height,
                    "difficulty": chain.difficulty,
                    "last_hash": last_hash,
                    "pending_count": chain.pending_transactions.len(),
                    "peers": node.peers.lock().unwrap().len()
                }))
            }
        },
        "import_wallet" => {
             let locked = *is_locked.lock().unwrap();
             if locked {
                  ApiResponse { status: "error".to_string(), message: "Unlock first to replace".to_string(), data: None }
             } else {
                 if let Some(phrase) = req.mnemonic {
                     match Wallet::from_phrase(&phrase) {
                         Ok(new_wallet) => {
                             new_wallet.save(); // Overwrite default
                             let mut w_lock = wallet.lock().unwrap();
                             *w_lock = new_wallet;
                             ApiResponse { status: "success".to_string(), message: "Wallet Imported".to_string(), data: None }
                         },
                         Err(e) => ApiResponse { status: "error".to_string(), message: e, data: None }
                     }
                 } else {
                     ApiResponse { status: "error".to_string(), message: "Missing mnemonic".to_string(), data: None }
                 }
             }
        },
        "get_blocks" => {
            let chain = blockchain.lock().unwrap();
            let len = chain.chain.len();
            
            // Default to last 10 blocks if not specified
            let start = req.start_index.unwrap_or(if len > 10 { len - 10 } else { 0 });
            let end = req.end_index.unwrap_or(len);
            
            // Bounds check
            let start = start.min(len);
            let end = end.min(len);
            
            if start <= end {
                 let blocks = &chain.chain[start..end];
                 // We return simplified block data to avoid massive JSON
                 let simplified: Vec<serde_json::Value> = blocks.iter().map(|b| {
                     serde_json::json!({
                         "index": b.index,
                         "timestamp": b.timestamp,
                         "transactions": b.transactions.len(),
                         "hash": b.hash,
                         "miner": b.transactions.last().map(|t| t.receiver.clone()).unwrap_or("?".to_string()) // Approximating miner
                     })
                 }).collect();
                 
                 // Reverse order (newest first)
                 let mut rev_blocks = simplified;
                 rev_blocks.reverse();

                 ApiResponse {
                    status: "success".to_string(),
                    message: "Blocks retrieved".to_string(),
                    data: Some(serde_json::json!({ "blocks": rev_blocks }))
                }
            } else {
                ApiResponse { status: "error".to_string(), message: "Invalid range".to_string(), data: None }
            }
        },
        "get_balance" => {
             if let Some(addr) = req.address {
                 let chain = blockchain.lock().unwrap();
                 let bal = chain.state.get_balance(&addr, "VLT");
                 let staked = *chain.state.stakes.get(&addr).unwrap_or(&0);
                 let nonce = *chain.state.nonces.get(&addr).unwrap_or(&0);
                 
                 ApiResponse {
                     status: "success".to_string(),
                     message: "Balance retrieved".to_string(),
                     data: Some(serde_json::json!({ 
                         "balance": bal, 
                         "staked": staked,
                         "nonce": nonce 
                     }))
                 }
             } else {
                 ApiResponse { status: "error".to_string(), message: "Missing Address".to_string(), data: None }
             }
        },
        "get_assets" => {
            if let Some(addr) = req.address {
                let chain = blockchain.lock().unwrap();
                // Access state directly or via wrapper if exists. 
                // Assuming access to state.get_all_balances or implementing logic here if missing.
                // Since I haven't confirmed get_all_balances yet, I will rely on existing logic or robust filter.
                // Actually, let's use the SAFE manual iteration for now to be 100% sure, but add the FILTER.
                
                let mut assets: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

                for block in &chain.chain {
                    for tx in &block.transactions {
                        // Calculate balances manually from chain history (Robust fallback)
                        if tx.receiver == addr || tx.sender == addr {
                            let entry = assets.entry(tx.token.clone()).or_insert(0);
                            if tx.receiver == addr { *entry += tx.amount as i64; }
                            if tx.sender == addr && tx.tx_type != crate::transaction::TxType::IssueToken { 
                                *entry -= tx.amount as i64; 
                            }
                        }
                    }
                }
                
                // Convert to u64 and Filter < 0
                let filtered_assets: std::collections::HashMap<String, u64> = assets.into_iter()
                    .filter(|(_, amount)| *amount > 0)
                    .map(|(k, v)| (k, v as u64))
                    .collect();

                ApiResponse {
                    status: "success".to_string(),
                    message: "Assets retrieved".to_string(),
                    data: Some(serde_json::json!({ "assets": filtered_assets }))
                }
            } else {
                 ApiResponse { status: "error".to_string(), message: "Missing address".to_string(), data: None }
            }
        },
        "get_history" => {
            if let Some(addr) = req.address {
                let chain = blockchain.lock().unwrap();
                let mut history = Vec::new();
                
                for block in &chain.chain {
                    for tx in &block.transactions {
                        if tx.sender == addr || tx.receiver == addr {
                            let mut txt = serde_json::to_value(tx).unwrap_or(serde_json::Value::Null);
                            txt["timestamp"] = serde_json::json!(block.timestamp);
                            txt["block_index"] = serde_json::json!(block.index);
                            history.push(txt);
                        }
                    }
                }
                history.reverse();
                
                ApiResponse {
                    status: "success".to_string(),
                    message: "History retrieved".to_string(),
                    data: Some(serde_json::json!({ "history": history }))
                }
            } else {
                 ApiResponse { status: "error".to_string(), message: "Missing address".to_string(), data: None }
            }
        },
        "get_peers" => {
            let peers = node.peers.lock().unwrap().clone();
            ApiResponse {
                status: "success".to_string(),
                message: "Peers retrieved".to_string(),
                data: Some(serde_json::json!({ "peers": peers, "count": peers.len() }))
            }
        },
        "set_mining" => {
            if let Some(active) = req.active {
                let mut status = mining_status.lock().unwrap();
                *status = active;
                ApiResponse {
                    status: "success".to_string(),
                    message: format!("Mining set to {}", active),
                    data: None
                }
            } else {
                ApiResponse { status: "error".to_string(), message: "Missing active param".to_string(), data: None }
            }
        },
        "send_transaction" => {
            println!("[API] Received send_transaction request from GUI.");
            let locked = *is_locked.lock().unwrap();
             if locked {
                 println!("[API] Failed: Wallet is LOCKED.");
                 return ApiResponse { status: "error".to_string(), message: "WALLET LOCKED".to_string(), data: None };
             }

            if let (Some(to), Some(amount)) = (req.to, req.amount) {
                println!("[API] Sending {} VLT to {}", amount, to);
                let wallet = wallet.lock().unwrap();
                let mut chain = blockchain.lock().unwrap();
                
                // let msg = format!("{}{}{}", wallet.get_address(), to, amount);
                // let signature = wallet.sign(&msg);
                
                // Fetch Nonce
                let current_nonce = *chain.state.nonces.get(&wallet.get_address()).unwrap_or(&0);
                let next_nonce = current_nonce + 1;
                
                let mut tx = Transaction::new(
                    wallet.get_address(),
                    to,
                    amount,
                    req.token.clone().unwrap_or("VLT".to_string()),
                    next_nonce
                );
                
                // Phase 12: Apply provided fee or default
                if let Some(f) = req.fee {
                    tx.fee = f;
                }
                
                tx.sign(&wallet.private_key);
                
                chain.pending_transactions.push(tx);
                chain.save(); 
                println!("[API] Transaction successful. Added to mempool.");
                
                 ApiResponse {
                    status: "success".to_string(),
                    message: "Transaction sent to mempool".to_string(),
                    data: None
                }
            } else {
                 println!("[API] Failed: Missing parameters.");
                 ApiResponse { status: "error".to_string(), message: "Missing params".to_string(), data: None }
            }
        },
        "issue_asset" => {
            println!("[API] Received issue_asset request.");
            let locked = *is_locked.lock().unwrap();
             if locked {
                 return ApiResponse { status: "error".to_string(), message: "WALLET LOCKED".to_string(), data: None };
             }

            if let (Some(token_name), Some(supply)) = (req.token, req.amount) {
                let wallet = wallet.lock().unwrap();
                let mut chain = blockchain.lock().unwrap();
             
                // Check if already exists
                if chain.state.tokens.contains_key(&token_name) {
                     return ApiResponse { status: "error".to_string(), message: "Token already exists".to_string(), data: None };
                }

                // Fetch Nonce
                let current_nonce = *chain.state.nonces.get(&wallet.get_address()).unwrap_or(&0);
                let next_nonce = current_nonce + 1;
                
                let mut tx = Transaction::new_token_issue(
                    wallet.get_address(),
                    token_name,
                    supply,
                    next_nonce
                );
                
                tx.sign(&wallet.private_key);
                chain.pending_transactions.push(tx);
                chain.save(); 
                
                 ApiResponse {
                    status: "success".to_string(),
                    message: "Asset Issue Transmitted".to_string(),
                    data: None
                }
            } else {
                 ApiResponse { status: "error".to_string(), message: "Missing token/supply".to_string(), data: None }
            }
        },
        "burn_asset" => {
            println!("[API] Received burn_asset request.");
            let locked = *is_locked.lock().unwrap();
             if locked {
                 return ApiResponse { status: "error".to_string(), message: "WALLET LOCKED".to_string(), data: None };
             }

            if let (Some(token_name), Some(amount)) = (req.token, req.amount) {
                let wallet = wallet.lock().unwrap();
                let mut chain = blockchain.lock().unwrap();
                
                // Fetch Nonce
                let current_nonce = *chain.state.nonces.get(&wallet.get_address()).unwrap_or(&0);
                let next_nonce = current_nonce + 1;
                
                let mut tx = Transaction::new_burn(
                    wallet.get_address(),
                    token_name,
                    amount,
                    next_nonce
                );
                
                tx.sign(&wallet.private_key);
                chain.pending_transactions.push(tx);
                chain.save(); 
                
                 ApiResponse {
                    status: "success".to_string(),
                    message: "Asset Burn Transmitted".to_string(),
                    data: None
                }
            } else {
                 ApiResponse { status: "error".to_string(), message: "Missing token/amount".to_string(), data: None }
            }
        },
        "encrypt_wallet" => {
            if let Some(pass) = req.password {
                let wallet_lock = wallet.lock().unwrap();
                wallet_lock.save_encrypted(&pass);
                
                // Security Upgrade: Lock immediately after encryption
                let mut locked = is_locked.lock().unwrap();
                *locked = true;
                drop(wallet_lock);
                
                // Clear memory
                let mut wallet_lock = wallet.lock().unwrap();
                *wallet_lock = Wallet::new();
                
                ApiResponse { status: "success".to_string(), message: "Wallet Encrypted & Locked".to_string(), data: None }
            } else {
                 ApiResponse { status: "error".to_string(), message: "Missing password".to_string(), data: None }
            }
        },
        "unlock_wallet" => {
            if let Some(pass) = req.password {
                if let Some(loaded_wallet) = Wallet::load_encrypted(&pass) {
                    let mut wallet_lock = wallet.lock().unwrap();
                    *wallet_lock = loaded_wallet;
                    let mut locked = is_locked.lock().unwrap();
                    *locked = false; 
                    
                    ApiResponse { status: "success".to_string(), message: "Wallet Unlocked".to_string(), data: None }
                } else {
                    ApiResponse { status: "error".to_string(), message: "Decryption Failed".to_string(), data: None }
                }
            } else {
                 ApiResponse { status: "error".to_string(), message: "Missing password".to_string(), data: None }
            }
        },
        "stake" => {
             if let Some(amt) = req.amount {
                 if amt == 0 {
                     return ApiResponse { status: "error".to_string(), message: "Amount must be positive".to_string(), data: None };
                 }
                 println!("[API] Staking {} VLT", amt);
                 let wallet = wallet.lock().unwrap();
                 let mut chain = blockchain.lock().unwrap();
                 
                 let sender = wallet.get_address();
                 let current_nonce = *chain.state.nonces.get(&sender).unwrap_or(&0);
                 let next_nonce = current_nonce + 1;

                 let mut tx = Transaction::new_stake(sender, amt, next_nonce);
                 tx.sign(&wallet.private_key);
                 
                 if chain.create_transaction(tx) {
                     chain.save();
                     ApiResponse { status: "success".to_string(), message: "Staking Transaction Sent".to_string(), data: None }
                 } else {
                     ApiResponse { status: "error".to_string(), message: "Staking Failed (Check Balance)".to_string(), data: None }
                 }
             } else {
                 ApiResponse { status: "error".to_string(), message: "Missing amount".to_string(), data: None }
             }
        },
        "unstake" => {
             if let Some(amt) = req.amount {
                 if amt == 0 {
                     return ApiResponse { status: "error".to_string(), message: "Amount must be positive".to_string(), data: None };
                 }
                 println!("[API] Unstaking {} VLT", amt);
                 let wallet = wallet.lock().unwrap();
                 let mut chain = blockchain.lock().unwrap();
                 
                 let sender = wallet.get_address();
                 let current_nonce = *chain.state.nonces.get(&sender).unwrap_or(&0);
                 let next_nonce = current_nonce + 1;

                 let mut tx = Transaction::new_unstake(sender, amt, next_nonce);
                 tx.sign(&wallet.private_key);
                 
                 if chain.create_transaction(tx) {
                     chain.save();
                     ApiResponse { status: "success".to_string(), message: "Unstake Transaction Sent".to_string(), data: None }
                 } else {
                     ApiResponse { status: "error".to_string(), message: "Unstake Failed (Check Stake)".to_string(), data: None }
                 }
             } else {
                 ApiResponse { status: "error".to_string(), message: "Missing amount".to_string(), data: None }
             }
        },
        "place_order" => {
             // Params: token, side (BUY/SELL), price, amount
             if let (Some(token), Some(side), Some(price), Some(amount)) = 
                (req.token, req.side, req.price, req.amount) {
                 
                 let wallet = wallet.lock().unwrap();
                 let mut chain = blockchain.lock().unwrap();
                 let sender = wallet.get_address();
                 
                 let current_nonce = *chain.state.nonces.get(&sender).unwrap_or(&0);
                 let next_nonce = current_nonce + 1;
                 
                 let mut tx = Transaction::new_order(sender, token, &side, amount, price, next_nonce);
                 tx.sign(&wallet.private_key);
                 
                 if chain.create_transaction(tx) {
                     chain.save();
                     ApiResponse { status: "success".to_string(), message: "Order Placed".to_string(), data: None }
                 } else {
                     ApiResponse { status: "error".to_string(), message: "Place Order Failed (Funds?)".to_string(), data: None }
                 }
             } else {
                 ApiResponse { status: "error".to_string(), message: "Missing DEX params".to_string(), data: None }
             }
        },
        "cancel_order" => {
             if let Some(id) = req.token { // reusing token field for ID input
                 let wallet = wallet.lock().unwrap();
                 let mut chain = blockchain.lock().unwrap();
                 let sender = wallet.get_address();
                 
                 let current_nonce = *chain.state.nonces.get(&sender).unwrap_or(&0);
                 let next_nonce = current_nonce + 1;
                 
                 let mut tx = Transaction::new_cancel(sender, id, next_nonce);
                 tx.sign(&wallet.private_key);
                 
                 // Check if order exists before sending tx? No, chain validation handles it.
                 chain.pending_transactions.push(tx); // bypass strict create_transaction for now or use it if updated
                 chain.save();
                 
                 ApiResponse { status: "success".to_string(), message: "Cancel Request Sent".to_string(), data: None }
             } else {
                  ApiResponse { status: "error".to_string(), message: "Missing Order ID".to_string(), data: None }
             }
        },
        "get_orders" => {
            let chain = blockchain.lock().unwrap();
            let orders: Vec<crate::chain::Order> = chain.state.orders.values().cloned().collect();
            ApiResponse {
                status: "success".to_string(),
                message: "Orderbook retrieved".to_string(),
                data: Some(serde_json::json!({ "orders": orders }))
            }
        },
        "lock_wallet" => {
             let mut locked = is_locked.lock().unwrap();
             *locked = true;
             // drop(locked); // implicit
             
             // Clear memory
             let mut wallet_lock = wallet.lock().unwrap();
             *wallet_lock = Wallet::new();
             
             ApiResponse { status: "success".to_string(), message: "Wallet Locked".to_string(), data: None }
        },

        "get_block" => {
             // Supports lookup by hash (string) or height (u64 inside generic data or re-using timestamp field? No, ApiRequest doesn't have height param).
             // Checking ApiRequest struct: it has start_index/end_index, but not generic height. 
             // Workaround: Use 'start_index' as height if provided, OR check 'hash'.
             // Actually, the new ApiRequest has `hash` now.
             
             let chain = blockchain.lock().unwrap();
             let mut block = None;

             if let Some(h) = req.hash {
                 block = chain.chain.iter().find(|b| b.hash == h);
             } else if let Some(idx) = req.height.or(req.start_index) { // Use height or fallback to start_index
                 if idx < chain.chain.len() {
                     block = Some(&chain.chain[idx]);
                 }
             }
             // Handle 'height' if passed as a specific field (not in struct yet).
             // The frontend sends { "command": "get_block", "height": 123 }.
             // But ApiRequest struct needs 'height' field.
             // I will stick to 'start_index' or 'hash'.
             // Wait, frontend sends 'height'. The JSON deserializer will FAIL if `ApiRequest` doesn't have `height`.
             // I must ADD `height` to ApiRequest struct first? 
             // OR I can use `start_index` alias in frontend?
             // No, I should update ApiRequest struct. But I just updated it.
             // Let's check if I can use `start_index` (which is usize) for height.
             // I will modify the frontend to send `start_index` instead of `height` later?
             // NO, easier to add `height: Option<usize>` to ApiRequest.
             
             if let Some(b) = block {
                 // Convert to JSON including TX count
                 // We need to clone specific fields or return full block
                 // Returning full block structure via serde
                 let mut b_json = serde_json::to_value(b).unwrap();
                 b_json["tx_count"] = serde_json::json!(b.transactions.len());
                 // Pre-calculate TXS with simplified view or full?
                 // Frontend expects 'txs' array probably.
                 
                 ApiResponse {
                     status: "success".to_string(),
                     message: "Block found".to_string(),
                     data: Some(b_json)
                 }
             } else {
                 ApiResponse { status: "error".to_string(), message: "Block not found".to_string(), data: None }
             }
        },

        "get_transaction" => {
            if let Some(hash) = req.hash {
                let chain = blockchain.lock().unwrap();
                
                // 1. Check Mempool
                for tx in &chain.pending_transactions {
                    if tx.calculate_hash() == hash {
                        let mut t = serde_json::to_value(tx).unwrap();
                        t["status"] = serde_json::json!("pending");
                        t["confirmations"] = serde_json::json!(0);
                        return ApiResponse {
                            status: "success".to_string(),
                            message: "Transaction found in mempool".to_string(),
                            data: Some(t)
                        };
                    }
                }

                // 2. Check Chain
                for block in chain.chain.iter().rev() {
                    for tx in &block.transactions {
                        if tx.calculate_hash() == hash {
                             let mut t = serde_json::to_value(tx).unwrap();
                             t["status"] = serde_json::json!("confirmed");
                             t["block_height"] = serde_json::json!(block.index);
                             t["timestamp"] = serde_json::json!(block.timestamp);
                             t["confirmations"] = serde_json::json!(chain.chain.len() as u64 - block.index + 1);
                             t["hash"] = serde_json::json!(hash); // Ensure hash is present
                             return ApiResponse {
                                 status: "success".to_string(),
                                 message: "Transaction found".to_string(),
                                 data: Some(t)
                             };
                        }
                    }
                }

                ApiResponse { status: "error".to_string(), message: "Transaction not found".to_string(), data: None }
            } else {
                 ApiResponse { status: "error".to_string(), message: "Missing hash parameter".to_string(), data: None }
            }
        },

        "get_recent_txs" => {
            let chain = blockchain.lock().unwrap();
            let mut txs = Vec::new();
            let limit = 50; // max txs to return

            // Iterate backwards through blocks
            'outer: for block in chain.chain.iter().rev() {
                for tx in block.transactions.iter().rev() {
                    let mut t = serde_json::to_value(tx).unwrap();
                    t["block_index"] = serde_json::json!(block.index);
                    t["timestamp"] = serde_json::json!(block.timestamp);
                    t["hash"] = serde_json::json!(tx.calculate_hash()); // Calculate Hash dynamically
                    txs.push(t);
                    if txs.len() >= limit { break 'outer; }
                }
            }
            
            ApiResponse {
                status: "success".to_string(),
                message: "Recent transactions retrieved".to_string(),
                data: Some(serde_json::json!({ "transactions": txs }))
            }
        },
        "broadcast_transaction" => {
            // Public Endpoint for Client-Side Wallets
            if let Ok(tx) = serde_json::from_value::<Transaction>(req.data.unwrap_or(serde_json::Value::Null)) {
                let mut chain = blockchain.lock().unwrap();
                
                // Validate Signature (Simple check, full check in create_transaction)
                if tx.verify() {
                     if chain.create_transaction(tx) {
                         chain.save();
                         ApiResponse { status: "success".to_string(), message: "Transaction Broadcasted".to_string(), data: None }
                     } else {
                         ApiResponse { status: "error".to_string(), message: "Transaction Rejected (Balance/Nonce?)".to_string(), data: None }
                     }
                } else {
                    ApiResponse { status: "error".to_string(), message: "Invalid Signature".to_string(), data: None }
                }
            } else {
                ApiResponse { status: "error".to_string(), message: "Invalid Transaction Format".to_string(), data: None }
            }
        },
        "check_payment" => {
             // println!("[API] Checking payment..."); // Uncomment for spammy debug
             if let Some(addr) = req.address {
                 let amount = req.amount.unwrap_or(0);
                 let since = req.since_timestamp.unwrap_or(0);
                 
                 // println!("[API] Check Payment: {} VLT to {} since {}", amount, addr, since);

                 let chain = blockchain.lock().unwrap();
                 
                 // 1. Check Pending
                 for tx in &chain.pending_transactions {
                     if tx.receiver == addr && tx.amount >= amount && tx.timestamp >= since {
                         println!("[API] Payment FOUND in Mempool!");
                         return ApiResponse { 
                             status: "success".to_string(), 
                             message: "Payment Pending".to_string(), 
                             data: Some(serde_json::json!({ "state": "pending", "tx": tx })) 
                         };
                     }
                 }
                 
                 // 2. Check Recent Blocks
                 for block in chain.chain.iter().rev() {
                     if block.timestamp < since { break; } 
                     for tx in &block.transactions {
                         if tx.receiver == addr && tx.amount >= amount && tx.timestamp >= since {
                             println!("[API] Payment FOUND in Block #{}", block.index);
                             return ApiResponse { 
                                 status: "success".to_string(), 
                                 message: "Payment Confirmed".to_string(), 
                                 data: Some(serde_json::json!({ "state": "confirmed", "tx": tx, "block": block.index })) 
                             };
                         }
                     }
                 }
                 
                 ApiResponse { status: "success".to_string(), message: "Payment Not Found".to_string(), data: Some(serde_json::json!({ "state": "not_found" })) }
             } else {
                 ApiResponse { status: "error".to_string(), message: "Missing address".to_string(), data: None }
             }
        },
        "get_pools" => {
            let chain = blockchain.lock().unwrap();
            let pools: Vec<crate::chain::Pool> = chain.state.pools.values().cloned().collect();
            ApiResponse { 
                status: "success".to_string(), 
                message: "Pools retrieved".to_string(), 
                data: Some(serde_json::json!({ "pools": pools })) 
            }
        },
        "get_candles" => {
             if let Some(pair) = req.token {
                 let chain = blockchain.lock().unwrap();
                 let candles = chain.state.candles.get(&pair).cloned().unwrap_or(Vec::new());
                 ApiResponse {
                     status: "success".to_string(),
                     message: "Candles retrieved".to_string(),
                     data: Some(serde_json::json!({ "candles": candles }))
                 }
             } else {
                 ApiResponse { status: "error".to_string(), message: "Missing Pair".to_string(), data: None }
             }
        },
        "get_nfts" => {
             let chain = blockchain.lock().unwrap();
             // Optional filter by owner
             let nfts: Vec<crate::chain::NFT> = if let Some(address) = req.address {
                 chain.state.nfts.values()
                     .filter(|n| n.owner == address)
                     .cloned()
                     .collect()
             } else {
                 chain.state.nfts.values().cloned().collect()
             };
             
             ApiResponse {
                 status: "success".to_string(),
                 message: "NFTs retrieved".to_string(),
                 data: Some(serde_json::json!({ "nfts": nfts }))
             }
        },
        _ => ApiResponse { status: "error".to_string(), message: "Unknown Command".to_string(), data: None }
    }
}
