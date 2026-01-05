use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use crate::block::Block;
use crate::transaction::Transaction;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Handshake { port: u16 },
    NewBlock(Block),
    NewTransaction(Transaction),
}

pub struct P2P {
    peers: Vec<SocketAddr>,
    tx_channel: broadcast::Sender<Message>,
}

impl P2P {
    pub fn new(tx_channel: broadcast::Sender<Message>) -> Self {
        P2P {
            peers: Vec::new(),
            tx_channel,
        }
    }

    pub async fn start_server(port: u16, tx: broadcast::Sender<Message>) {
        let addr = format!("0.0.0.0:{}", port);
        // Bind failure is fatal at startup, expect is fine here.
        let listener = TcpListener::bind(&addr).await.expect("Failed to bind port");
        println!("P2P Server listening on {}", addr);

        loop {
            // FIX: Handle accept error without crashing
            let (mut socket, addr) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                     println!("Connection failed: {}", e);
                     continue;
                }
            };
            
            println!("New connection from: {}", addr);
            let tx = tx.clone();
            let mut rx = tx.subscribe();

            tokio::spawn(async move {
                let (reader, mut writer) = socket.split();
                let mut reader = BufReader::new(reader);
                let mut line = String::new();

                loop {
                    tokio::select! {
                        result = reader.read_line(&mut line) => {
                            // FIX: Handle read error gracefully
                            match result {
                                Ok(0) => break, // EOF
                                Ok(_) => {
                                    // println!("Received from {}: {}", addr, line.trim());
                                    // Parse Message (Optional phase)
                                    if let Ok(msg) = serde_json::from_str::<Message>(&line) {
                                        println!("Valid P2P Message from {}: {:?}", addr, msg);
                                    } else {
                                        println!("Raw/Invalid from {}: {}", addr, line.trim());
                                    }
                                    line.clear();
                                }
                                Err(e) => {
                                    println!("Read error from {}: {}", addr, e);
                                    break;
                                }
                            }
                        }
                        recv_res = rx.recv() => {
                             match recv_res {
                                Ok(msg) => {
                                    // Broadcast message to this client
                                    // FIX: Handle serialization/write error
                                    if let Ok(json) = serde_json::to_string(&msg) {
                                        if writer.write_all(json.as_bytes()).await.is_err() || 
                                           writer.write_all(b"\n").await.is_err() {
                                            break; // Client disconnected
                                        }
                                    }
                                }
                                Err(_) => {
                                    // Lagged or Closed, ignore for now to keep connection alive if possible
                                }
                             }
                        }
                    }
                }
                println!("Connection closed: {}", addr);
            });
        }
    }
}
