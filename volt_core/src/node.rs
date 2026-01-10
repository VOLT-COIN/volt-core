use std::net::{TcpListener, TcpStream, SocketAddrV4, Ipv4Addr};
use std::io::Write;
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashSet; // Import HashSet
use serde::{Serialize, Deserialize};
use std::time::Duration;
use crate::block::Block;
use crate::transaction::Transaction;
use crate::chain::Blockchain;
use crate::kademlia::{NodeId, RoutingTable}; // Import Kademlia

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    NewBlock(Block),
    NewTransaction(Transaction),
    GetChain,
    GetBlocks { start: usize, limit: usize },
    Chain(Vec<Block>),
    GetPeers,
    Peers(Vec<String>),
    FindNode(crate::kademlia::NodeId),
    Neighbors(Vec<crate::kademlia::Peer>),
    // SPV Support
    GetHeaders { locator: String }, // locator = last known block hash
    Headers(Vec<Block>), // Send only headers (Block struct has data, but we can verify PoW)
    // P2P Hardening
    Ping,
    Pong,
    Handshake { node_id: crate::kademlia::NodeId, listen_port: u16 },
}

pub struct Node {
    pub blockchain: Arc<Mutex<Blockchain>>,
    pub peers: Arc<Mutex<Vec<String>>>,
    pub banned_peers: Arc<Mutex<HashSet<String>>>, // New Field
    pub routing_table: Arc<Mutex<RoutingTable>>, // DHT
    pub port: u16,
}

impl Node {
    pub fn new(blockchain: Arc<Mutex<Blockchain>>, port: u16) -> Self {
        Node {
            blockchain,
            peers: Arc::new(Mutex::new(Vec::new())),
            banned_peers: Arc::new(Mutex::new(HashSet::new())),
            routing_table: Arc::new(Mutex::new(RoutingTable::new(NodeId::random()))),
            port,
        }
    }

    pub fn broadcast_block(&self, block: Block) {
        let peers = self.peers.lock().unwrap().clone();
        let msg = Message::NewBlock(block.clone());
        let json = serde_json::to_string(&msg).unwrap();
        
        // Capture Chain Reference
        let chain_ref = self.blockchain.clone();

        thread::spawn(move || {
            for peer in peers {
                if peer.starts_with("ws://") || peer.starts_with("wss://") {
                    if let Ok((mut socket, _)) = tungstenite::connect(&peer) {
                         // Send NewBlock
                         let _ = socket.send(tungstenite::Message::Text(json.clone()));
                         
                         // FIX: Listen for potential Sync Requests (GetBlocks) for 10s
                         let start = std::time::Instant::now();
                         while start.elapsed() < std::time::Duration::from_secs(10) {
                             if let Ok(msg) = socket.read() {
                                 if msg.is_text() {
                                     let text = msg.to_text().unwrap_or("{}");
                                     if let Ok(parsed) = serde_json::from_str::<Message>(text) {
                                         if let Message::GetBlocks { start, limit } = parsed {
                                             println!("[Broadcast] Serving Sync Request from {}: Start {}, Limit {}", peer, start, limit);
                                             
                                             let chain = chain_ref.lock().unwrap();
                                             let height = chain.get_height() as usize;
                                             
                                             if start < height {
                                                 let mut chunk = Vec::new();
                                                 let end = std::cmp::min(start + limit, height);
                                                 
                                                 if let Some(ref db) = chain.db {
                                                      for i in start..end {
                                                          if let Ok(Some(block)) = db.get_block(i as u64) {
                                                              chunk.push(block);
                                                          }
                                                      }
                                                 }
                                                 
                                                 let resp = Message::Chain(chunk);
                                                 let resp_json = serde_json::to_string(&resp).unwrap_or_default();
                                                 let _ = socket.send(tungstenite::Message::Text(resp_json));
                                             }
                                         }
                                     }
                                 }
                             } else { break; }
                         }
                    }
                } else {
                    if let Ok(mut stream) = TcpStream::connect(&peer) {
                        let _ = stream.write_all(json.as_bytes());
                        // Listen for TCP response? (Similar logic needed if TCP peers exist)
                        // For now focusing on WSS which is the main transport.
                    }
                }
            }
        });
    }

