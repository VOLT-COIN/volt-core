use serde::{Serialize, Deserialize};
use crate::transaction::Transaction;
use sha2::{Sha256, Digest};
use k256::ecdsa::{Signature, VerifyingKey};
use k256::ecdsa::signature::Verifier;


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum OpCode {
    /// Push data onto the stack
    OpPush(Vec<u8>),
    /// Duplicate the top item on the stack
    OpDup,
    /// Hash the top item (SHA256)
    OpHash256,
    /// Verify ECDSA signature
    OpCheckSig,
    /// Verify 2 items are equal
    OpEqualVerify,
    /// Lock until timestamp (CLTV)
    OpCheckLockTimeVerify,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Script {
    pub ops: Vec<OpCode>,
}

impl Script {
    pub fn new() -> Self {
        Script { ops: Vec::new() }
    }

    pub fn push(mut self, op: OpCode) -> Self {
        self.ops.push(op);
        self
    }
}

pub struct VirtualMachine {
    stack: Vec<Vec<u8>>,
}

impl VirtualMachine {
    pub fn new() -> Self {
        VirtualMachine { stack: Vec::new() }
    }

    pub fn execute(&mut self, script: &Script, context: &Transaction) -> bool {
        for op in &script.ops {
            match op {
                OpCode::OpPush(data) => {
                    self.stack.push(data.clone());
                },
                OpCode::OpDup => {
                    if let Some(top) = self.stack.last() {
                        self.stack.push(top.clone());
                    } else { return false; }
                },
                OpCode::OpHash256 => {
                    if let Some(item) = self.stack.pop() {
                        let mut hasher = Sha256::new();
                        hasher.update(&item);
                        let result1 = hasher.finalize();
                        
                        let mut hasher2 = Sha256::new();
                        hasher2.update(result1);
                        let result2 = hasher2.finalize();
                        
                        self.stack.push(result2.to_vec());
                    } else { return false; }
                },
                OpCode::OpEqualVerify => {
                    if self.stack.len() < 2 { return false; }
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if a != b { return false; }
                },
                OpCode::OpCheckSig => {
                    if self.stack.len() < 2 { return false; }
                    let pub_key_bytes = self.stack.pop().unwrap();
                    let sig_bytes = self.stack.pop().unwrap();
                    
                    // Verify Signature
                    // 1. Decode PubKey
                    // 2. Decode Signature
                    // 3. Verify
                    
                    let valid = if let Ok(pk) = VerifyingKey::from_sec1_bytes(&pub_key_bytes) {
                        if let Ok(sig) = Signature::from_der(&sig_bytes) {
                            // Verify against Transaction Context!
                            // We hash the Transaction (excluding signature) usually.
                            // Here we use `get_hash()` which includes EVERYTHING.
                            // This is a circular dependency if the signature is INSIDE the hash.
                            // But `get_hash()` includes `script_sig`.
                            // So we CANNOT use `get_hash()` unless we zero out script_sig first.
                            // For Phase 28, we will sign a "Pre-Image" which is (Sender + Receiver + Amount + Timestamp + Nonce).
                            // Simplification: Verify signature against `sender` field (which is the PubKey).
                            
                            // Let's rely on the Signature being valid for the "Message" which we can define as the `get_hash()` logic excluding scripts.
                            // But `Transaction::verify` uses `get_hash`.
                            // So we should verify against `context.get_data_for_signing()`.
                            // But we don't have that method.
                            // Valid Signature Verification
                            // We verify against context.get_hash() which now correctly excludes the signature itself (via Script::new() placeholder).
                            pk.verify(&context.get_hash(), &sig).is_ok()
                        } else { false }
                    } else { false };
                    
                    self.stack.push(vec![if valid { 1 } else { 0 }]);
                },
                OpCode::OpCheckLockTimeVerify => {
                    if let Some(item) = self.stack.last() {
                         // Parse item as u64 timestamp
                         let lock_time = u64::from_be_bytes(item[0..8].try_into().unwrap_or([0;8]));
                         if context.timestamp < lock_time { return false; } // Lock is still active
                    } else { return false; }
                }
            }
        }
        
        // Validation successful if stack top is TRUE (1)
        if let Some(top) = self.stack.last() {
             return top == &vec![1];
        }
        false
    }
}
