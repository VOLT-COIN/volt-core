use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use k256::ecdsa::{SigningKey, VerifyingKey, Signature, signature::Signer};
use k256::ecdsa::signature::Verifier;
use crate::script::Script;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TxType {
    Transfer,
    IssueToken,
    Stake,
    Unstake,
    Burn,
    PlaceOrder,
    CancelOrder,
    AddLiquidity,
    RemoveLiquidity,
    Swap,
    IssueNFT,
    TransferNFT,
    BurnNFT
}

fn default_tx_type() -> TxType { TxType::Transfer }
fn default_token() -> String { "VLT".to_string() }
fn default_script() -> Script { Script::new() }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub sender: String,
    pub receiver: String,
    pub amount: u64,
    pub signature: String,
    pub timestamp: u64,
    
    // Token Extension
    #[serde(default = "default_token")]
    pub token: String, 
    
    // Protocol V2
    #[serde(default = "default_tx_type")]
    pub tx_type: TxType, 
    
    // Security: Replay Protection
    #[serde(default)]
    pub nonce: u64,
    
    // Phase 12: Fee Model
    #[serde(default)]
    pub fee: u64,

    // Phase 28: Smart Scripting
    #[serde(default = "default_script")]
    pub script_pub_key: Script, // Locking Script (Receiver)
    #[serde(default = "default_script")]
    pub script_sig: Script,     // Unlocking Script (Sender)

    // Phase 34: DEX
    #[serde(default)]
    pub price: u64, // For Limit Orders (VLT per Token Unit)
}

impl Transaction {
    pub fn new(sender: String, receiver: String, amount: u64, token: String, nonce: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // P2PKH Logic
        let pub_key_bytes = hex::decode(&receiver).unwrap_or(vec![]);
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(&pub_key_bytes).to_vec();
        
        let script_pub_key = Script::new()
            .push(crate::script::OpCode::OpDup)
            .push(crate::script::OpCode::OpHash256)
            .push(crate::script::OpCode::OpPush(hash))
            .push(crate::script::OpCode::OpEqualVerify)
            .push(crate::script::OpCode::OpCheckSig);

        Transaction {
            sender,
            receiver,
            amount,
            signature: String::new(),
            timestamp,
            token,
            tx_type: TxType::Transfer,
            nonce,
            fee: 100_000,
            script_pub_key,
            script_sig: Script::new(),
            price: 0,
        }
    }

    pub fn new_token_issue(sender: String, symbol: String, supply: u64, nonce: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Transaction {
            sender: sender.clone(),
            receiver: sender, // Issue to self
            amount: supply,
            signature: String::new(),
            timestamp,
            token: symbol.clone(),
            tx_type: TxType::IssueToken,
            nonce,
            fee: 500_000, // Higher fee for issuance
            script_pub_key: Script::new(),
            script_sig: Script::new(),
            price: 0,
        }
    }

    pub fn calculate_hash(&self) -> String {
        // Deterministic Hashing for Cross-Platform Signing (JS <-> Rust)
        // Including price and tx_type to secure DEX orders
        let payload = format!(
            "{}:{}:{}:{}:{}:{}:{}:{:?}",
            self.sender,
            self.receiver,
            self.amount,
            self.nonce,
            self.token,
            self.timestamp,
            self.price,
            self.tx_type
        );
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let result = hasher.finalize();
        hex::encode(result)
    }

    pub fn new_burn(sender: String, token: String, amount: u64, nonce: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Transaction {
            sender: sender.clone(),
            receiver: "BURN".to_string(), // Sentinel value, unused by validation
            amount,
            signature: String::new(),
            timestamp,
            token,
            tx_type: TxType::Burn,
            nonce,
            fee: 100_000,
            script_pub_key: Script::new(),
            script_sig: Script::new(),
            price: 0,
        }
    }

    pub fn new_order(sender: String, token: String, side: &str, amount: u64, price: u64, nonce: u64) -> Self {
         let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Receiver functions as "Side" indicator for simple storage
        let receiver = if side == "BUY" { "DEX_BUY".to_string() } else { "DEX_SELL".to_string() };

        Transaction {
            sender,
            receiver,
            amount,
            signature: String::new(),
            timestamp,
            token,
            tx_type: TxType::PlaceOrder,
            nonce,
            fee: 100_000,
            script_pub_key: Script::new(),
            script_sig: Script::new(),
            price,
        }
    }

    pub fn new_cancel(sender: String, order_id: String, nonce: u64) -> Self {
         let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // We use 'token' field to store the Order ID (string format) for cancellation
        Transaction {
            sender,
            receiver: "DEX_CANCEL".to_string(),
            amount: 0,
            signature: String::new(),
            timestamp,
            token: order_id, 
            tx_type: TxType::CancelOrder,
            nonce,
            fee: 10_000, 
            script_pub_key: Script::new(),
            script_sig: Script::new(),
            price: 0,
        }
    }

