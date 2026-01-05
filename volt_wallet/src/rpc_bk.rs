use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcResponse {
    pub status: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

pub struct RpcClient {
    address: String,
}

impl RpcClient {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
        }
    }

    pub fn call(&self, command: &str, params: Option<serde_json::Value>) -> Result<RpcResponse, String> {
        // 1. Construct Request JSON
        let mut map = serde_json::Map::new();
        map.insert("command".to_string(), serde_json::Value::String(command.to_string()));
        
        if let Some(p) = params {
            if let serde_json::Value::Object(obj) = p {
                for (k, v) in obj {
                    map.insert(k, v);
                }
            }
        }
        let request_val = serde_json::Value::Object(map);
        let request_str = serde_json::to_string(&request_val)
            .map_err(|e| e.to_string())?;

        // 2. Connect via TCP with Timeout
        // Use timeout to fail fast if node is down/busy
        use std::net::ToSocketAddrs;
        let socket_addr = self.address.to_socket_addrs()
            .map_err(|e| format!("DNS Resolution Failed: {}", e))?
            .next()
            .ok_or_else(|| "Could not resolve hostname".to_string())?;

        let mut stream = std::net::TcpStream::connect_timeout(&socket_addr, Duration::from_secs(3))
            .map_err(|e| format!("Connection Failed: {}", e))?;
        
        stream.set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;

        // 3. Send Request
        stream.write_all(request_str.as_bytes())
            .map_err(|e| format!("Write Failed: {}", e))?;
            
        // 4. Read Response
        // Since my node might or might not send newline, and I'm using `read_to_string` or a buffer?
        // `api.rs` spawns a thread and writes output then drops stream? 
        // "let _ = stream.write_all(response_json.as_bytes());" -> Then the thread ends.
        // So reading to end of stream is fine.
        
        let mut response_bytes = Vec::new();
        stream.read_to_end(&mut response_bytes)
            .map_err(|e| format!("Read Failed: {}", e))?;
            
        let response_str = String::from_utf8(response_bytes)
            .map_err(|e| format!("Invalid UTF8: {}", e))?;

        // 5. Parse JSON
        let response: RpcResponse = serde_json::from_str(&response_str)
            .map_err(|e| format!("Parse Failed: {} | Raw: {}", e, response_str))?;

        Ok(response)
    }
}