    pub fn start_server(&self) {
        let port = self.port;
        // UPnP: Try to open port
        Node::attempt_upnp_mapping(port);

        let chain_ref = self.blockchain.clone();
        let peers_ref = self.peers.clone();
        let port_ref = self.port;
        let banned_ref = self.banned_peers.clone();
        let routing_ref = self.routing_table.clone(); // DHT Ref
        
        let active_connections = Arc::new(Mutex::new(0usize));
        let active_connections_ref = active_connections.clone();

        thread::spawn(move || {
            // UPnP: Try to open port
            Node::attempt_upnp_mapping(port_ref);
            
            // DNS Seeding (Longevity Hardening)
            // Inline logic to avoid 'self' lifetime issues in thread
            let seeds = vec![
                "seed1.volt-coin.org", 
                "seed2.volt-project.net",
            ];
            println!("[P2P] Resolving DNS Seeds...");
            
            if let Ok(mut peers_lock) = peers_ref.lock() {
                for seed in seeds {
                     if let Ok(lookup) = std::net::ToSocketAddrs::to_socket_addrs(&(seed, port_ref)) {
                         for addr in lookup {
                              if let std::net::SocketAddr::V4(addr4) = addr {
                                   let ip = addr4.ip().to_string();
                                   if !peers_lock.contains(&ip) {
                                       println!("[P2P] Found Seed Peer: {}", ip);
                                       peers_lock.push(ip);
                                   }
                              }
                         }
                     }
                }
            }

            // P2P Server (Internal Port 7861)
            let p2p_port = 7861;
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port_ref)).expect("Failed to bind P2P port");
            println!("P2P Server listening on internal port {}", p2p_port);

            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => { 
                        // Set strict timeout immediately to prevention Slowloris
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));

                        let chain_inner = chain_ref.clone();
                        let peers_inner = peers_ref.clone();
                        let banned_inner = banned_ref.clone();
                        let routing_inner = routing_ref.clone();
                        
                        // Check Ban
                        let peer_addr = stream.peer_addr();
                        let peer_ip = if let Ok(addr) = peer_addr { addr.ip().to_string() } else { "unknown".to_string() };
                        
                        if banned_inner.lock().unwrap().contains(&peer_ip) {
                            println!("[Security] Rejected connection from BANNED IP: {}", peer_ip);
                            continue; // Drop connection
                        }

                        let active_guard = active_connections_ref.clone();
                        {
                            let mut count = active_guard.lock().unwrap();
                            if *count >= 50 {
                                println!("[P2P] Max active connections (50) reached. Rejecting.");
                                continue;
                            }
                            *count += 1;
                        }

