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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    NewBlock(Block),
    NewTransaction(Transaction),
    GetChain,
    GetBlocks { start: usize, limit: usize },
    Chain(Vec<Block>),
    GetPeers,
    Peers(Vec<String>),
}

pub struct Node {
    pub blockchain: Arc<Mutex<Blockchain>>,
    pub peers: Arc<Mutex<Vec<String>>>,
    pub banned_peers: Arc<Mutex<HashSet<String>>>, // New Field
    pub port: u16,
}

impl Node {
    pub fn new(blockchain: Arc<Mutex<Blockchain>>, port: u16) -> Self {
        Node {
            blockchain,
            peers: Arc::new(Mutex::new(Vec::new())),
            banned_peers: Arc::new(Mutex::new(HashSet::new())),
            port,
        }
    }

    pub fn start_server(&self) {
        let port = self.port;
        // UPnP: Try to open port
        Node::attempt_upnp_mapping(port);

        let chain_ref = self.blockchain.clone();
        let peers_ref = self.peers.clone();
        let port_ref = self.port;
        
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
                    Ok(mut stream) => { 
                        let chain_inner = chain_ref.clone();
                        let peers_inner = peers_ref.clone();
                        let banned_inner = self.banned_peers.clone(); // Capture
                        
                        // Check Ban
                        let peer_addr = stream.peer_addr();
                        let peer_ip = if let Ok(addr) = peer_addr { addr.ip().to_string() } else { "unknown".to_string() };
                        
                        if banned_inner.lock().unwrap().contains(&peer_ip) {
                            println!("[Security] Rejected connection from BANNED IP: {}", peer_ip);
                            continue; // Drop connection
                        }

                        thread::spawn(move || {
                            // ---------------------------------------------------------
                            // SERVER SIDE: Dual Mode (WebSocket + Raw TCP)
                            // ---------------------------------------------------------
                            
                            // Define the message handler logic (Shared by both paths)
                            // Return type changed to Result<Option<Message>, ()> to signal "Ban/Disconnect"
                            let process_message = |msg: Message, chain_inner: Arc<Mutex<Blockchain>>, peers_inner: Arc<Mutex<Vec<String>>>, port: u16| -> Result<Option<Message>, ()> {
                                match msg {
                                    Message::NewBlock(block) => {
                                        println!("[P2P] Received Block #{}", block.index);
                                        let mut chain = chain_inner.lock().unwrap();
                                        let last_block = chain.get_last_block();
                                        let last_index = last_block.as_ref().map(|b| b.index).unwrap_or(0);

                                        if block.index > last_index + 1 {
                                            println!("[P2P] Received Future Block #{} (Head: {}). Requesting Sync.", block.index, last_index);
                                            Ok(Some(Message::GetChain))
                                        } else if block.index <= last_index {
                                            println!("[P2P] Received Stale Block #{}. Ignoring.", block.index);
                                            Ok(None)
                                        } else {
                                            // Verify Linkage
                                            let last_hash = last_block.as_ref().map(|b| b.hash.clone()).unwrap_or_default();
                                            if block.previous_hash != last_hash {
                                                 println!("[P2P] Block #{} Fork Detected. Requesting Sync to resolve.", block.index);
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
                                    Message::GetChain => {
                                        println!("[P2P] Received Chain Request (Full). Ignored in DB Mode (Use GetBlocks).");
                                        // Return empty to signal no-op or just handle sync elsewhere
                                        Ok(None)
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
                                    }
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
                                                        match process_message(parsed_msg, chain_inner.clone(), peers_inner.clone(), port) {
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
                                     match process_message(msg, chain_inner.clone(), peers_inner.clone(), port) {
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
                let mut peers_list = peers_client.lock().unwrap().clone();
                if peers_list.is_empty() { continue; }
                
                // 2. Determine Sync Status
                let chain = chain_client.lock().unwrap();
                let my_height = chain.chain.len();
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
                        let ws_url = format!("ws://{}", peer_addr);
                        if let Ok((mut socket, _)) = tungstenite::connect(ws_url) {
                            // Handshake: GetPeers
                            let _ = socket.send(tungstenite::Message::Text(serde_json::to_string(&Message::GetPeers).unwrap()));
                            
                            // CHUNK REQUEST: Ask for next 500 blocks
                            let msg = Message::GetBlocks { start: my_height, limit: 500 };
                             
                            if let Ok(json) = serde_json::to_string(&msg) {
                                let _ = socket.send(tungstenite::Message::Text(json));
                            }
                            
                            // Listen for Response (Short lived connection for sync step)
                            // We wait for a few seconds to receive data then close
                            socket.get_mut().set_read_timeout(Some(Duration::from_secs(5))).ok();
                            
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
                                                     // Reuse the logic: index 0 = Full, index > 0 = Chunk
                                                     if let Some(first) = chunk.first() {
                                                         if first.index == 0 { c.attempt_chain_replacement(chunk); }
                                                         else { c.handle_sync_chunk(chunk); }
                                                     }
                                                },
                                                _ => {}
                                             }
                                        }
                                    }
                                } else { break; }
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
                     
                     // Handshake: GetChain
                     let msg = Message::GetChain;
                     let json = serde_json::to_string(&msg).unwrap_or_default();
                     if let Err(e) = socket.send(tungstenite::Message::Text(json)) {
                         println!("[P2P] Failed to send Handshake: {}", e);
                         return;
                     }
                     
                     // Read Response
                     match socket.read() {
                         Ok(msg) => {
                             if let tungstenite::Message::Text(text) = msg {
                                 if let Ok(Message::Chain(remote_chain)) = serde_json::from_str(&text) {
                                     println!("[Sync] Received chain from peer (Height: {})", remote_chain.len());
                                     let mut chain = self.blockchain.lock().unwrap();
                                     if chain.attempt_chain_replacement(remote_chain) {
                                         println!("[Sync] Sync complete.");
                                     }
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
             if let Ok(mut stream) = TcpStream::connect(&peer_addr) {
                  // ... (Existing Logic)
                  let msg = Message::GetChain;
                  let json = serde_json::to_string(&msg).unwrap_or_default();
                  let _ = stream.write_all(json.as_bytes());
                  
                  let mut de = serde_json::Deserializer::from_reader(&stream);
                  if let Ok(Message::Chain(remote_chain)) = Message::deserialize(&mut de) {
                      println!("[Sync] Received chain from peer (Height: {})", remote_chain.len());
                      let mut chain = self.blockchain.lock().unwrap();
                      if chain.attempt_chain_replacement(remote_chain) {
                          println!("[Sync] Sync complete.");
                      }
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
                     let msg = Message::Chain(chain.chain.clone());
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
                 let msg = Message::Chain(chain.chain.clone());
                 
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
    pub fn broadcast_block(&self, block: Block) {
        let msg = Message::NewBlock(block);
        let msg_json = serde_json::to_string(&msg).unwrap_or_default();
        let peers = self.peers.lock().unwrap();
        
        for peer in peers.iter() {
             if let Ok(mut stream) = TcpStream::connect(peer) {
                 let _ = stream.write(msg_json.as_bytes());
             }
        }
    }

    pub fn start_discovery(&self) {
        let peers_ref = self.peers.clone();
        
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
        
        thread::spawn(move || {
            // Immediate First Run
            loop {
                println!("[Discovery] Asking peers for more peers...");
                
                // 1. Snapshot known peers to avoid locking during network IO
                let known_peers = peers_ref.lock().unwrap().clone();
                
                for peer in known_peers {
                    if peer.starts_with("ws://") || peer.starts_with("wss://") {
                         match tungstenite::connect(&peer) {
                             Ok((mut socket, _)) => {
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

    pub fn resolve_dns_seeds(&self) {
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
