use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use k256::ecdsa::{SigningKey, VerifyingKey, Signature, signature::Signer};
use k256::ecdsa::signature::Verifier;
use k256::elliptic_curve::scalar::IsHigh; // Trait required for is_high() check
use crate::script::Script;
use hex;

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
    BurnNFT,
    DeployContract,
    CallContract
}

impl std::fmt::Display for TxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn default_tx_type() -> TxType { TxType::Transfer }
fn default_token() -> String { "VLT".to_string() }
fn default_script() -> Script { Script::new() }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub version: u32, // Added for Longevity
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

    // Phase 6: Smart Contracts
    #[serde(default)]
    pub data: Vec<u8>, // Bytecode (Deploy) or Args (Call)
}

impl Transaction {
    pub fn new(sender: String, receiver: String, amount: u64, token: String, nonce: u64, fee: u64) -> Self {
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
            version: 1,
            sender,
            receiver,
            amount,
            signature: String::new(),
            timestamp,
            token,
            tx_type: TxType::Transfer,
            nonce,
            fee: if fee == 0 { 100_000 } else { fee }, // Use provided fee or default
            script_pub_key,
            script_sig: Script::new(),
            price: 0,
            data: vec![],
        }
    }

    pub fn new_token_issue(sender: String, symbol: String, supply: u64, nonce: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Transaction {
            version: 1,
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
            data: vec![],
        }
    }

    pub fn calculate_hash(&self) -> String {
        // MATCHING LOGIC: Use get_hash() (Binary Serialization) as the single source of truth.
        // This ensures the TxID matches what is signed/verified.
        let bytes = self.get_hash();
        hex::encode(bytes)
    }

    pub fn new_burn(sender: String, token: String, amount: u64, nonce: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Transaction {
            version: 1,
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
            data: vec![],
        }
    }

    pub fn new_order(sender: String, token: String, side: &str, amount: u64, _price: u64, nonce: u64) -> Self {
         let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Receiver functions as "Side" indicator for simple storage
        let receiver = if side == "BUY" { "DEX_BUY".to_string() } else { "DEX_SELL".to_string() };

        Transaction {
            version: 1,
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
            price: 0,
            data: vec![],
        }
    }

    pub fn new_cancel(sender: String, order_id: String, nonce: u64) -> Self {
         let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // We use 'token' field to store the Order ID (string format) for cancellation
        Transaction {
            version: 1,
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
            data: vec![],
        }
    }

    pub fn new_stake(sender: String, amount: u64, nonce: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Transaction {
            version: 1,
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
            data: vec![],
        }
    }

    pub fn new_deploy_contract(sender: String, bytecode: Vec<u8>, nonce: u64) -> Self {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        Transaction {
            version: 1,
            sender: sender.clone(),
            receiver: "CONTRACT_DEPLOY".to_string(),
            amount: 0,
            signature: String::new(),
            timestamp,
            token: "VLT".to_string(),
            tx_type: TxType::DeployContract,
            nonce,
            fee: 1_000_000,
            script_pub_key: Script::new(),
            script_sig: Script::new(),
            price: 0,
            data: bytecode,
        }
    }

    pub fn new_call_contract(sender: String, contract: String, data: Vec<u8>, nonce: u64) -> Self {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        Transaction {
            version: 1,
            sender,
            receiver: contract,
            amount: 0, 
            signature: String::new(),
            timestamp,
            token: "VLT".to_string(),
            tx_type: TxType::CallContract,
            nonce,
            fee: 500_000,
            script_pub_key: Script::new(),
            script_sig: Script::new(),
            price: 0,
            data,
        }
    }

    pub fn new_unstake(sender: String, amount: u64, nonce: u64) -> Self {
         let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Transaction {
            version: 1,
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
            data: vec![],
        }
    }

    pub fn new_deploy(sender: String, bytecode: Vec<u8>, nonce: u64) -> Self {
         let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
         Transaction {
            version: 1,
            sender: sender.clone(),
            receiver: String::new(), // No receiver for deploy
            amount: 0,
            signature: String::new(),
            timestamp,
            token: "VLT".to_string(),
            tx_type: TxType::DeployContract,
            nonce,
            fee: 200_000, 
            script_pub_key: Script::new(),
            script_sig: Script::new(),
            price: 0,
            data: bytecode,
        }
    }

    pub fn new_call(sender: String, contract: String, _method: &str, args: Vec<u8>, nonce: u64) -> Self {
         let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
         // define arg payload? For now just raw bytes
         Transaction {
            version: 1,
            sender,
            receiver: contract,
            amount: 0,
            signature: String::new(),
            timestamp,
            token: "VLT".to_string(),
            tx_type: TxType::CallContract,
            nonce,
            fee: 50_000, 
            script_pub_key: Script::new(),
            script_sig: Script::new(),
            price: 0,
            data: args, // We might need to encode method name here too? Or use formatted string "method|args"
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

             // Fallback for Local Generation: RECONSTRUCT BITCOIN-STYLE COINBASE
             // When Stratum reconstructs the block, it sets script_sig with [Height, Nonce].
             // We must serialize this exactly as the Miner did (Version 1, Inputs, Outputs, Locktime).
             
             // Check if we have the standard Coinbase pattern: [Push(Height), Push(Nonce)]
             if self.script_sig.ops.len() == 2 {
                 if let (Some(crate::script::OpCode::OpPush(height_bytes)), Some(crate::script::OpCode::OpPush(nonce_bytes))) = 
                    (self.script_sig.ops.get(0), self.script_sig.ops.get(1)) 
                 {
                     // Found it! Build Bitcoin Serialization.
                     let mut btc_tx = Vec::new();
                     println!("[Tx Debug] Native Coinbase Reconstruction Triggered!");
                     
                     // 1. Version (4 Bytes LE) - Fixed to 1 in Stratum
                     btc_tx.extend(&1u32.to_le_bytes());
                     
                     // 2. Input Count (VarInt 1)
                     btc_tx.push(0x01);
                     
                     // 3. Input 0
                     // PrevHash (32 bytes 0)
                     btc_tx.extend(&[0u8; 32]);
                     // Index (4 bytes 0xFFFFFFFF)
                     btc_tx.extend(&[0xff, 0xff, 0xff, 0xff]);
                     
                     // ScriptSig
                     // Format: [PushOp(Height) + PushOp(Nonce)]
                     // Note: OpPush serialization usually includes length prefix.
                     // But Stratum logic was: "04" + Height + "08" + Nonce.
                     // We manually construct it to match Stratum EXACTLY.
                     let mut script_bytes = Vec::new();
                     // Height Push (0x04 + 4 bytes)
                     script_bytes.push(0x04); 
                     script_bytes.extend(height_bytes);
                     // Nonce Push (0x08 + 8 bytes)
                     script_bytes.push(0x08);
                     script_bytes.extend(nonce_bytes);
                     
                     // Script Length (VarInt)
                     btc_tx.push(script_bytes.len() as u8); // Assuming < 253
                     btc_tx.extend(script_bytes);
                     
                     // Sequence (4 bytes 0xFFFFFFFF)
                     btc_tx.extend(&[0xff, 0xff, 0xff, 0xff]);
                     
                     // 4. Output Count (VarInt 1)
                     btc_tx.push(0x01);
                     
                     // 5. Output 0
                     // Amount (8 bytes LE)
                     btc_tx.extend(&self.amount.to_le_bytes());
                     
                     // ScriptPubKey (Standard P2PKH)
                     // Stratum uses: 1976a914{PUBKEYHASH}88ac
                     // 0x19 (25 bytes) + 76 (OP_DUP) + a9 (OP_HASH160) + 14 (20 bytes) + Hash + 88 (OP_EQUALVERIFY) + ac (OP_CHECKSIG)
                     // We need to extract the PubKeyHash from our script_pub_key if possible, or reconstruct it from receiver?
                     // self.script_pub_key contains Ops.
                     // Standard P2PKH Ops: Dup, Hash256(Not 160!), Push(Hash), EqualVerify, CheckSig.
                     // WAIT. Transaction::new() uses OP_HASH256 (SHA256).
                     // But Stratum uses `1976a914...` which is STANDARD BITCOIN P2PKH (OP_HASH160 = SHA256 + RIPEMD160).
                     // Stratum code:
                     // let mut rip = Ripemd160::new(); rip.update(&sha_hash); ...
                     // So Stratum generates HASH160.
                     // Does our Transaction::new() generate HASH160?
                     // In Transaction::new(): `let hash = Sha256::digest(&pub_key_bytes).to_vec();` -> ONLY SHA256!
                     // THIS IS A MISMATCH. Stratum uses Ripemd160, Chain uses Sha256.
                     
                     // FIX: For this specific block reconstruction, we must trust the `script_pub_key` stored in the struct
                     // if it was correctly deserialized from Stratum's data?
                     // No, `process_rpc_request` clones `block.transactions[0]`.
                     // The block template was created by `create_block_template` (Chain).
                     // `chain.rs` uses `Transaction::new` -> Uses SHA256.
                     // `stratum.rs` IGNORES `block.transactions[0].script_pub_key` and constructs its own P2PKH using Ripemd160!
                     // "let cb2 = format!(... pub_key_hash_hex ...)"
                     
                     // So the internal `Transaction` object has a SHA256 logic, but the Miner receives a Ripemd160 Payout.
                     // The Miner calculates the hash based on Ripemd160.
                     // The Node calculates hash based on `Transaction` struct (SHA256).
                     // They differ.
                     
                     // We must serialize using the ACTUAL Payout Script used by Stratum.
                     // But we don't have the Ripemd160 hash here easily unless `script_pub_key` matches.
                     // BUT, `process_rpc_request` modifies `block.transactions[0]` but DOES NOT update `script_pub_key` to match the Ripemd one.
                     // It only updates `script_sig`.
                     
                     // CRITICAL FIX: We need to reconstruct the Payout Script using the method Stratum uses.
                     // Stratum derives it from `server_wallet.get_address()`.
                     // The `Transaction` object in the block has `receiver` = pool address.
                     // We can try to re-derive the Ripemd160 hash from `receiver` (if it's public key hex).
                     // `Transaction::new` stores `receiver` as string.
                     
                     // Reconstruct Payout:
                     // 1. Decode Receiver (Pub Key Hex)
                     // 2. Hash160 (Sha256 + Ripemd160)
                     // 3. Build Standard P2PKH Script
                     
                     // 1. Decode Receiver (Pub Key Hex)
                     // MATCH STRATUM: If decode fails, use 33 bytes of zeros (Comp/Uncomp key length placeholder).
                     // Stratum uses: .unwrap_or(vec![0;33])
                     let pub_key_bytes = hex::decode(&self.receiver).unwrap_or(vec![0;33]);
                     use sha2::{Sha256, Digest};
                     let sha_h = Sha256::digest(&pub_key_bytes);
                     use ripemd::Ripemd160;
                     let mut rip = Ripemd160::new();
                     rip.update(&sha_h);
                     let pub_key_hash = rip.finalize();
                     
                     let mut pk_script = Vec::new();
                     pk_script.push(0x76); // OP_DUP
                     pk_script.push(0xa9); // OP_HASH160
                     pk_script.push(0x14); // Push 20 bytes
                     pk_script.extend(pub_key_hash);
                     pk_script.push(0x88); // OP_EQUALVERIFY
                     pk_script.push(0xac); // OP_CHECKSIG
                     
                     btc_tx.push(pk_script.len() as u8);
                     btc_tx.extend(pk_script);
                     
                     // 6. Locktime (4 bytes 0)
                     btc_tx.extend(&[0u8, 0, 0, 0]);
                     
                      // Hash it
                      println!("[Tx Debug] Reconstructed Hex: {}", hex::encode(&btc_tx));
                      let mut hasher = Sha256::new();
                     hasher.update(&btc_tx);
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
                 TxType::DeployContract => 13,
                 TxType::CallContract => 14,
             };
             bytes.push(type_byte);
             bytes.extend(&self.nonce.to_le_bytes());
             bytes.extend(&self.fee.to_le_bytes());
             bytes.extend(&self.price.to_le_bytes()); // CRITICAL FIX: Include Price in Hash (DEX Security)
             bytes.extend(&self.data); // Include data in hash
             
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

        let public_key_bytes = match hex::decode(&self.sender) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        let public_key = match VerifyingKey::from_sec1_bytes(&public_key_bytes) {
            Ok(key) => key,
            Err(_) => return false,
        };
        
        let signature_bytes = match hex::decode(&self.signature) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        let signature = match Signature::from_der(&signature_bytes) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        // Advanced Security: Enforce Low-S (BIP-62)
        // If the S-value is "High" (greater than N/2), it is malleable.
        // We reject High-S signatures to ensure TxID immutability.
        if signature.s().is_high().into() {
             return false;
        }

        let hash = self.get_hash();
        public_key.verify(&hash, &signature).is_ok()
    }
}