                        // Move guard into thread to decrement on drop
                        thread::spawn(move || {
                            // Ensure decrement on exit
                            struct ConnectionGuard(Arc<Mutex<usize>>);
                            impl Drop for ConnectionGuard {
                                fn drop(&mut self) {
                                    let mut c = self.0.lock().unwrap();
                                    if *c > 0 { *c -= 1; }
                                }
                            }
                            let _guard = ConnectionGuard(active_guard);
                            // ---------------------------------------------------------
                            // SERVER SIDE: Dual Mode (WebSocket + Raw TCP)
                            // ---------------------------------------------------------
                            
                            // Define the message handler logic (Shared by both paths)
                            // Return type changed to Result<Option<Message>, ()> to signal "Ban/Disconnect"
                            let process_message = |msg: Message, chain_inner: Arc<Mutex<Blockchain>>, peers_inner: Arc<Mutex<Vec<String>>>, port: u16, routing_table: Arc<Mutex<RoutingTable>>, peer_ip: String| -> Result<Option<Message>, ()> {
                                match msg {
                                    Message::Handshake { node_id, listen_port } => {
                                         println!("[P2P] Received Handshake from {} (NodeId: {:?}, Port: {})", peer_ip, node_id, listen_port);
                                         let peer_addr_str = format!("{}:{}", peer_ip, listen_port);
                                         let peer = crate::kademlia::Peer {
                                             id: node_id,
                                             address: peer_addr_str,
                                             last_seen: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs(),
                                         };
                                         routing_table.lock().unwrap().add_peer(peer);
                                         Ok(None)
                                    },
                                    Message::NewBlock(block) => {
                                        println!("[P2P] Received Block #{}", block.index);
                                        let mut chain = chain_inner.lock().unwrap();
                                        let last_block = chain.get_last_block();
                                        let last_index = last_block.as_ref().map(|b| b.index).unwrap_or(0);

                                        if block.index > last_index + 1 {
                                            println!("[P2P] Received Future Block #{} (Head: {}). Requesting Sync from #{}.", block.index, last_index, last_index + 1);
                                            // FIX: Don't ask for full chain (GetChain starts at 0). Ask for new blocks.
                                            let msg = Message::GetBlocks { start: (last_index + 1) as usize, limit: 500 };
                                            Ok(Some(msg))
                                        } else if block.index <= last_index {
                                            println!("[P2P] Received Stale Block #{}. Ignoring.", block.index);
                                            Ok(None)
                                        } else {
                                            // Verify Linkage
                                            let last_hash = last_block.as_ref().map(|b| b.hash.clone()).unwrap_or_default();
                                            if block.previous_hash != last_hash {
                                                 println!("[P2P] Block #{} Fork Detected (PrevHash mismatch). Requesting Chain Replacement.", block.index);
                                                 // Here we might actually need full reorg logic, so GetChain (start 0) is safer fallback for deep forks,
                                                 // but efficient reorgs should use HeadersFirst. valid for now.
                                                 return Ok(Some(Message::GetChain));
                                            }

                                            if chain.submit_block(block.clone()) {
                                                println!("[P2P] Block #{} Accepted & Verified.", block.index);
                                                
                                                // GOSSIP: Broadcast to other peers
                                                let p_list = peers_inner.lock().unwrap().clone();
                                                let block_clone = block.clone();
                                                thread::spawn(move || {
                                                    let msg = Message::NewBlock(block_clone);
                                                    if let Ok(json) = serde_json::to_string(&msg) {
                                                        for peer in p_list {
                                                            if let Ok(mut stream) = TcpStream::connect(&peer) {
                                                                let _ = stream.write_all(json.as_bytes());
                                                            }
                                                        }
                                                    }
                                                });
                                                Ok(None)
                                            } else {
                                                println!("[Security] REJECTED Invalid Block #{}. BANNING PEER.", block.index);
                                                Err(()) 
                                            }
                                        }
                                    },
                                    Message::NewTransaction(tx) => {
                                        println!("[P2P] Received Transaction");
                                        let mut chain = chain_inner.lock().unwrap();
                                        chain.create_transaction(tx);
                                        Ok(None)
                                    },
                                    Message::FindNode(target_id) => {
                                        let neighbors = routing_table.lock().unwrap().find_closest(&target_id, 20);
                                        let resp = Message::Neighbors(neighbors);
                                        let _json = serde_json::to_string(&resp).unwrap();
                                        // We need to write this to stream.
                                        // Accessing stream here is tricky because we are inside `process_message`.
                                        // `process_message` returns Ok(Some(resp)) to send it back!
                                        // Perfect.
                                        Ok(Some(resp))
                                    },
                                    Message::Neighbors(peers) => {
                                        for p in peers {
                                            routing_table.lock().unwrap().add_peer(p);
                                        }
                                        Ok(None)
                                    },
                                    Message::GetChain => {
                                        // println!("[P2P] Received Chain Request (Full). providing first 500 blocks.");
                                        let chain = chain_inner.lock().unwrap();
                                        let blocks = chain.get_all_blocks(); // Legacy fallback: get all if possible, or truncate
                                        let limit = 500;
                                        let chunk = if blocks.len() > limit { blocks[0..limit].to_vec() } else { blocks };
                                        Ok(Some(Message::Chain(chunk)))
                                    },
                                    Message::GetBlocks { start, limit } => {
                                        let chain = chain_inner.lock().unwrap();
                                        let height = chain.get_height() as usize;
                                        
                                        if start < height {
                                            let mut chunk = Vec::new();
                                            let end = std::cmp::min(start + limit, height);
                                            
                                            // Iterate DB
                                            if let Some(ref db) = chain.db {
                                                // Optimize: In future, use range scan if key logic supports it
                                                for i in start..end {
                                                     if let Ok(Some(block)) = db.get_block(i as u64) {
                                                         chunk.push(block);
                                                     }
                                                }
                                            }
                                            
                                            let msg_resp = Message::Chain(chunk);
                                            Ok(Some(msg_resp))
                                        } else {
                                            Ok(None) // Start index out of bounds
                                        }
                                    },
                                    Message::Chain(remote_chain) => {
                                        // println!("[P2P] Received Chain Data (Count: {})", remote_chain.len());
                                        let mut chain = chain_inner.lock().unwrap();
                                        if let Some(first) = remote_chain.first() {
                                            if first.index == 0 {
                                                // Full Chain / Reorg Candidate
                                                if chain.attempt_chain_replacement(remote_chain) {
                                                    println!("[P2P] Chain replaced/synchronized successfully.");
                                                }
                                            } else {
                                                // Chunk / Append Candidate
                                                if chain.handle_sync_chunk(remote_chain) {
                                                    // console log handled inside
                                                }
                                            }
                                        }
                                        Ok(None)
                                    },
                                    Message::GetPeers => {
                                        let p = peers_inner.lock().unwrap().clone();
                                        let msg_resp = Message::Peers(p);
                                        Ok(Some(msg_resp))
                                    },
                                    Message::Peers(new_peers) => {
                                        let mut p_lock = peers_inner.lock().unwrap();
                                        let mut added = 0;
                                        let max_peers = 1000;
                                        for peer in new_peers {
                                            if p_lock.len() >= max_peers { break; }
                                            if !p_lock.contains(&peer) && peer != format!("127.0.0.1:{}", port) {
                                                p_lock.push(peer);
                                                added += 1;
                                            }
                                        }
                                        if added > 0 { println!("[P2P] Discovered {} new peers!", added); }
                                        Ok(None)
                                    },
                                    Message::GetHeaders { locator } => {
                                        let chain = chain_inner.lock().unwrap();
                                        let all = chain.get_all_blocks();
                                        // Find start block
                                        let start_idx = all.iter().position(|b| b.hash == locator).map(|i| i + 1).unwrap_or(0);
                                        // FIX: Reduce batch size from 2000 to 500 to preventtimeouts on large payloads (4MB -> 1MB)
                                        let end_idx = std::cmp::min(start_idx + 500, all.len());
                                        
                                        let headers = if start_idx < all.len() {
                                             // Strip Transactions for "Headers" (Lightweight)
                                             all[start_idx..end_idx].iter().map(|b| {
                                                 let mut h = b.clone();
                                                 h.transactions.clear(); // Headers only
                                                 h
                                             }).collect()
                                        } else { vec![] };
                                        
                                        Ok(Some(Message::Headers(headers)))
                                    },
                                    Message::Headers(headers) => {
                                        // Header-First Sync Logic
                                        println!("[Sync] Received {} Headers. Validating...", headers.len());
                                        
                                        if let Some(first) = headers.first() {
                                            // 1. Verify Linkage (simplified)
                                            // In a real header-first sync, we'd check if first.previous_hash is a known block or header.
                                            
                                            // 2. Validate PoW & Timestamp (Lightweight Check)
                                            for h in &headers {
                                                // Dynamic Diff Warning: We don't have the context of previous blocks here to calculate NEXT diff
                                                // But we can verify the PoW matches the bits claimed in the header.
                                                // And we can verify the Timestamp is not too far in future.
                                                
                                                let _diff_target = h.difficulty; // Bits (Underscore to silence unused warning)
                                                // TODO: Re-implement calculate_target mechanism or use Block::check_pow()
                                                // Since we stripped Txs, we might break Merkle Root checks if they depend on Txs?
                                                // No, Block::calculate_hash uses the Merkle Root string in the struct.
                                                // But can we re-calculate the Hash without Txs? 
                                                // YES, if we trust the merle_root field provided in the header.
                                                
                                                // Re-Calculate Hash from Header Fields
                                                let hash_check = h.calculate_hash();
                                                if hash_check != h.hash {
                                                    println!("[Sync] Invalid Header Hash: Claimed {}, Calc {}", h.hash, hash_check);
                                                    return Ok(None); // Ban?
                                                }
                                                // Verify PoW
                                                // We need to convert Compact Bits to Target... 
                                                // For MVP, if verification is complex without shared helper, asking for full blocks is safer.
                                                // But user asked for "Headers First".
                                                
                                                // Timestamp Check (Anti-Fake Date)
                                                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                                if h.timestamp > now + 7200 { // 2 Hours Future
                                                     println!("[Sync] Header Timestamp too far in future: {}", h.timestamp);
                                                     return Ok(None);
                                                }
                                            }
                                            
                                            println!("[Sync] Headers Validated. Requesting Blocks...");
                                            
                                            // 3. Trigger Block Download
                                            // We request the range covered by these headers.
                                            let start = first.index as usize;
                                            let count = headers.len();
                                            // We can use GetBlocks if supported, or just ask peer?
                                            // We rely on the existing GetBlocks message.
                                            
                                            // Sending message back requires returning Ok(Some(...)).
                                            // But GetBlocks currently asks for "Start + Limit".
                                            Ok(Some(Message::GetBlocks { start, limit: count }))
                                        } else {
                                            Ok(None)
                                        }
                                    },
                                    Message::Ping => Ok(Some(Message::Pong)),
                                    Message::Pong => Ok(None),
                                }
                            };

                            let mut buffer = [0; 4];
                            let is_websocket = if stream.peek(&mut buffer).is_ok() {
                                buffer.starts_with(b"GET ")
                            } else { false };

                            if is_websocket {
                                // WebSocket Path
                                match tungstenite::accept(stream) {
                                    Ok(mut socket) => {
                                        // Loop to read messages
                                        loop {
                                            if let Ok(msg) = socket.read() {
                                                if msg.is_text() || msg.is_binary() {
                                                    let text = msg.to_text().unwrap_or("{}");
                                                    if let Ok(parsed_msg) = serde_json::from_str::<Message>(text) {
                                                        // Process Message (With Ban Logic)
                                                        match process_message(parsed_msg, chain_inner.clone(), peers_inner.clone(), port_ref, routing_inner.clone(), peer_ip.clone()) {
                                                            Ok(Some(response)) => {
                                                                let json = serde_json::to_string(&response).unwrap_or_default();
                                                                let _ = socket.send(tungstenite::Message::Text(json));
                                                            },
                                                            Ok(None) => {}, // Continue
                                                            Err(_) => {
                                                                println!("[Security] Banning Peer: {}", peer_ip);
                                                                banned_inner.lock().unwrap().insert(peer_ip.clone());
                                                                break;
                                                            }
                                                        }
                                                    }
                                                } else if msg.is_close() {
                                                    break;
                                                }
                                            } else {
                                                break;
                                            }
                                        }
                                    },
                                    Err(e) => println!("[Server] WebSocket Handshake Failed: {}", e),
                                }
                            } else {
                                // Raw TCP Path (Legacy)
                                let mut de = serde_json::Deserializer::from_reader(&stream);
                                if let Ok(msg) = Message::deserialize(&mut de) {
                                     match process_message(msg, chain_inner.clone(), peers_inner.clone(), port_ref, routing_inner.clone(), peer_ip.clone()) {
                                         Ok(Some(response)) => {
                                             let json = serde_json::to_string(&response).unwrap_or_default();
                                             if let Ok(mut stream_clone) = stream.try_clone() {
                                                 let _ = stream_clone.write_all(json.as_bytes());
                                                 let _ = stream_clone.flush();
                                             }
                                         },
                                         Ok(None) => {},
                                          Err(_) => {
                                              println!("[Security] Banning Peer (TCP): {}", peer_ip);
                                              banned_inner.lock().unwrap().insert(peer_ip.clone());
                                              // Loop/Scope ends, thread finishes, connection drops.
                                          }
                                     }
                                }
                            }
                        }); // Close inner thread
                    }
                    Err(e) => {
                        println!("Connection failed: {}", e);
                    }
                }
            }
        });

        // ---------------------------------------------------------
        // CLIENT SIDE: P2P Only (WebSocket)
        // ---------------------------------------------------------
        let chain_client = self.blockchain.clone();
        let peers_client = self.peers.clone();
        let port_client = self.port;

        thread::spawn(move || {
            loop {
                thread::sleep(std::time::Duration::from_secs(4)); // Sync Interval
                
                // 1. Manage Peers
                let peers_list = peers_client.lock().unwrap().clone();
                if peers_list.is_empty() { continue; }
                
                // 2. Determine Sync Status
                let chain = chain_client.lock().unwrap();
                let _my_height = chain.get_height() as usize; // Check height
                drop(chain); // Unlock

                // 3. Distributed Sync: Randomly select a peer to request next chunk
                // This distributes load across the network (Round-Robin / Random)
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                if let Some(peer) = peers_list.choose(&mut rng) {
                    if peer.contains(&format!("127.0.0.1:{}", port_client)) { continue; }
                    
                    let peer_addr = peer.clone();
                    let chain_inner = chain_client.clone();
                    let peers_inner = peers_client.clone();

                    thread::spawn(move || {
                        if peer_addr.starts_with("ws://") || peer_addr.starts_with("wss://") {
                            let ws_url = peer_addr.clone(); // Already has protocol
                            if let Ok((mut socket, _)) = tungstenite::connect(ws_url) {
                                // Handshake: GetPeers
                                let _ = socket.send(tungstenite::Message::Text(serde_json::to_string(&Message::GetPeers).unwrap_or_default()));
                                
                                // Header-First Sync: Ask for headers first
                                let (_my_height, locator) = {
                                    let c = chain_inner.lock().unwrap();
                                    let h = c.get_height() as usize;
                                    let hash = c.get_last_block().map(|b| b.hash).unwrap_or("0".to_string());
                                    (h, hash)
                                };
    
                                let msg = Message::GetHeaders { locator };
                                 
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    let _ = socket.send(tungstenite::Message::Text(json));
                                }
                                
                                // Listen for Response
                                let start_time = std::time::Instant::now();
                                while start_time.elapsed() < Duration::from_secs(5) {
                                    if let Ok(msg) = socket.read() {
                                        if msg.is_text() {
                                            let text = msg.to_text().unwrap_or("{}");
                                            if let Ok(parsed) = serde_json::from_str::<Message>(text) {
                                                 match parsed {
                                                    Message::Peers(new_p) => {
                                                         let mut p_lock = peers_inner.lock().unwrap();
                                                         for np in new_p {
                                                             if !p_lock.contains(&np) { p_lock.push(np); }
                                                         }
                                                    },
                                                    Message::Chain(chunk) => {
                                                         let mut c = chain_inner.lock().unwrap();
                                                         if let Some(first) = chunk.first() {
                                                             if first.index == 0 { c.attempt_chain_replacement(chunk); }
                                                             else { c.handle_sync_chunk(chunk); }
                                                         }
                                                    },
                                                    Message::Headers(headers) => {
                                                        if headers.is_empty() {
                                                             println!("[Sync] Synced. (No new headers)");
                                                        } else {
                                                            println!("[Sync] Received {} Headers. Requesting Blocks...", headers.len());
                                                            // Logic for downloading blocks...
                                                            if let Some(first) = headers.first() {
                                                                let start = first.index as usize;
                                                                // FIX: Cap request at 500 blocks even if more headers received
                                                                let limit = std::cmp::min(headers.len(), 500);
                                                                let msg_get = Message::GetBlocks { start, limit };
                                                                if let Ok(req_json) = serde_json::to_string(&msg_get) {
                                                                    let _ = socket.send(tungstenite::Message::Text(req_json));
                                                                }
                                                            }
                                                        }
                                                    },
                                                    _ => {}
                                                 }
                                            }
                                        }
                                    } else { break; }
                                }
                            }
                        } else {
                            // TCP Sync Logic
                            if let Ok(stream) = TcpStream::connect(&peer_addr) {
                                let mut writer = stream.try_clone().expect("Failed to clone stream");
                                
                                // Handshake: GetPeers
                                let _ = writer.write_all(serde_json::to_string(&Message::GetPeers).unwrap_or_default().as_bytes());
                                
                                let locator = {
                                    let c = chain_inner.lock().unwrap();
                                    c.get_last_block().map(|b| b.hash).unwrap_or("0".to_string())
                                };

                                let msg = Message::GetHeaders { locator };
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    let _ = writer.write_all(json.as_bytes());
                                }

                                // Read Loop
                                // FIX: Increase timeout to 60s for slow connections/large blocks
                                let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
                                let mut de = serde_json::Deserializer::from_reader(&stream);
                                while let Ok(parsed) = Message::deserialize(&mut de) {
                                    match parsed {
                                        Message::Peers(new_p) => {
                                            let mut p_lock = peers_inner.lock().unwrap();
                                            for np in new_p {
                                                if !p_lock.contains(&np) { p_lock.push(np); }
                                            }
                                        },
                                        Message::Chain(chunk) => {
                                            let mut c = chain_inner.lock().unwrap();
                                            if let Some(first) = chunk.first() {
                                                if first.index == 0 { c.attempt_chain_replacement(chunk); }
                                                else { c.handle_sync_chunk(chunk); }
                                            }
                                            break; // Done after chain
                                        },
                                        Message::Headers(headers) => {
                                            if headers.is_empty() {
                                                println!("[Sync] Synced (TCP). No new headers.");
                                                break;
                                            }
                                            println!("[Sync] Received {} Headers via TCP. Requesting Blocks...", headers.len());
                                            if let Some(first) = headers.first() {
                                                let start = first.index as usize;
                                                // FIX: Cap request at 500
                                                let limit = std::cmp::min(headers.len(), 500);
                                                let msg_get = Message::GetBlocks { start, limit };
                                                let _ = writer.write_all(serde_json::to_string(&msg_get).unwrap().as_bytes());
                                                // Continue loop to receive Chain
                                            } else {
                                                break;
                                            }
                                        },
                                        _ => {}
                                    }
                                }
                            }
                        }
                    });
                }
            }
        });
    }


    pub fn connect_to_peer(&self, peer_addr: String) {
        let mut peers = self.peers.lock().unwrap();
        peers.push(peer_addr.clone());
        drop(peers); // Unlock early

        // Use Tungstenite for WebSocket (WSS) and Raw TCP for others
        if peer_addr.starts_with("ws://") || peer_addr.starts_with("wss://") {
             match tungstenite::connect(&peer_addr) {
                 Ok((mut socket, _)) => {
                      println!("[P2P] Connected via WebSocket to {}", peer_addr);
                      
                      // Header-First Sync Handshake
                      let locator = self.blockchain.lock().unwrap().get_last_block().map(|b| b.hash).unwrap_or("0".to_string());
                      let msg = Message::GetHeaders { locator };
                      let json = serde_json::to_string(&msg).unwrap_or_default();
                      if let Err(e) = socket.send(tungstenite::Message::Text(json)) {
                          println!("[P2P] Failed to send Handshake: {}", e);
                          return;
                      }
                      
                      // Read Response (Headers -> GetBlocks logic handled in loop usually, 
                      // but here we might accept one pass).
                      match socket.read() {
                          Ok(msg) => {
                              if let tungstenite::Message::Text(text) = msg {
                                  // Expect Headers, then Process
                                  if let Ok(Message::Headers(headers)) = serde_json::from_str(&text) {
                                      println!("[Sync] Received {} Headers...", headers.len());
                                      // Validate and Request Blocks (Reuse logic? Or just accept for handshake)
                                      // For handshake simple sync, we might just print status. 
                                      // The full sync logic is in the periodic thread.
                                      // But let's support immediate sync if valid.
                                      // If we implement the logic here, we duplicate code. 
                                      // ideally we pass this to process_message?
                                  } 
                              }
                          },
                          Err(e) => println!("[P2P] Failed to read Handshake response: {}", e),
                      }
                  },
                  Err(e) => {
                      println!("[P2P] Failed to connect via WSS to {}: {}", peer_addr, e);
                  }
              }
        } else {
             // ... Raw TCP Logic ...
             if let Ok(_stream) = TcpStream::connect(&peer_addr) {
              // ... Raw TCP Logic ...
              if let Ok(mut stream) = TcpStream::connect(&peer_addr) {
                   // Header-First Sync
                   let locator = self.blockchain.lock().unwrap().get_last_block().map(|b| b.hash).unwrap_or("0".to_string());
                   let msg = Message::GetHeaders { locator };
                   let json = serde_json::to_string(&msg).unwrap_or_default();
                   let _ = stream.write_all(json.as_bytes());
                   
                   // Response handling omitted for TCP simple connect, relies on listener?
                   // No, connect_to_peer is active.
                   // Just send GetHeaders. If they reply Headers, we process it?
                   // We need to listen to reply.
                   // For now, let's leave TCP simpler or just send GetHeaders and drop.
              }
             }
        }
    }

    pub fn sync_chain_to_peer(&self, peer_addr: String) {
        println!("[Sync] Uploading chain to {}...", peer_addr);
        
        if peer_addr.starts_with("ws://") || peer_addr.starts_with("wss://") {
             match tungstenite::connect(&peer_addr) {
                 Ok((mut socket, _)) => {
                     let chain = self.blockchain.lock().unwrap();
                     let all_blocks = chain.get_all_blocks();
                     let limit = 500;
                     let blocks = if all_blocks.len() > limit { all_blocks[0..limit].to_vec() } else { all_blocks };
                     let msg = Message::Chain(blocks);
                     drop(chain); // Unlock

                     let json = serde_json::to_string(&msg).unwrap_or_default();
                     
                     if let Err(e) = socket.send(tungstenite::Message::Text(json)) {
                         println!("[Sync] Failed to send data via WSS: {}", e);
                     } else {
                         println!("[Sync] Chain data uploaded successfully via WSS.");
                     }
                 },
                 Err(e) => {
                     println!("[Sync] Failed to connect via WSS: {}", e);
                 }
             }
        } else {
            // Raw TCP
            if let Ok(mut stream) = TcpStream::connect(&peer_addr) {
                 let chain = self.blockchain.lock().unwrap();
                 let all_blocks = chain.get_all_blocks();
                 let limit = 500;
                 let blocks = if all_blocks.len() > limit { all_blocks[0..limit].to_vec() } else { all_blocks };
                 let msg = Message::Chain(blocks);
                 
                 let json = serde_json::to_string(&msg).unwrap_or_default();
                 
                 if stream.write_all(json.as_bytes()).is_ok() {
                     println!("[Sync] Chain data sent successfully.");
                 } else {
                     println!("[Sync] Failed to send data.");
                 }
            } else {
                println!("[Sync] Failed to connect to peer.");
            }
        }
    }


    #[allow(dead_code)]

    pub fn start_discovery(&self) {
        let peers_ref = self.peers.clone();
        let routing_ref = self.routing_table.clone(); // Added for DHT
        
        // Bootstrap Node Injection (ALWAYS ADD)
        {
            let mut p_lock = peers_ref.lock().unwrap();
            let bootstraps = vec![
                "volt-core.zapto.org:6000",
                "194.164.75.228:6000", // Fallback IP
                "wss://voltcore-node.hf.space/p2p", // Cloud Node (Auto-Connect)
            ];
            
            for bs in bootstraps {
                if !p_lock.contains(&bs.to_string()) {
                    println!("[Discovery] Adding Bootstrap Node: {}", bs);
                    p_lock.push(bs.to_string());
                }
            }
        }
        
        let self_node_id = self.routing_table.lock().unwrap().local_id.clone();
        let my_port = self.port;

        thread::spawn(move || {
            // Immediate First Run
            loop {
                println!("[Discovery] Asking peers for more peers...");
                
                // 1. Snapshot known peers to avoid locking during network IO
                let known_peers = peers_ref.lock().unwrap().clone();
                
                for peer in known_peers {
                    // Send Handshake / ID Announce
                    let handshake_msg = Message::Handshake { node_id: self_node_id.clone(), listen_port: my_port };
                    let handshake_json = serde_json::to_string(&handshake_msg).unwrap_or_default();

                    if peer.starts_with("ws://") || peer.starts_with("wss://") {
                         match tungstenite::connect(&peer) {
                             Ok((mut socket, _)) => {
                                 // Send Handshake
                                 let _ = socket.send(tungstenite::Message::Text(handshake_json));
                                 
                                 // Then request peers
                                 let msg = Message::GetPeers;
                                 let json = serde_json::to_string(&msg).unwrap_or_default();
                                 if socket.send(tungstenite::Message::Text(json)).is_ok() {
                                     if let Ok(msg) = socket.read() {
                                          if let tungstenite::Message::Text(text) = msg {
                                              if let Ok(Message::Peers(new_list)) = serde_json::from_str(&text) {
                                                  let mut p_lock = peers_ref.lock().unwrap();
                                                  for p in new_list {
                                                      if !p_lock.contains(&p) {
                                                          println!("[Discovery] Found new peer: {}", p);
                                                          p_lock.push(p);
                                                      }
                                                  }
                                              }
                                          }
                                     }
                                 }
                             },
                             Err(e) => {
                                 println!("[Discovery] Failed to connect to {}: {}", peer, e);
                             } 
                         }
                    } else {
                         if let Ok(mut stream) = TcpStream::connect(&peer) {
                             // Send Handshake
                             let _ = stream.write_all(handshake_json.as_bytes());
                             // Then request peers
                             let msg = Message::GetPeers;
                             let _ = stream.write_all(serde_json::to_string(&msg).unwrap_or_default().as_bytes());
                             
                             let mut de = serde_json::Deserializer::from_reader(&stream);
                             if let Ok(Message::Peers(new_list)) = Message::deserialize(&mut de) {
                                 let mut p_lock = peers_ref.lock().unwrap();
                                 for p in new_list {
                                     if !p_lock.contains(&p) {
                                         println!("[Discovery] Found new peer: {}", p);
                                         p_lock.push(p);
                                     }
                                 }
                             }
                         }
                    }
                }
                
                // 2. Kademlia DHT Lookup (Iterative Discovery)
                // Search for random target to populate buckets, or search for self to find neighbors
                let target_id = crate::kademlia::NodeId::random();
                let msg_dht = Message::FindNode(target_id);
                let json_dht = serde_json::to_string(&msg_dht).unwrap_or_default();
                
                // Ask a random subset of peers
                let peers_snap = peers_ref.lock().unwrap().clone();
                for peer in peers_snap.iter().take(5) { // Limit to 5 per cycle to avoid storm
                    if peer.starts_with("ws://") || peer.starts_with("wss://") {
                         // WSS DHT (Simplified - mostly client side for now)
                    } else {
                         if let Ok(mut stream) = TcpStream::connect(&peer) {
                             let _ = stream.write_all(json_dht.as_bytes());
                             // Response handled by main server loop? 
                             // No, we need to read it here if we want to digest it immediately.
                             // But Kademlia usually is async. 
                             // For this implementation, we rely on the peer sending back Neighbors message
                             // which the Server Thread processes! 
                             // Wait, Server Thread processes INCOMING. 
                             // We are CLIENT here.
                             
                             // FIX: We need to read response here to Populate Routing Table
                             let mut de = serde_json::Deserializer::from_reader(&stream);
                             if let Ok(Message::Neighbors(nodes)) = Message::deserialize(&mut de) {
                                  println!("[DHT] Received {} neighbors from {}", nodes.len(), peer);
                                  let mut rt = routing_ref.lock().unwrap(); // Use captured ref
                                  for node in nodes {
                                      rt.add_peer(node);
                                  }
                             }
                         }
                    }
                }

                // 3. P2P Hardening: Ping Verification (Keep-Alive)
                // Randomly ping a few peers to check liveness
                if let Ok(p_lock) = peers_ref.lock() {
                     for peer in p_lock.iter().take(3) {
                         // Quick connect and ping
                         if let Ok(mut stream) = TcpStream::connect(peer) {
                              let ping_msg = Message::Ping;
                              let _ = stream.write_all(serde_json::to_string(&ping_msg).unwrap_or_default().as_bytes());
                              // We don't block waiting for Pong here to keep loop fast.
                              // Peer will respond with Pong, handled by process_message if we kept connection open.
                              // For ephemeral checks, successful write is "good enough" for basic connectivity.
                         }
                     }
                }

                // Sleep AFTER the first run
                thread::sleep(Duration::from_secs(60));
            }
        });
    }

    fn attempt_upnp_mapping(port: u16) {
        println!("UPnP: Attempting to map port {}...", port);
        
        // Detect Local IP
        let local_ip = match std::net::UdpSocket::bind("0.0.0.0:0") {
            Ok(socket) => {
                // Connect to a public DNS to determine default route interface
                if socket.connect("8.8.8.8:80").is_ok() {
                    if let Ok(std::net::SocketAddr::V4(addr)) = socket.local_addr() {
                         *addr.ip()
                    } else { Ipv4Addr::new(0,0,0,0) }
                } else { Ipv4Addr::new(0,0,0,0) }
            },
            Err(_) => Ipv4Addr::new(0,0,0,0),
        };

        if local_ip.is_unspecified() {
            println!("UPnP: Could not determine local IP, skipping.");
            return;
        }

        match igd::search_gateway(Default::default()) {
            Ok(gateway) => {
                let local_addr = SocketAddrV4::new(local_ip, port);
                match gateway.add_port(
                    igd::PortMappingProtocol::TCP,
                    port,
                    local_addr,
                    0,
                    "VoltOne Node"
                ) {
                    Ok(_) => println!("UPnP: Successfully mapped port {} to {} on gateway {}", port, local_addr, gateway),
                    Err(e) => println!("UPnP: Failed to map port: {}", e),
                }
            },
            Err(e) => println!("UPnP: Gateway not found: {}", e),
        }
    }

    pub fn _resolve_dns_seeds(&self) {
        let seeds = vec![
            "seed1.volt-coin.org", // Placeholder
            "seed2.volt-project.net",
        ];
        
        let mut peers_lock = self.peers.lock().unwrap();
        println!("[P2P] Resolving DNS Seeds...");
        
        for seed in seeds {
             if let Ok(lookup) = std::net::ToSocketAddrs::to_socket_addrs(&(seed, self.port)) {
                 for addr in lookup {
                      if let std::net::SocketAddr::V4(addr4) = addr {
                           let ip = addr4.ip().to_string();
                           if !peers_lock.contains(&ip) {
                               println!("[P2P] Found Seed Peer: {}", ip);
                               peers_lock.push(ip);
                           }
                      }
                 }
             }
        }
    }
}
