use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use crate::block::Block;
use crate::transaction::Transaction;
use crate::kademlia::{NodeId, Peer, RoutingTable};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Handshake { port: u16, id: Option<NodeId> }, // Added ID for DHT
    NewBlock(Block),
    NewTransaction(Transaction),
    // DHT
    FindNode(NodeId),
    Neighbors(Vec<Peer>),
}

pub struct P2P {
    tx_channel: broadcast::Sender<Message>,
    routing_table: Arc<Mutex<RoutingTable>>,
}

impl P2P {
    pub fn new(tx_channel: broadcast::Sender<Message>, local_id: NodeId) -> Self {
        P2P {
            tx_channel,
            routing_table: Arc::new(Mutex::new(RoutingTable::new(local_id))),
        }
    }

    pub async fn start_server(port: u16, tx: broadcast::Sender<Message>, routing_table: Arc<Mutex<RoutingTable>>) {
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr).await.expect("Failed to bind port");
        println!("[P2P] Server listening on {} (DHT Enabled)", addr);

        loop {
            let (mut socket, addr) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                     println!("Connection failed: {}", e);
                     continue;
                }
            };
            
            println!("[P2P] New connection from: {}", addr);
            let tx = tx.clone();
            let mut rx = tx.subscribe();
            let rt = routing_table.clone();

            tokio::spawn(async move {
                let (reader, mut writer) = socket.split();
                let mut reader = BufReader::new(reader);
                let mut line = String::new();

                loop {
                    // DoS Protection: Limit line length to 10KB
                    let mut limited_reader = reader.take(10_240); 
                    tokio::select! {
                        result = tokio::time::timeout(std::time::Duration::from_secs(30), limited_reader.read_line(&mut line)) => {
                            let result = match result {
                                Ok(res) => res,
                                Err(_) => {
                                    println!("[P2P] Connection timed out (Slowloris protection).");
                                    break;
                                }
                            };
                            match result {
                                Ok(0) => break, // EOF
                                Ok(_) => {
                                    if let Ok(msg) = serde_json::from_str::<Message>(&line) {
                                        // Process Message
                                        match msg.clone() {
                                            Message::Handshake { port, id } => {
                                                if let Some(node_id) = id {
                                                    let mut peer_addr = addr.ip().to_string();
                                                    if peer_addr == "127.0.0.1" { peer_addr = addr.ip().to_string(); } // Keep local if local
                                                    let full_addr = format!("{}:{}", peer_addr, port);
                                                    
                                                    let peer = Peer {
                                                        id: node_id,
                                                        address: full_addr, 
                                                        last_seen: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
                                                    };
                                                    
                                                    rt.lock().unwrap().add_peer(peer);
                                                    // println!("[DHT] Added peer: {:?}", node_id);
                                                }
                                            },
                                            Message::FindNode(target_id) => {
                                                // Return K-Closest
                                                let neighbors = rt.lock().unwrap().find_closest(&target_id, 8);
                                                let response = Message::Neighbors(neighbors);
                                                if let Ok(json) = serde_json::to_string(&response) {
                                                    let _ = writer.write_all((json + "\n").as_str().as_bytes()).await;
                                                }
                                            },
                                            Message::Neighbors(peers) => {
                                                 // Add discovered peers to our table
                                                 let mut table = rt.lock().unwrap();
                                                 for p in peers {
                                                     table.add_peer(p);
                                                 }
                                                 println!("[DHT] Sync: {} peers known", table.buckets.iter().map(|b| b.peers.len()).sum::<usize>());
                                            },
                                            _ => {
                                                // Forward Block/Tx to Internal Channel (Main Node logic handles logic)
                                                // println!("[P2P] Forwarding Inbound Message to System...");
                                                let _ = tx.send(msg);
                                            }
                                        }
                                    }
                                    line.clear();
                                }
                                Err(_) => break,
                            }
                        }
                        }
                        recv_res = rx.recv() => {
                             if let Ok(msg) = recv_res {
                                 // Don't echo back exact messages if possible, but for broadcast it's fine
                                 if let Ok(json) = serde_json::to_string(&msg) {
                                     if writer.write_all(json.as_bytes()).await.is_err() || 
                                        writer.write_all(b"\n").await.is_err() {
                                         break; 
                                     }
                                 }
                             }
                        }
                    }
                }
            });
        }
    }
}
