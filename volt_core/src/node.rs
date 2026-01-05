use std::net::{TcpListener, TcpStream, SocketAddrV4, Ipv4Addr};
use std::io::{Read, Write};
use std::thread;
use std::sync::{Arc, Mutex};
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
    Chain(Vec<Block>),
    GetPeers,
    Peers(Vec<String>),
}

pub struct Node {
    pub blockchain: Arc<Mutex<Blockchain>>,
    pub peers: Arc<Mutex<Vec<String>>>,
    pub port: u16,
}

impl Node {
    pub fn new(blockchain: Arc<Mutex<Blockchain>>, port: u16) -> Self {
        Node {
            blockchain,
            peers: Arc::new(Mutex::new(Vec::new())),
            port,
        }
    }

    pub fn start_server(&self) {
        let port = self.port;
        // UPnP: Try to open port
        Node::attempt_upnp_mapping(port);

        let chain_ref = self.blockchain.clone();
        let peers_ref = self.peers.clone();
        
        thread::spawn(move || {
            // P2P Server (Internal Port 7861)
            let p2p_port = 7861;
            let listener = TcpListener::bind(format!("0.0.0.0:{}", p2p_port)).expect("Failed to bind P2P port");
            println!("P2P Server listening on internal port {}", p2p_port);

            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => { 
                        let chain_inner = chain_ref.clone();
                        let peers_inner = peers_ref.clone();
                        thread::spawn(move || {
                            // ---------------------------------------------------------
                            // SERVER SIDE: Dual Mode (WebSocket + Raw TCP)
                            // ---------------------------------------------------------
                            
                            // Define the message handler logic (Shared by both paths)
                            let process_message = |msg: Message, chain_inner: Arc<Mutex<Blockchain>>, peers_inner: Arc<Mutex<Vec<String>>>, port: u16| {
                                match msg {
                                    Message::NewBlock(block) => {
                                        println!("[P2P] Received Block #{}", block.index);
                                        let mut chain = chain_inner.lock().unwrap();
                                        if chain.submit_block(block.clone()) {
                                            println!("[P2P] Block #{} Accepted & Verified.", block.index);
                                        } else {
                                            println!("[Security] Rejected Invalid Block #{} from Peer.", block.index);
                                        }
                                        None
                                    },
                                    Message::NewTransaction(tx) => {
                                        println!("[P2P] Received Transaction");
                                        let mut chain = chain_inner.lock().unwrap();
                                        chain.create_transaction(tx);
                                        None
                                    },
                                    Message::GetChain => {
                                        println!("[P2P] Received Chain Request");
                                        let chain = chain_inner.lock().unwrap();
                                        let msg_resp = Message::Chain(chain.chain.clone());
                                        drop(chain); // Unlock
                                        // Return response to caller to send back
                                        Some(msg_resp)
                                    },
                                    Message::Chain(remote_chain) => {
                                        println!("[P2P] Received Chain Data (Height: {})", remote_chain.len());
                                        let mut chain = chain_inner.lock().unwrap();
                                        if chain.attempt_chain_replacement(remote_chain) {
                                            println!("[P2P] Chain synchronized successfully.");
                                        }
                                        None
                                    },
                                    Message::GetPeers => {
                                        let p = peers_inner.lock().unwrap().clone();
                                        let msg_resp = Message::Peers(p);
                                        Some(msg_resp)
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
                                        None
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
                                            if let Ok(msg) = socket.read_message() {
                                                if msg.is_text() || msg.is_binary() {
                                                    let text = msg.to_text().unwrap_or("{}");
                                                    if let Ok(parsed_msg) = serde_json::from_str::<Message>(text) {
                                                        // Process Message
                                                        if let Some(response) = process_message(parsed_msg, chain_inner.clone(), peers_inner.clone(), port) {
                                                            let json = serde_json::to_string(&response).unwrap_or_default();
                                                            let _ = socket.write_message(tungstenite::Message::Text(json));
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
                                     // NOTE: Original code only handled ONE message per connection for some reason?
                                     // Or the stream closes? Keeping original behavior for Raw TCP but reusing logic.
                                     if let Some(response) = process_message(msg, chain_inner.clone(), peers_inner.clone(), port) {
                                         let json = serde_json::to_string(&response).unwrap_or_default();
                                         if let Ok(mut stream_clone) = stream.try_clone() {
                                             let _ = stream_clone.write_all(json.as_bytes());
                                             let _ = stream_clone.flush();
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
                     if let Err(e) = socket.write_message(tungstenite::Message::Text(json)) {
                         println!("[P2P] Failed to send Handshake: {}", e);
                         return;
                     }
                     
                     // Read Response
                     match socket.read_message() {
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
                     
                     if let Err(e) = socket.write_message(tungstenite::Message::Text(json)) {
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
                    if let Ok(mut stream) = TcpStream::connect(&peer) {
                        let msg = Message::GetPeers;
                        let _ = stream.write_all(serde_json::to_string(&msg).unwrap_or_default().as_bytes());
                        
                        // Read response immediately
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
}
