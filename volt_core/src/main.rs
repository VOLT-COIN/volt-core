mod block;
mod chain;
mod transaction;
mod wallet;
mod node;
mod api;
mod stratum;
mod db;
mod script;
// mod gui; // Phase 37: GUI Module (Disabled)

use chain::Blockchain;
use wallet::Wallet;
use node::Node;
use api::ApiServer;
use stratum::StratumServer;
// use gui::VoltNodeApp; // Import GUI (Disabled)
use std::sync::{Arc, Mutex};
use std::env;
use std::thread;
use std::time::Duration;

// Custom Logger function (pushes to GUI logs and stdout)
fn log(msg: &str, logs: &Arc<Mutex<Vec<String>>>) {
    println!("{}", msg);
    let mut l = logs.lock().unwrap();
    l.push(format!("[{}] {}", chrono::Local::now().format("%H:%M:%S"), msg));
    if l.len() > 1000 { l.remove(0); } // Keep buffer small
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Config: Port
    let port = args.iter()
        .skip(1)
        .find(|a| !a.starts_with("--") && a.parse::<u16>().is_ok())
        .and_then(|a| a.parse::<u16>().ok())
        .unwrap_or(6000);

    // Config: Headless Mode?
    let _headless = args.iter().any(|a| a == "--no-gui" || a == "--headless");

    // Shared State for GUI
    let logs = Arc::new(Mutex::new(Vec::new()));
    let peers_count = Arc::new(Mutex::new(0));

    log(&format!("--- Volt (VLT) Node Starting [Port: {}] ---", port), &logs);

    // 1. Blockchain
    let blockchain = Arc::new(Mutex::new(Blockchain::load()));
    
    // 2. Node
    let node = Arc::new(Node::new(blockchain.clone(), port));
    node.start_server();
    node.start_discovery(); 

    // 3. Connect to Peer
    let peer_addr = args.iter()
        .skip(2)
        .find(|&a| !a.starts_with("--") && a.parse::<u16>().is_err() && a != "--no-gui");

    if let Some(peer) = peer_addr {
        log(&format!("Connecting to peer: {}", peer), &logs);
        node.connect_to_peer(peer.to_string());
    }

    // 4. Wallet & Mining
    let miner_wallet = Arc::new(Mutex::new(Wallet::new()));
    let mut addr_miner = miner_wallet.lock().unwrap().get_address();
    let external_addr_flag = args.iter().find(|a| a.starts_with("VLT") && a.len() > 20); // Basic check
    let _use_external = external_addr_flag.is_some();

    if let Some(ext) = external_addr_flag {
         addr_miner = ext.clone();
         log(&format!("Mining Mode: EXTERNAL -> {}", addr_miner), &logs);
    } else {
         log(&format!("Mining Mode: INTERNAL -> {}", addr_miner), &logs);
    }

    let auto_mine = args.iter().any(|a| a == "--mine");
    let is_mining = Arc::new(Mutex::new(auto_mine));
    if auto_mine { log("Mining Mode: ENABLED (Auto-Start)", &logs); }

    // Dynamic Port Allocation
    let api_port = if port == 6000 { 6001 } else { port + 1 };
    let stratum_base = if port == 6000 { 3333 } else { port + 2000 };

    // 5. API Server
    let api_server = ApiServer::new(
        blockchain.clone(),
        is_mining.clone(),
        miner_wallet.clone(),
        api_port,
        node.clone()
    );
    api_server.start();

    // ...

    // 7. Stratum Servers
    log(&format!("[Stratum] Starting Multi-Port Mining Servers (Base: {})...", stratum_base), &logs);
    let s1 = StratumServer::new(blockchain.clone(), stratum_base, stratum::PoolMode::PPLNS); s1.start();
    let s2 = StratumServer::new(blockchain.clone(), stratum_base + 1, stratum::PoolMode::PPS); s2.start();
    let s3 = StratumServer::new(blockchain.clone(), stratum_base + 2, stratum::PoolMode::SOLO); s3.start();
    let s4 = StratumServer::new(blockchain.clone(), stratum_base + 3, stratum::PoolMode::FPPS); s4.start();

    // 8. Peer Count Updater Thread
    let node_gui = node.clone();
    let peers_count_gui = peers_count.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));
            let count = node_gui.peers.lock().unwrap().len();
            *peers_count_gui.lock().unwrap() = count;
        }
    });

    // 9. Launch Interface
    // 9. Launch Interface (Console Only for now due to linker issues)
    log("Running in CONSOLE mode. Type commands (START_MINING, STOP_MINING, SEND...)", &logs);
    
    let stdin = std::io::stdin();
    let mut buffer = String::new();
    loop {
        buffer.clear();
        if stdin.read_line(&mut buffer).is_ok() {
            let input = buffer.trim();
            // Simple command handler here (subset of previous)
            if input == "START_MINING" { *is_mining.lock().unwrap() = true; log("Mining STARTED", &logs); }
            else if input == "STOP_MINING" { *is_mining.lock().unwrap() = false; log("Mining STOPPED", &logs); }
            else if input == "EXIT" { break; }
        }
    }
    
    /* GUI DISABLED - LINKER ISSUES
    if headless {
       // ... existing headless code ...
    } else {
       // ... existing gui code ...
    }
    */
}