    pub fn new_stake(sender: String, amount: u64, nonce: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Transaction {
            sender: sender.clone(),
            receiver: String::from("STAKE_SYSTEM"),
            amount,
            signature: String::new(),
            timestamp,
            token: "VLT".to_string(),
            tx_type: TxType::Stake,
            nonce,
            fee: 100_000,
            script_pub_key: Script::new(),
            script_sig: Script::new(),
            price: 0,
        }
    }

    pub fn new_unstake(sender: String, amount: u64, nonce: u64) -> Self {
         let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Transaction {
            sender: sender.clone(),
            receiver: sender, // Return to self
            amount,
            signature: String::new(),
            timestamp,
            token: "VLT".to_string(),
            tx_type: TxType::Unstake,
            nonce,
            fee: 100_000,
            script_pub_key: Script::new(),
            script_sig: Script::new(),
            price: 0,
        }
    }

    pub fn get_hash(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        if self.sender == "SYSTEM" {
             // STRATUM COMPATIBILITY MODE
             // If script_sig contains a single OpPush, treat it as the RAW COINBASE BLOB.
             // This allows Stratum to inject the byte-perfect Coinbase which matches the Miner's hash.
             if let Some(crate::script::OpCode::OpPush(blob)) = self.script_sig.ops.first() {
                 if self.script_sig.ops.len() == 1 {
                      // Hash the Blob directly (Double SHA256)
                      use sha2::{Sha256, Digest};
                      let mut hasher = Sha256::new();
                      hasher.update(blob);
                      let res1 = hasher.finalize();
                      let mut hasher2 = Sha256::new();
                      hasher2.update(res1);
                      return hasher2.finalize().to_vec();
                 }
             }

             // Fallback for Local Generation (Validation Mode)
             // We serialize "sender", "receiver", "amount", "timestamp".
             bytes.extend(self.sender.as_bytes());
             bytes.extend(self.receiver.as_bytes());
             bytes.extend(&self.amount.to_le_bytes());
             bytes.extend(&self.timestamp.to_le_bytes());
        } else {
             // Regular Transaction (Binary Packing)
             bytes.extend(self.sender.as_bytes());
             bytes.extend(self.receiver.as_bytes());
             bytes.extend(&self.amount.to_le_bytes());
             bytes.extend(&self.timestamp.to_le_bytes());
             bytes.extend(self.token.as_bytes());
             // TxType as u8
             let type_byte = match self.tx_type {
                 TxType::Transfer => 0,
                 TxType::IssueToken => 1,
                 TxType::Stake => 2,
                 TxType::Unstake => 3,
                 TxType::Burn => 4,
                 TxType::PlaceOrder => 5,
                 TxType::CancelOrder => 6,
                 TxType::AddLiquidity => 7,
                 TxType::RemoveLiquidity => 8,
                 TxType::Swap => 9,
                 TxType::IssueNFT => 10,
                 TxType::TransferNFT => 11,
                 TxType::BurnNFT => 12,
             };
             bytes.push(type_byte);
             bytes.extend(&self.nonce.to_le_bytes());
             bytes.extend(&self.fee.to_le_bytes());
             
             // Script Pub Key (Ops)
             for _op in &self.script_pub_key.ops {
                  // Serialize Op (Simplification: just stringify? No, binary)
                  // For MVP, skip script serialization in Hash for now (Signature covers body)
                  // Or assume default P2PKH logic relies on sender/receiver.
             }
        }

        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let res1 = hasher.finalize();
        
        let mut hasher2 = Sha256::new();
        hasher2.update(res1);
        hasher2.finalize().to_vec()
    }

    pub fn sign(&mut self, private_key: &SigningKey) {
        let hash = self.get_hash();
        let signature: Signature = private_key.sign(&hash);
        self.signature = hex::encode(signature.to_der());
        
        // Phase 28: Populate ScriptSig (Unlocking Script)
        // Push <Signature> <PubKey>
        let verifying_key = private_key.verifying_key();
        let pub_key_bytes = verifying_key.to_sec1_bytes().to_vec();
        let sig_bytes = signature.to_der();
        let sig_vec = sig_bytes.as_ref().to_vec();
        
        self.script_sig = Script::new()
            .push(crate::script::OpCode::OpPush(sig_vec))
            .push(crate::script::OpCode::OpPush(pub_key_bytes));
    }

    pub fn verify(&self) -> bool {
         if self.sender == "SYSTEM" {
             return true; // Mining rewards have no sender
         }

        let public_key_bytes = hex::decode(&self.sender).expect("Invalid sender hex");
        let public_key = VerifyingKey::from_sec1_bytes(&public_key_bytes).expect("Invalid public key");
        
        let signature_bytes = hex::decode(&self.signature).expect("Invalid signature hex");
        let signature = Signature::from_der(&signature_bytes).expect("Invalid signature");

        let hash = self.get_hash();
        public_key.verify(&hash, &signature).is_ok()
    }
}
