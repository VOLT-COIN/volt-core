use std::net::{TcpListener, TcpStream, SocketAddrV4, Ipv4Addr};
use std::io::Write;
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
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).expect("Failed to bind P2P port");
            println!("Server listening on port {}", port);

            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => { 
                        let chain_inner = chain_ref.clone();
                        let peers_inner = peers_ref.clone(); // Clone for this connection thread
                        thread::spawn(move || {
                            let mut de = serde_json::Deserializer::from_reader(&stream);
                            if let Ok(msg) = Message::deserialize(&mut de) {
                                match msg {
                                    Message::NewBlock(block) => {
                                        println!("[P2P] Received Block #{}", block.index);
                                        let mut chain = chain_inner.lock().unwrap();
                                        // FIX: Validate block before accepting! (Prevent Chain Corruption)
                                        if chain.submit_block(block.clone()) {
                                            println!("[P2P] Block #{} Accepted & Verified.", block.index);
                                            // chain.save() is called inside submit_block
                                        } else {
                                            println!("[Security] Rejected Invalid Block #{} from Peer.", block.index);
                                        }
                                    },
                                    Message::NewTransaction(tx) => {
                                        println!("[P2P] Received Transaction");
                                        let mut chain = chain_inner.lock().unwrap();
                                        chain.create_transaction(tx);
                                    },
                                    Message::GetChain => {
                                        println!("[P2P] Received Chain Request");
                                        let chain = chain_inner.lock().unwrap();
                                        let msg = Message::Chain(chain.chain.clone());
                                        let json = serde_json::to_string(&msg).unwrap_or_default();
                                        
                                        // Try to write back to the stream
                                        if let Ok(mut stream_clone) = stream.try_clone() {
                                            let _ = stream_clone.write_all(json.as_bytes());
                                            let _ = stream_clone.flush();
                                        }
                                    },
                                    Message::Chain(remote_chain) => {
                                        println!("[P2P] Received Chain Data (Height: {})", remote_chain.len());
                                        let mut chain = chain_inner.lock().unwrap();
                                        if chain.attempt_chain_replacement(remote_chain) {
                                            println!("[P2P] Chain synchronized successfully.");
                                        }
                                    },
                                    Message::GetPeers => {
                                        // Send our peer list
                                        let p = peers_inner.lock().unwrap().clone();
                                        let msg = Message::Peers(p);
                                        let json = serde_json::to_string(&msg).unwrap_or_default();
                                        if let Ok(mut stream_clone) = stream.try_clone() {
                                            let _ = stream_clone.write_all(json.as_bytes());
                                            let _ = stream_clone.flush();
                                        }
                                    },
                                    Message::Peers(new_peers) => {
                                        // Save unknown peers
                                        let mut p_lock = peers_inner.lock().unwrap();
                                        let mut added = 0;
                                        // FIX: Cap peer list at 1000 to prevent DoS
                                        let max_peers = 1000;
                                        
                                        for peer in new_peers {
                                            if p_lock.len() >= max_peers { break; }
                                            if !p_lock.contains(&peer) && peer != format!("127.0.0.1:{}", port) {
                                                p_lock.push(peer);
                                                added += 1;
                                            }
                                        }
                                        if added > 0 { println!("[P2P] Discovered {} new peers!", added); }
                                    }
                                }
                            }
                        });
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
        
        // Initial Handshake: Ask for chain
        if let Ok(mut stream) = TcpStream::connect(&peer_addr) {
             let msg = Message::GetChain;
             let json = serde_json::to_string(&msg).unwrap_or_default();
             let _ = stream.write_all(json.as_bytes());
             // We don't read response here; server thread handles response if peer connects back?
             // Actually, TcpStream is 1-way unless we keep it open.
             // Our architecture (start_server) spawns a thread for INCOMING.
             // For OUTGOING `connect_to_peer`, we write and close?
             // If we write GetChain, the peer processes it.
             // It tries to write BACK to the stream.
             // If we close 'stream' here, peer write fails.
             // We need to keep this stream open or listen for response?
             // Since `start_server` handles incoming, the peer must `connect_to_peer` BACK to us to send the Chain?
             // Or we read response here?
             // Simplest: Read response here.
             
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
        
        // Bootstrap Node Injection
        {
            let mut p_lock = peers_ref.lock().unwrap();
            if p_lock.is_empty() {
                // UPDATE: New Global Bootstrap Node (P2P Port 6000)
                let bootstrap = "volt-core.zapto.org:6000".to_string();
                println!("[Discovery] Initializing with Bootstrap Node: {}", bootstrap);
                p_lock.push(bootstrap);
            }
        }
        
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(60));
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
